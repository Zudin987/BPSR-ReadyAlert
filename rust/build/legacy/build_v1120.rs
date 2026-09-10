use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1110.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.12.0 patch `{label}` expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_exact_count(source:&mut String,from:&str,to:&str,expected:usize,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,expected,"v1.12.0 patch `{label}` expected {expected} matches, found {count}");
    *source=source.replace(from,to);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.12.0 patch `{label}` start expected one match, found {count}");
    let begin=source.find(start).expect("v1.12 start checked");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.12.0 patch `{label}` end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // ---- Combat skill statistics -------------------------------------------------
    // Keep the old generated telemetry core, but retain the smallest positive
    // skill event as well as the existing maximum. The model field is serde-
    // defaulted so old v1.9-v1.11 history remains readable.
    let telemetry_path=out.join("telemetry_v170_fixed.rs");
    let mut telemetry=fs::read_to_string(&telemetry_path).expect("read generated v1.11 telemetry");
    replace_once(
        &mut telemetry,
        "    luckies: u64,\n    max_value: i64,",
        "    luckies: u64,\n    min_value: i64,\n    max_value: i64,",
        "skill min accumulator",
    );
    replace_once(
        &mut telemetry,
        "        stat.max_value = stat.max_value.max(value);",
        "        stat.min_value = if stat.min_value <= 0 { value } else { stat.min_value.min(value) };\n        stat.max_value = stat.max_value.max(value);",
        "outgoing skill min observation",
    );
    replace_once(
        &mut telemetry,
        "                    incoming.max_value = incoming.max_value.max(value);",
        "                    incoming.min_value = if incoming.min_value <= 0 { value } else { incoming.min_value.min(value) };\n                    incoming.max_value = incoming.max_value.max(value);",
        "incoming skill min observation",
    );
    replace_exact_count(
        &mut telemetry,
        "                    lucky_hits: value.luckies,\n                    max_value: value.max_value,",
        "                    lucky_hits: value.luckies,\n                    min_value: value.min_value,\n                    max_value: value.max_value,",
        2,
        "skill min payloads",
    );
    fs::write(&telemetry_path,telemetry).expect("write v1.12 generated telemetry");

    // ---- Lag-tolerant capture ----------------------------------------------------
    // v1.6's TCP reassembler could immediately jump to a later complete frame on
    // the first out-of-order segment. Under high jitter that can discard a packet
    // that was merely delayed. Give normal TCP reordering a grace period, retain
    // more pending data, and only hard-resync after a longer bounded timeout.
    let capture_path=out.join("capture_v185.rs");
    let mut capture=fs::read_to_string(&capture_path).expect("read generated v1.11 capture");
    replace_once(&mut capture,"    io::Cursor,","    io::{Cursor, Read},","bounded decode Read import");
    replace_once(&mut capture,"const MAX_GAME_FRAME: usize = 2 * 1024 * 1024;","const MAX_GAME_FRAME: usize = 8 * 1024 * 1024;","larger valid game frame cap");
    replace_once(&mut capture,"const MAX_PENDING: usize = 2 * 1024 * 1024;","const MAX_PENDING: usize = 8 * 1024 * 1024;","larger reorder buffer");
    replace_once(&mut capture,"const MAX_UNSYNC: usize = 512 * 1024;\nconst UNSYNC_TAIL: usize = 128 * 1024;","const MAX_UNSYNC: usize = 2 * 1024 * 1024;\nconst UNSYNC_TAIL: usize = 256 * 1024;\nconst MAX_DECODED_PACKET: usize = 32 * 1024 * 1024;\nconst REORDER_GRACE: Duration = Duration::from_millis(750);\nconst GAP_RECOVERY: Duration = Duration::from_secs(5);","capture safety limits");

    replace_between(
        &mut capture,
        "    fn insert_segment(&mut self, flow:&mut FlowState, mut seq:u32, mut data:&[u8]){",
        "    fn drain_pending(&mut self,flow:&mut FlowState){",
        r#"    fn insert_segment(&mut self, flow:&mut FlowState, mut seq:u32, mut data:&[u8]){
        if flow.next_seq.is_none(){flow.next_seq=Some(seq);flow.synchronized=false;}
        let next=flow.next_seq.unwrap();
        if seq_before(seq,next){let overlap=next.wrapping_sub(seq) as usize;if overlap>=data.len(){return;}data=&data[overlap..];seq=next;}
        if seq==flow.next_seq.unwrap(){self.append_stream(flow,data);flow.next_seq=Some(seq.wrapping_add(data.len()as u32));self.drain_pending(flow);if flow.pending.is_empty(){flow.gap_started=None;}return;}
        if !flow.pending.contains_key(&seq){flow.pending_bytes=flow.pending_bytes.saturating_add(data.len());flow.pending.insert(seq,data.to_vec());}
        flow.gap_started.get_or_insert_with(Instant::now);
        let elapsed=flow.gap_started.map(|x|x.elapsed()).unwrap_or_default();
        if gap_fast_resync_ready(elapsed,flow.pending_bytes,flow.pending.len())&&self.try_fast_recover_gap(flow){return;}
        if elapsed>=GAP_RECOVERY||flow.pending_bytes>MAX_PENDING||flow.pending.len()>=512{self.recover_gap(flow);}
    }

"#,
        "lag tolerant segment insertion",
    );
    replace_between(
        &mut capture,
        "    fn drain_pending(&mut self,flow:&mut FlowState){",
        "    fn try_fast_recover_gap(&mut self,flow:&mut FlowState)->bool{",
        r#"    fn drain_pending(&mut self,flow:&mut FlowState){
        loop{
            let next=match flow.next_seq{Some(v)=>v,None=>return};
            if let Some(exact)=flow.pending.remove(&next){flow.pending_bytes=flow.pending_bytes.saturating_sub(exact.len());self.append_stream(flow,&exact);flow.next_seq=Some(next.wrapping_add(exact.len()as u32));continue;}
            let Some((&first_key,_))=flow.pending.iter().next()else{flow.gap_started=None;return;};
            if !seq_before(first_key,next){
                flow.gap_started.get_or_insert_with(Instant::now);
                let elapsed=flow.gap_started.map(|x|x.elapsed()).unwrap_or_default();
                if gap_fast_resync_ready(elapsed,flow.pending_bytes,flow.pending.len())&&self.try_fast_recover_gap(flow){return;}
                return;
            }
            let data=flow.pending.remove(&first_key).unwrap();
            flow.pending_bytes=flow.pending_bytes.saturating_sub(data.len());
            let overlap=next.wrapping_sub(first_key)as usize;
            if overlap>=data.len(){continue;}
            let tail=&data[overlap..];
            self.append_stream(flow,tail);
            flow.next_seq=Some(next.wrapping_add(tail.len()as u32));
        }
    }

"#,
        "lag tolerant pending drain",
    );

    replace_once(
        &mut capture,
        "zstd::stream::decode_all(Cursor::new(&payload[4..]))",
        "decode_zstd_limited(&payload[4..])",
        "bounded nested zstd decode",
    );
    replace_once(
        &mut capture,
        "zstd::stream::decode_all(Cursor::new(raw))",
        "decode_zstd_limited(raw)",
        "bounded notify zstd decode",
    );
    replace_once(
        &mut capture,
        "fn seq_before(a:u32,b:u32)->bool{(a.wrapping_sub(b) as i32)<0}",
        r#"fn gap_fast_resync_ready(elapsed:Duration,pending_bytes:usize,pending_segments:usize)->bool{elapsed>=REORDER_GRACE&&(pending_bytes>=256*1024||pending_segments>=16)}
fn decode_zstd_limited(data:&[u8])->Result<Vec<u8>,String>{let decoder=zstd::stream::read::Decoder::new(Cursor::new(data)).map_err(|err|err.to_string())?;let mut bytes=Vec::new();decoder.take((MAX_DECODED_PACKET+1)as u64).read_to_end(&mut bytes).map_err(|err|err.to_string())?;if bytes.len()>MAX_DECODED_PACKET{return Err(format!("decoded packet exceeds {} MiB safety limit",MAX_DECODED_PACKET/(1024*1024)));}Ok(bytes)}
fn seq_before(a:u32,b:u32)->bool{(a.wrapping_sub(b) as i32)<0}"#,
        "capture recovery helpers",
    );

    // Surface real Npcap kernel/interface drop counters. A single old drop does
    // not permanently poison health: warning state is based on new drops since
    // the previous five-second health sample while the total remains visible.
    replace_once(
        &mut capture,
        "            let mut last_status_emit = Instant::now().checked_sub(Duration::from_secs(10)).unwrap_or_else(Instant::now);\n            let mut reopen = false;",
        "            let mut last_status_emit = Instant::now().checked_sub(Duration::from_secs(10)).unwrap_or_else(Instant::now);\n            let mut last_drop_total = 0u64;\n            let mut reopen = false;",
        "capture drop baseline",
    );
    replace_once(
        &mut capture,
        r#"                        let health = if !running {
                            "Capture ready"
                        } else if packet_recent && frame_recent {
                            "Capture OK"
                        } else if packet_recent {
                            "Capture warning: protocol frames stale"
                        } else {
                            "Capture warning: no recent game packets"
                        };
                        let detail = if running {
                            format!("{health} | {} | packet {} | frame {}", device.description, age_text(last_packet), age_text(processor.last_valid_frame))
                        } else {
                            format!("{health} | {} | game not detected", device.description)
                        };"#,
        r#"                        let stats=handle.stats();
                        let drop_total=stats.map(|s|u64::from(s.dropped).saturating_add(u64::from(s.interface_dropped))).unwrap_or(0);
                        let new_drops=drop_total.saturating_sub(last_drop_total);
                        last_drop_total=drop_total;
                        let health = if !running {
                            "Capture ready"
                        } else if new_drops>0 {
                            "Capture warning: Npcap dropped packets"
                        } else if packet_recent && frame_recent {
                            "Capture OK"
                        } else if packet_recent {
                            "Capture warning: protocol frames stale"
                        } else {
                            "Capture warning: no recent game packets"
                        };
                        let stats_text=stats.map(|s|format!(" | recv {} | drop +{} ({})",s.received,new_drops,drop_total)).unwrap_or_default();
                        let detail = if running {
                            format!("{health} | {} | packet {} | frame {}{}", device.description, age_text(last_packet), age_text(processor.last_valid_frame),stats_text)
                        } else {
                            format!("{health} | {} | game not detected{}", device.description,stats_text)
                        };"#,
        "capture drop health status",
    );
    replace_once(
        &mut capture,
        " #[test]fn seq_wrap(){assert!(seq_before(u32::MAX-2,3));}\n}",
        " #[test]fn seq_wrap(){assert!(seq_before(u32::MAX-2,3));}\n #[test]fn reorder_grace_prevents_immediate_resync(){assert!(!gap_fast_resync_ready(Duration::from_millis(100),512*1024,32));assert!(gap_fast_resync_ready(Duration::from_millis(800),512*1024,32));}\n #[test]fn bounded_decode_roundtrip(){let encoded=zstd::stream::encode_all(Cursor::new(b\"readyalert\"),1).unwrap();assert_eq!(decode_zstd_limited(&encoded).unwrap(),b\"readyalert\");}\n}",
        "capture reliability regression tests",
    );
    fs::write(&capture_path,capture).expect("write v1.12 generated capture");

    // ---- Entity Inspector ---------------------------------------------------------
    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.11 overlay");
    replace_between(
        &mut overlay,
        "unsafe fn open_detail(parent:HWND,state:&mut State,row:DpsRow){",
        "unsafe fn update_detail(hwnd:HWND,row:DpsRow,encounter_ms:u64){",
        include_str!("overlay_v1120_open_detail_patch.txt"),
        "screen-safe Entity Inspector placement",
    );
    replace_between(
        &mut overlay,
        "unsafe fn paint_detail_skills(hdc:HDC,rc:RECT,state:&DetailState){",
        "unsafe fn paint_detail_taken(hdc:HDC,rc:RECT,state:&DetailState){",
        include_str!("overlay_v1120_skill_table_patch.txt"),
        "rich skill statistics table",
    );
    fs::write(&overlay_path,overlay).expect("write v1.12 generated overlay");

    println!("cargo:rerun-if-changed=build_v1120.rs");
    println!("cargo:rerun-if-changed=overlay_v1120_open_detail_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1120_skill_table_patch.txt");
    println!("cargo:rerun-if-changed=src/model_v170.rs");
    println!("cargo:rerun-if-changed=src/npcap.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v190.rs");
}
