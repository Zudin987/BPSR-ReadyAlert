use crate::{
    chat::ChatRuntime,
    game_filter::{parse_tcp, Endpoint, GamePacketFilter},
    logging,
    model::{AlertEvent, AlertKind, AppEvent, PlayerIdentity},
    npcap::{CaptureHandle, NpcapDevice, PcapApi},
    proto,
    settings::AppSettings,
    telemetry::TelemetryRuntime,
};
use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
    sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

const MAX_GAME_FRAME: usize = 2 * 1024 * 1024;
const MAX_PENDING: usize = 2 * 1024 * 1024;
const MAX_FLOWS: usize = 2048;
const MAX_DEPTH: usize = 4;
const MAX_UNSYNC: usize = 512 * 1024;
const UNSYNC_TAIL: usize = 128 * 1024;

pub fn spawn(
    api: Arc<PcapApi>,
    settings: Arc<RwLock<AppSettings>>,
    identity: Arc<RwLock<Option<PlayerIdentity>>>,
    chat_runtime: ChatRuntime,
    tx: Sender<AppEvent>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new().name("readyalert-capture".into()).spawn(move || {
        let mut failures = 0u32;
        while !stop.load(Ordering::Relaxed) {
            let snapshot = settings.read().map(|x| x.clone()).unwrap_or_default();
            let device = match select_device(&api, &snapshot) {
                Ok(device) => device,
                Err(err) => {
                    failures += 1;
                    logging::write(format!("capture: adapter selection failed: {err}"));
                    let _ = tx.send(AppEvent::CaptureStatus(format!("Waiting for Npcap adapter: {err}")));
                    sleep_interruptible(&stop, retry_delay(failures));
                    continue;
                }
            };
            let mut handle = match CaptureHandle::open(api.clone(), &device.name) {
                Ok(handle) => handle,
                Err(err) => {
                    failures += 1;
                    logging::write(format!("capture: open {} failed: {err}", device.description));
                    let _ = tx.send(AppEvent::CaptureStatus(format!("Npcap recovery: {err}")));
                    sleep_interruptible(&stop, retry_delay(failures));
                    continue;
                }
            };
            failures = 0;
            let _ = tx.send(AppEvent::CaptureStatus(format!("Capturing {}", device.description)));
            logging::write(format!("capture: opened {} datalink={} source={}", device.description, handle.datalink, device.source));
            let mut filter = GamePacketFilter::new();
            let mut processor = CaptureProcessor::new(tx.clone(), identity.clone(), chat_runtime.clone());
            let opened = Instant::now();
            let mut last_packet: Option<Instant> = None;
            let mut last_watchdog = Instant::now();
            let mut reopen = false;

            while !stop.load(Ordering::Relaxed) {
                let datalink = handle.datalink;
                match handle.next_packet() {
                    Ok(Some(packet)) => {
                        if filter.is_game_server_packet(packet, datalink) {
                            last_packet = Some(Instant::now());
                            processor.process_packet(packet, datalink);
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        logging::write(format!("capture: read failed: {err}"));
                        reopen = true;
                        break;
                    }
                }
                if last_watchdog.elapsed() >= Duration::from_secs(1) {
                    last_watchdog = Instant::now();
                    let running = filter.game_running();
                    let anchor = last_packet.unwrap_or(opened);
                    if running && anchor.elapsed() >= Duration::from_secs(45) {
                        logging::write("capture-recovery: silent watchdog reopening Npcap");
                        reopen = true;
                        break;
                    }
                    if running && last_packet.map(|x| x.elapsed() <= Duration::from_secs(5)).unwrap_or(false)
                        && processor.last_valid_frame.map(|x| x.elapsed() >= Duration::from_secs(20)).unwrap_or(false) {
                        processor.reset_flows();
                        logging::write("capture-recovery: protocol frame stall; reset flows");
                    }
                    processor.cleanup_flows(false);
                }
            }
            drop(handle);
            if stop.load(Ordering::Relaxed) { break; }
            if reopen {
                failures += 1;
                sleep_interruptible(&stop, retry_delay(failures));
            }
        }
        logging::write("capture: stopped");
    }).expect("spawn capture thread")
}

#[derive(Clone, Debug)]
struct SelectedDevice { name:String, description:String, source:String }

fn select_device(api: &PcapApi, settings: &AppSettings) -> Result<SelectedDevice, String> {
    let devices = api.devices()?;
    if devices.is_empty() { return Err("Npcap found no adapters".into()); }
    logging::write(format!("npcap: {} devices, {}", devices.len(), api.version()));
    if !settings.npcap_device_name.trim().is_empty() {
        if let Some(d) = devices.iter().find(|d| d.name.eq_ignore_ascii_case(settings.npcap_device_name.trim())) {
            return Ok(selected(d, "User selected"));
        }
        return Err(format!("manual adapter unavailable: {}", settings.npcap_device_name));
    }
    let d = devices.iter().find(|d| !looks_loopback(d) && !looks_virtual(d))
        .or_else(|| devices.iter().find(|d| !looks_loopback(d))).unwrap_or(&devices[0]);
    Ok(selected(d, "Auto-selected"))
}

fn selected(d:&NpcapDevice, source:&str) -> SelectedDevice { SelectedDevice { name:d.name.clone(), description:d.description.clone(), source:source.into() } }
fn looks_loopback(d:&NpcapDevice) -> bool { format!("{} {}", d.name,d.description).to_ascii_lowercase().contains("loopback") }
fn looks_virtual(d:&NpcapDevice) -> bool {
    let text=format!("{} {}",d.name,d.description).to_ascii_lowercase();
    ["virtual","vmware","hyper-v","virtualbox","vpn","tap","tunnel","wsl","tailscale","zerotier","wireguard","bluetooth","wan miniport"].iter().any(|x| text.contains(x))
}
fn retry_delay(failures:u32)->Duration { Duration::from_millis(match failures {0|1=>1000,2=>2000,3=>5000,_=>10000}) }
fn sleep_interruptible(stop:&AtomicBool, duration:Duration) { let step=Duration::from_millis(100); let start=Instant::now(); while start.elapsed()<duration && !stop.load(Ordering::Relaxed) { thread::sleep(step); } }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct FlowKey { source:Endpoint, destination:Endpoint }

#[derive(Default)]
struct ByteStream { buf:Vec<u8>, start:usize, scan_cursor:usize }
impl ByteStream {
    fn len(&self)->usize { self.buf.len().saturating_sub(self.start) }
    fn slice(&self)->&[u8] { &self.buf[self.start..] }
    fn append(&mut self,data:&[u8]) { self.buf.extend_from_slice(data); }
    fn clear(&mut self){ self.buf.clear(); self.start=0; self.scan_cursor=0; }
    fn consume(&mut self,n:usize){ self.start=(self.start+n).min(self.buf.len()); self.scan_cursor=self.scan_cursor.saturating_sub(n); if self.start>64*1024 && self.start*2>self.buf.len(){ self.buf.drain(..self.start); self.start=0; } }
    fn trim_unsync(&mut self){ if self.len()>MAX_UNSYNC { let n=self.len()-UNSYNC_TAIL; self.consume(n); } }
}

struct FlowState {
    next_seq:Option<u32>, pending:BTreeMap<u32,Vec<u8>>, pending_bytes:usize,
    stream:ByteStream, synchronized:bool, gap_started:Option<Instant>, last_seen:Instant,
}
impl Default for FlowState { fn default()->Self { Self { next_seq:None,pending:BTreeMap::new(),pending_bytes:0,stream:ByteStream::default(),synchronized:false,gap_started=None,last_seen:Instant::now() } } }
impl FlowState { fn reset(&mut self,next:Option<u32>, synchronized:bool){ self.next_seq=next; self.pending.clear();self.pending_bytes=0;self.stream.clear();self.synchronized=synchronized;self.gap_started=None; } }

struct CaptureProcessor {
    tx:Sender<AppEvent>, identity:Arc<RwLock<Option<PlayerIdentity>>>, chat:ChatRuntime, telemetry:TelemetryRuntime,
    flows:HashMap<FlowKey,FlowState>, last_ready:Option<Instant>, last_match:Option<Instant>, last_vote:Option<Instant>,
    last_invite:(u64,Option<Instant>), last_request:(u64,Option<Instant>), sequence:u64,
    pub last_valid_frame:Option<Instant>, last_cleanup:Instant,
}

impl CaptureProcessor {
    fn new(tx:Sender<AppEvent>, identity:Arc<RwLock<Option<PlayerIdentity>>>, chat:ChatRuntime)->Self {
        let telemetry = TelemetryRuntime::new(tx.clone());
        Self { tx,identity,chat,telemetry,flows:HashMap::new(),last_ready:None,last_match:None,last_vote:None,last_invite:(0,None),last_request:(0,None),sequence:0,last_valid_frame:None,last_cleanup:Instant::now() }
    }
    fn reset_flows(&mut self){ self.flows.clear(); self.last_valid_frame=Some(Instant::now()); }
    fn cleanup_flows(&mut self, aggressive:bool){
        if !aggressive && self.last_cleanup.elapsed()<Duration::from_secs(30){return;} self.last_cleanup=Instant::now();
        let cutoff=if aggressive {Duration::from_secs(10)} else {Duration::from_secs(90)};
        self.flows.retain(|_,f| f.last_seen.elapsed()<cutoff);
        if aggressive && self.flows.len()>=MAX_FLOWS { if let Some(key)=self.flows.iter().max_by_key(|(_,f)|f.last_seen.elapsed()).map(|(k,_)|*k){self.flows.remove(&key);} }
    }

    fn process_packet(&mut self, packet:&[u8], datalink:i32){
        let Some(tcp)=parse_tcp(packet,datalink) else{return;};
        let key=FlowKey{source:tcp.source,destination:tcp.destination};
        if self.flows.len()>=MAX_FLOWS && !self.flows.contains_key(&key){self.cleanup_flows(true);}
        let mut flow=self.flows.remove(&key).unwrap_or_default(); flow.last_seen=Instant::now();
        let syn=tcp.flags&0x02!=0; if syn{flow.reset(Some(tcp.seq.wrapping_add(1)),true);} let payload_seq=if syn{tcp.seq.wrapping_add(1)}else{tcp.seq};
        if tcp.payload_len>0 && tcp.payload_offset+tcp.payload_len<=packet.len(){ self.insert_segment(&mut flow,payload_seq,&packet[tcp.payload_offset..tcp.payload_offset+tcp.payload_len]); }
        let closed=tcp.flags&0x05!=0; if !closed{self.flows.insert(key,flow);}
    }

    fn insert_segment(&mut self, flow:&mut FlowState, mut seq:u32, mut data:&[u8]){
        if flow.next_seq.is_none(){flow.next_seq=Some(seq);flow.synchronized=false;}
        let next=flow.next_seq.unwrap();
        if seq_before(seq,next){let overlap=next.wrapping_sub(seq) as usize;if overlap>=data.len(){return;}data=&data[overlap..];seq=next;}
        if seq==flow.next_seq.unwrap(){ self.append_stream(flow,data); flow.next_seq=Some(seq.wrapping_add(data.len() as u32)); self.drain_pending(flow); if flow.pending.is_empty(){flow.gap_started=None;} return; }
        if !flow.pending.contains_key(&seq){flow.pending_bytes+=data.len();flow.pending.insert(seq,data.to_vec());}
        if self.try_fast_recover_gap(flow){return;}
        flow.gap_started.get_or_insert_with(Instant::now);
        let recover=flow.pending_bytes>=256*1024 || flow.pending.len()>=64 || flow.gap_started.map(|x|x.elapsed()>=Duration::from_millis(1500)).unwrap_or(false);
        if recover{self.recover_gap(flow);return;} if flow.pending_bytes>MAX_PENDING{flow.reset(None,false);}
    }

    fn drain_pending(&mut self,flow:&mut FlowState){
        loop { let next=match flow.next_seq{Some(v)=>v,None=>return};
            if let Some(exact)=flow.pending.remove(&next){flow.pending_bytes=flow.pending_bytes.saturating_sub(exact.len());self.append_stream(flow,&exact);flow.next_seq=Some(next.wrapping_add(exact.len() as u32));continue;}
            let Some((&first_key,_))=flow.pending.iter().next() else{flow.gap_started=None;return;};
            if !seq_before(first_key,next){if self.try_fast_recover_gap(flow){return;}flow.gap_started.get_or_insert_with(Instant::now);return;}
            let data=flow.pending.remove(&first_key).unwrap();flow.pending_bytes=flow.pending_bytes.saturating_sub(data.len());let overlap=next.wrapping_sub(first_key) as usize;if overlap>=data.len(){continue;}let tail=&data[overlap..];self.append_stream(flow,tail);flow.next_seq=Some(next.wrapping_add(tail.len() as u32));
        }
    }

    fn try_fast_recover_gap(&mut self,flow:&mut FlowState)->bool{
        let Some((&first_seq,_))=flow.pending.iter().next() else{return false;}; let mut expected=first_seq;let mut combined=Vec::new();let mut keys=Vec::new();
        for (&seq,data) in &flow.pending {if seq!=expected||combined.len()+data.len()>MAX_UNSYNC{break;}combined.extend_from_slice(data);keys.push(seq);expected=expected.wrapping_add(data.len() as u32);}
        if combined.len()<6{return false;}let Some((offset,_,_))=find_strong_frame(&combined,0) else{return false;};
        for key in keys{if let Some(v)=flow.pending.remove(&key){flow.pending_bytes=flow.pending_bytes.saturating_sub(v.len());}}
        flow.stream.clear();flow.synchronized=true;flow.next_seq=Some(first_seq.wrapping_add(combined.len() as u32));flow.gap_started=None;self.append_stream(flow,&combined[offset..]);self.drain_pending(flow);true
    }
    fn recover_gap(&mut self,flow:&mut FlowState){let Some((&first,_))=flow.pending.iter().next() else{return;};flow.stream.clear();flow.synchronized=false;flow.next_seq=Some(first);flow.gap_started=None;self.drain_pending(flow);}

    fn append_stream(&mut self,flow:&mut FlowState,data:&[u8]){flow.stream.append(data);self.process_frames(flow);if flow.stream.len()>if flow.synchronized{MAX_GAME_FRAME*2}else{MAX_UNSYNC}{if flow.synchronized{flow.stream.clear();flow.synchronized=false;}else{flow.stream.trim_unsync();}}}

    fn process_frames(&mut self,flow:&mut FlowState){
        loop{if flow.stream.len()<6{return;}
            if !flow.synchronized{let start=flow.stream.scan_cursor.saturating_sub(40);let found=find_strong_frame(flow.stream.slice(),start);match found{Some((offset,_,_))=>{if offset>0{flow.stream.consume(offset);}flow.synchronized=true;flow.stream.scan_cursor=0;},None=>{flow.stream.scan_cursor=flow.stream.len().saturating_sub(40);flow.stream.trim_unsync();return;}}}
            let slice=flow.stream.slice();let Some((size,_))=plausible_header(slice,0) else{flow.synchronized=false;continue;};
            if slice.len()<size{if let Some((offset,_,_))=find_strong_frame(slice,1).filter(|x|x.0>0){flow.stream.consume(offset);flow.synchronized=true;continue;}return;}
            let frame=&slice[..size];self.last_valid_frame=Some(Instant::now());self.process_messages(frame,0);flow.stream.consume(size);
        }
    }

    fn process_messages(&mut self,data:&[u8],depth:usize){if depth>MAX_DEPTH{return;}let mut cursor=0;while cursor+6<=data.len(){let Some((size,type_raw))=plausible_header(data,cursor)else{return;};if cursor+size>data.len(){return;}let compressed=type_raw&0x8000!=0;let kind=type_raw&0x7fff;let payload=&data[cursor+6..cursor+size];match kind{2=>self.process_notify(payload,compressed),6=>{if payload.len()>=4{if compressed{if let Ok(nested)=zstd::stream::decode_all(Cursor::new(&payload[4..])){self.process_messages(&nested,depth+1);}}else{self.process_messages(&payload[4..],depth+1);}}},_=>{}}cursor+=size;}}

    fn process_notify(&mut self,payload:&[u8],compressed:bool){
        if payload.len()<16{return;}
        let service=be64(payload,0).unwrap_or(0);let method=be32(payload,12).unwrap_or(0);let raw=&payload[16..];let owned;
        let body=if compressed{match zstd::stream::decode_all(Cursor::new(raw)){Ok(v)=>{owned=v;&owned[..]},Err(err)=>{logging::write(format!("packet: zstd notify failed: {err}"));return;}}}else{raw};

        // One decoded server Notify stream fans out to every feature. No second Npcap session.
        self.telemetry.handle_notify(service,method,body);

        if service==proto::CHAT_SERVICE&&method==proto::CHAT_NOTIFY_NEWEST{self.sequence=self.sequence.wrapping_add(1).max(1);if let Some(message)=proto::parse_chat(body,self.sequence){self.chat.handle(&message);let _=self.tx.send(AppEvent::Chat(message));}return;}
        if service==proto::WORLD_SERVICE&&method==proto::ENTER_SCENE_METHOD{if let Some(id)=proto::parse_identity(body){let changed=self.identity.read().ok().and_then(|g|g.clone()).map(|x|x.uid!=id.uid||x.name!=id.name).unwrap_or(true);if changed{if let Ok(mut g)=self.identity.write(){*g=Some(id.clone());}let _=self.tx.send(AppEvent::Identity(id));}}}
        if service==proto::WORLD_SERVICE&&method==proto::READY_ALL_METHOD{
            match parse_ready_open(body) {
                Some(true) => {
                    if allow_after(&mut self.last_ready,Duration::from_secs(3)){self.alert(AlertKind::Ready,"BPSR Ready Check","Party Ready Check started.");}
                }
                Some(false) => logging::write("packet: ignored NotifyAllMemberReady close event"),
                None => logging::write("packet: ignored malformed NotifyAllMemberReady"),
            }
            return;
        }
        if service==proto::WORLD_SERVICE&&method==proto::READY_CAPTAIN_METHOD{return;}
        if service==proto::MATCH_SERVICE&&method==proto::MATCH_ENTER_RESULT_METHOD&&proto::parse_match_wait_ready(body){if allow_after(&mut self.last_match,Duration::from_secs(5)){self.alert(AlertKind::Queue,"BPSR Match Found","Matchmaking is waiting for acceptance.");}return;}
        if service==proto::TEAM_SERVICE&&method==proto::TEAM_ACTIVITY_METHOD&&proto::parse_team_activity_voting(body){if allow_after(&mut self.last_vote,Duration::from_secs(5)){self.alert(AlertKind::Queue,"BPSR Party Ready Vote","A party activity is waiting for your vote.");}return;}
        if service==proto::TEAM_SERVICE&&matches!(method,proto::TEAM_INVITATION_METHOD|proto::TEAM_APPLY_JOIN_METHOD){
            if !valid_proto_message(body){logging::write(format!("packet: ignored malformed team notify method={method}"));return;}
            let hash=fnv1a(body);let slot=if method==proto::TEAM_INVITATION_METHOD{&mut self.last_invite}else{&mut self.last_request};if slot.0==hash&&slot.1.map(|x|x.elapsed()<Duration::from_secs(5)).unwrap_or(false){return;}*slot=(hash,Some(Instant::now()));if method==proto::TEAM_INVITATION_METHOD{self.alert(AlertKind::PartyInvite,"BPSR Party Invite","You received a party invitation.");}else{self.alert(AlertKind::PartyRequest,"BPSR Party Join Request","Someone requested to join your party.");}
        }
    }
    fn alert(&self,kind:AlertKind,title:&str,message:&str){let _=self.tx.send(AppEvent::Alert(AlertEvent{kind,title:title.into(),message:message.into()}));}
}

fn allow_after(slot:&mut Option<Instant>,window:Duration)->bool{if slot.map(|x|x.elapsed()<window).unwrap_or(false){return false;}*slot=Some(Instant::now());true}
fn parse_ready_open(data:&[u8])->Option<bool>{
    if !valid_proto_message(data){return None;}
    let mut p=0usize;
    let mut open=None;
    while p<data.len(){
        let key=proto::read_varint(data,&mut p)?;
        let field=key>>3;
        let wire=(key&7) as u8;
        if field==1{
            if wire!=0{return None;}
            open=Some(proto::read_varint(data,&mut p)?!=0);
            continue;
        }
        match wire{
            0=>{proto::read_varint(data,&mut p)?;},
            1=>{p=p.checked_add(8)?;},
            2=>{let len=usize::try_from(proto::read_varint(data,&mut p)?).ok()?;p=p.checked_add(len)?;},
            5=>{p=p.checked_add(4)?;},
            _=>return None,
        }
        if p>data.len(){return None;}
    }
    open
}
fn valid_proto_message(data:&[u8])->bool{
    if data.is_empty(){return false;}
    let mut p=0usize;
    while p<data.len(){
        let Some(key)=proto::read_varint(data,&mut p)else{return false;};
        if key>>3==0{return false;}
        match (key&7) as u8{
            0=>{if proto::read_varint(data,&mut p).is_none(){return false;}},
            1=>{let Some(end)=p.checked_add(8)else{return false;};if end>data.len(){return false;}p=end;},
            2=>{let Some(raw_len)=proto::read_varint(data,&mut p)else{return false;};let Ok(len)=usize::try_from(raw_len)else{return false;};let Some(end)=p.checked_add(len)else{return false;};if end>data.len(){return false;}p=end;},
            5=>{let Some(end)=p.checked_add(4)else{return false;};if end>data.len(){return false;}p=end;},
            _=>return false,
        }
    }
    true
}
fn seq_before(a:u32,b:u32)->bool{(a.wrapping_sub(b) as i32)<0}
fn plausible_header(data:&[u8],offset:usize)->Option<(usize,u16)>{if offset+6>data.len(){return None;}let size=be32(data,offset)? as usize;let t=be16(data,offset+4)?;if size<6||size>MAX_GAME_FRAME||(t&0x7fff)>8{return None;}Some((size,t))}
fn find_strong_frame(data:&[u8],start:usize)->Option<(usize,usize,u16)>{if data.len()<6{return None;}for offset in start.min(data.len()-6)..=data.len()-6{let Some((size,t))=plausible_header(data,offset)else{continue;};let kind=t&0x7fff;if kind==2&&size>=22&&offset+22<=data.len(){if known_service(be64(data,offset+6).unwrap_or(0)){return Some((offset,size,t));}}if size<=data.len()-offset{let next=offset+size;if plausible_header(data,next).is_some_and(|(s,_)|s<=data.len().saturating_sub(next)){return Some((offset,size,t));}}if kind!=6||size<10{continue;}if t&0x8000!=0{let z=offset+10;if size>=14&&z+4<=data.len()&&data[z..z+4]==[0x28,0xb5,0x2f,0xfd]{return Some((offset,size,t));}}else{let nested=offset+10;if nested+22<=data.len()&&size>=32{if let Some((_,nt))=plausible_header(data,nested){if nt&0x7fff==2&&known_service(be64(data,nested+6).unwrap_or(0)){return Some((offset,size,t));}}}}}None}
fn known_service(s:u64)->bool{matches!(s,proto::CHAT_SERVICE|proto::WORLD_SERVICE|proto::MATCH_SERVICE|proto::TEAM_SERVICE)}
fn fnv1a(data:&[u8])->u64{let mut hash=14_695_981_039_346_656_037u64;for b in data{hash^=u64::from(*b);hash=hash.wrapping_mul(1_099_511_628_211);}hash}
fn be16(d:&[u8],at:usize)->Option<u16>{Some(u16::from_be_bytes(d.get(at..at+2)?.try_into().ok()?))}
fn be32(d:&[u8],at:usize)->Option<u32>{Some(u32::from_be_bytes(d.get(at..at+4)?.try_into().ok()?))}
fn be64(d:&[u8],at:usize)->Option<u64>{Some(u64::from_be_bytes(d.get(at..at+8)?.try_into().ok()?))}

#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn strong_known_notify(){let mut d=vec![0u8;22];d[0..4].copy_from_slice(&22u32.to_be_bytes());d[4..6].copy_from_slice(&2u16.to_be_bytes());d[6..14].copy_from_slice(&proto::CHAT_SERVICE.to_be_bytes());assert_eq!(find_strong_frame(&d,0).map(|x|x.0),Some(0));}
 #[test]fn proto_validation_rejects_false_ready_payloads(){assert!(!valid_proto_message(&[]));assert!(!valid_proto_message(&[0x08,0x80]));assert!(!valid_proto_message(&[0x00]));assert!(valid_proto_message(&[0x08,0x01]));}
 #[test]fn ready_open_close_semantics(){assert_eq!(parse_ready_open(&[0x08,0x01]),Some(true));assert_eq!(parse_ready_open(&[0x08,0x00]),Some(false));assert_eq!(parse_ready_open(&[]),None);assert_eq!(parse_ready_open(&[0x10,0x01]),None);}
 #[test]fn seq_wrap(){assert!(seq_before(u32::MAX-2,3));}
}