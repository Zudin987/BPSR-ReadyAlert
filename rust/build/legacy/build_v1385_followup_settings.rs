// R07/R08 follow-up: move actual Windows settings controls, not HTML labels.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1384_followup_measured_layout.rs");pub fn run(){main();}}
fn edit(source:&mut String,old:&str,new:&str,id:&str){assert_eq!(source.matches(old).count(),1,"follow-up {id}: outdated/ambiguous settings anchor");*source=source.replacen(old,new,1);assert!(source.contains(new),"follow-up {id}: missing native output");}
fn main(){previous::run();let file=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");let mut src=fs::read_to_string(&file).expect("native settings source");
edit(&mut src,"feature_label(hwnd,0,\"Collapse\",20,136,116,22);feature_combo_sized(hwnd,7003,146,130,132,&COLLAPSE_EDGE_ITEMS);","feature_label(hwnd,0,\"Collapse\",20,164,116,22);feature_combo_sized(hwnd,7003,146,158,132,&COLLAPSE_EDGE_ITEMS);","R07 separate collapse row");
edit(&mut src,"feature_label(hwnd,0,\"UI size affects text and controls; readable minimums apply.\",20,116,670,18);","feature_label(hwnd,0,\"Scales text and controls; minimum readable sizes apply.\",20,126,670,20);","R07 compact helper above collapse");
for(old,new,id)in [
("\"DISPLAY\",20,174,180,18","\"DISPLAY\",20,202,180,18","R07 DPS display heading"),
("20+col*286,196+row*27,270","20+col*286,224+row*27,270","R07 DPS checks"),
("\"Food / Serum indicators\",306,277,270","\"Food / Serum indicators\",306,305,270","R07 food serum"),
("\"ROSTER & HISTORY\",20,312,220,18","\"ROSTER & HISTORY\",20,340,220,18","R07 history heading"),
("20+col*286,334+row*27,270","20+col*286,362+row*27,270","R07 roster checks"),
("\"Max rows\",20,398,90,22","\"Max rows\",20,426,90,22","R08 complete row count label"),
("7010,110,392,96","7010,110,420,96","R07 row selector"),
("\"Encounter history\",288,398,116,22","\"Encounter history\",288,426,116,22","R07 history selector label"),
("7014,406,392,142","7014,406,420,142","R07 history selector"),
("\"Changes save immediately.\",20,438,250,20","\"Changes save immediately.\",20,466,250,20","R07 save helper"),
("feature_button(hwnd,2,\"Close\",20,432,92)","feature_button(hwnd,2,\"Close\",20,460,92)","R07 close button"),
("feature_label(hwnd,7020,\"\",20,184,260,24)","feature_label(hwnd,7020,\"\",20,212,260,24)","R07 tracker subtitle"),
("\"Event Tracker\",534,180,150","\"Event Tracker\",534,208,150","R07 tracker action"),
("\"COMBAT ATTRIBUTES\",20,224,300,20","\"COMBAT ATTRIBUTES\",20,252,300,20","R07 tracker heading"),
("20+(i/rows)as i32*224,250+(i%rows)as i32*28,216","20+(i/rows)as i32*224,278+(i%rows)as i32*28,216","R07 tracker grid"),
("let bottom=250+rows as i32*28;","let bottom=278+rows as i32*28;","R07 tracker footer")
]{edit(&mut src,old,new,id);}
fs::write(file,src).expect("write shifted native settings");println!("cargo:rerun-if-changed=build/legacy/build_v1385_followup_settings.rs");}
