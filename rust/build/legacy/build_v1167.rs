use std::{env, fs, path::PathBuf};

mod prior {
    include!("build_v1166.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.16.7 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_telemetry(out:&std::path::Path){
    let path=out.join("telemetry_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated v1.16.7 telemetry");

    replace_once(
        &mut source,
        r#"    fn observe_consumable_change(&mut self,host:i64,buff_uuid:i32,change:&[u8]){
        if entity_kind(host)!=ENTITY_PLAYER{return;}
        let Some((_kind,status))=self.consumable_instances.get_mut(&(host,buff_uuid))else{return;};
        // BuffChange fields: 1=layer, 2=duration, 3=create_time. Rebase the
        // countdown immediately; full snapshots remain authoritative correction.
        let duration=proto::get_varint_field(change,2).unwrap_or(0).min(24*60*60*1000)as i64;
        if duration>0{status.duration_ms=duration;status.expires_unix_ms=now_ms().saturating_add(duration);}
        if host>>16==self.local_uid{self.refresh_local_consumables(host);self.emit_mechanics(true);}
        self.emit_dps();
    }"#,
        r#"    fn observe_consumable_change(&mut self,host:i64,buff_uuid:i32,change:&[u8]){
        if entity_kind(host)!=ENTITY_PLAYER{return;}
        let Some((_kind,status))=self.consumable_instances.get_mut(&(host,buff_uuid))else{return;};
        // BuffChange fields: 1=layer, 2=duration, 3=create_time. The live
        // duration is the grant's total duration, not necessarily "time left".
        // Preserve elapsed time when a stack extends that total duration.
        // Example: 10m serum with 5m left, then total duration grows to 20m:
        // the correct remaining time is 15m, not now+20m.
        let duration=proto::get_varint_field(change,2).unwrap_or(0).min(24*60*60*1000)as i64;
        let create=proto::get_varint_field(change,3).map(signed).unwrap_or(0);
        if duration>0{
            let now=now_ms();
            let old_duration=status.duration_ms.max(0);
            let old_expiry=status.expires_unix_ms;
            let absolute_expiry=if create>=1_000_000_000_000{
                Some(create.saturating_add(duration))
            }else if create>=1_000_000_000{
                Some(create.saturating_mul(1000).saturating_add(duration))
            }else{None};
            status.expires_unix_ms=if old_expiry>now&&old_duration>0&&duration>old_duration{
                old_expiry.saturating_add(duration.saturating_sub(old_duration))
            }else if let Some(expiry)=absolute_expiry{
                expiry
            }else if old_expiry>now{
                // A redundant/tick-like BuffChange without an absolute grant
                // timestamp must not reset an already-consumed timer.
                old_expiry
            }else{
                now.saturating_add(duration)
            };
            status.duration_ms=duration;
        }
        if host>>16==self.local_uid{self.refresh_local_consumables(host);self.emit_mechanics(true);}
        self.emit_dps();
    }"#,
        "preserve elapsed serum time across stack duration growth",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1167_serum_stack_tests{
    use super::*;use std::sync::mpsc;
    fn push_varint(mut v:u64,out:&mut Vec<u8>){while v>=0x80{out.push((v as u8&0x7f)|0x80);v>>=7;}out.push(v as u8);}
    fn buff_change(layer:i32,duration:i64,create:i64)->Vec<u8>{let mut out=vec![0x08];push_varint(layer.max(0)as u64,&mut out);out.push(0x10);push_varint(duration.max(0)as u64,&mut out);out.push(0x18);push_varint(create.max(0)as u64,&mut out);out}

    #[test]
    fn second_serum_adds_only_new_duration_to_existing_remaining_time(){
        let(tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;let host=canonical_player_uuid(42);let now=now_ms();
        runtime.consumable_instances.insert((host,77),(ConsumableKind::Serum,ConsumableStatus{buff_id:2_033_011,name:"S1 Fire +240".into(),expires_unix_ms:now+5*60*1000,duration_ms:10*60*1000}));
        runtime.observe_consumable_change(host,77,&buff_change(2,20*60*1000,0));
        let after=runtime.player_consumable(host,ConsumableKind::Serum).expect("serum");let remaining=after.expires_unix_ms.saturating_sub(now_ms());
        assert!((14*60*1000..=15*60*1000+2_000).contains(&remaining),"remaining={remaining}");
        assert_eq!(after.duration_ms,20*60*1000);
    }

    #[test]
    fn same_total_duration_does_not_reset_partly_consumed_serum(){
        let(tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;let host=canonical_player_uuid(42);let now=now_ms();
        runtime.consumable_instances.insert((host,77),(ConsumableKind::Serum,ConsumableStatus{buff_id:2_033_011,name:"S1 Fire +240".into(),expires_unix_ms:now+5*60*1000,duration_ms:10*60*1000}));
        runtime.observe_consumable_change(host,77,&buff_change(1,10*60*1000,0));
        let after=runtime.player_consumable(host,ConsumableKind::Serum).expect("serum");let remaining=after.expires_unix_ms.saturating_sub(now_ms());
        assert!((4*60*1000..=5*60*1000+2_000).contains(&remaining),"remaining={remaining}");
    }
}
"#);
    fs::write(path,source).expect("write v1.16.7 telemetry");
}

fn patch_overlay(out:&std::path::Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated v1.16.7 overlay");
    replace_once(
        &mut source,
        "    SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,rgb(248,248,248));draw(hdc,label,RECT{left:x,top:y,right,bottom},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "    let old_bk=SetBkMode(hdc,TRANSPARENT as i32);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,rgb(255,255,255));draw(hdc,label,RECT{left:x,top:y,right,bottom},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetBkMode(hdc,old_bk);",
        "draw visible F S glyphs without opaque white text background",
    );
    source.push_str(r#"

#[cfg(test)]
mod v1167_consumable_popup_tests{
    use super::*;
    #[test]fn floating_indicator_geometry_stays_round_and_large(){assert_eq!(DPS_CONSUMABLE_ICON,21);assert!(DPS_CONSUMABLE_POPUP_W>=2*DPS_CONSUMABLE_ICON+DPS_CONSUMABLE_GAP);}
}
"#);
    fs::write(path,source).expect("write v1.16.7 overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1167.rs");
}
