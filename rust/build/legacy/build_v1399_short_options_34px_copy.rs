// Follow-up to the user's native screenshots: short dropdowns should not carry
// WS_VSCROLL just because their factory creates a combo. Keep a genuine scroll
// path on cramped work areas; do not paint over a native scrollbar.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1398_native_caption_combo_normal_pitch.rs");pub fn run(){main();}}
fn once(s:&mut String,a:&str,b:&str,id:&str){assert_eq!(s.matches(a).count(),1,"v1399 {id} anchor");*s=s.replacen(a,b,1);}
fn main(){
 previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 let meter=out.join("feature_overlays_v170_fixed.rs");let mut m=fs::read_to_string(&meter).expect("meter");
 once(&mut m,"if scale<=80{38}else{36}","if scale<=80{38}else{34}","Normal row 34 at 100 percent; Raid unchanged");
 assert_eq!(m.matches("0x00210213").count(),2,"two feature combo factories");
 m=m.replace("0x00210213","0x00010213"); // remove only WS_VSCROLL
 fs::write(&meter,m).expect("write meter");
 for (name,from,to) in [
   ("settings_ui_v1160_fixed.rs","WS_TABSTOP|WS_VSCROLL|CBS_DROPDOWNLIST","WS_TABSTOP|CBS_DROPDOWNLIST"),
   ("event_tracker_ui_v1160_fixed.rs","WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST","WS_TABSTOP | CBS_DROPDOWNLIST"),
 ] {
  let p=out.join(name);let mut s=fs::read_to_string(&p).expect("settings/event");
  once(&mut s,from,to,"do not force combo scrollbar");
  fs::write(p,s).expect("write short combo factory");
 }
 let p=out.join("ui_pixel_controls_v1316.rs");let mut c=fs::read_to_string(&p).expect("combo controller");
 once(&mut c,"    // This is the REAL native scrollbar, never a painted-over decoration.\n    // Hide its otherwise-white track when all fixed choices fit; show it again\n    // for constrained monitors so thumb/wheel/arrows remain functional.\n    combo_show_scroll_bar(info.hwndList,1,(wanted>list_h) as i32);",
 r#"    // Own the actual listbox scrollbar STYLE, not a decorative overpaint.
    // Combobox factories do not force it for 3-6 choices. Re-enable the true
    // scroll rail only if the monitor cannot accommodate all choices.
    let needs_scroll=wanted>list_h;
    let old_style=GetWindowLongPtrW(info.hwndList,GWL_STYLE_);
    let next_style=if needs_scroll{old_style|0x0020_0000isize}else{old_style&!0x0020_0000isize};
    if next_style!=old_style{
        SetWindowLongPtrW(info.hwndList,GWL_STYLE_,next_style);
        SetWindowPos(info.hwndList,null_mut(),0,0,0,0,0x0037); // FRAMECHANGED, keep popup position
    }
    combo_show_scroll_bar(info.hwndList,1,needs_scroll as i32);"#,"native actual scroll style switches with work-area fit");
 fs::write(p,c).expect("write popup");
 // Apply requested punctuation to generated native UI copy after all earlier
 // checked build-time transforms have run. Never alter game icon glyph artwork.
 let mut changed=0usize;
 for entry in fs::read_dir(&out).expect("generated source directory") {
  let path=entry.expect("generated entry").path();if path.extension().and_then(|x|x.to_str())!=Some("rs"){continue;}
  let mut s=fs::read_to_string(&path).expect("generated UTF-8 source");
  if s.contains('—'){changed+=s.matches('—').count();s=s.replace('—',"-");fs::write(&path,s).expect("write hyphen copy");}
 }
 assert!(changed>=3,"expected player-detail title and native placeholder copy");
 println!("cargo:rerun-if-changed=build/legacy/build_v1399_short_options_34px_copy.rs");
}
