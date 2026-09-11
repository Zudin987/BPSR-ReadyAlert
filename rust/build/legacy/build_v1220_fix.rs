use std::{env,fs,path::PathBuf};

mod base {
    include!("build_v1214_fix.rs");
    pub fn run(){main();}
}

mod raid {
    include!("build_v1220.rs");
    pub fn apply(out:&std::path::Path){patch_feature_overlay(out);}
}

fn insert_fn_guard(source:&mut String,signature:&str,guard:&str,label:&str){
    let count=source.matches(signature).count();
    assert_eq!(count,1,"v1.22.0 fix {label} expected one function, found {count}");
    let start=source.find(signature).expect("function signature checked");
    let open_rel=source[start..].find('{').unwrap_or_else(||panic!("v1.22.0 fix {label} function body missing"));
    source.insert_str(start+open_rel+1,guard);
}

fn main(){
    base::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay=out.join("feature_overlays_v170_fixed.rs");

    // v1.21.x evolved hover_badge_at, so the exact historical condition used by
    // build_v1220 may no longer occur. Give that one legacy replacement a harmless
    // comment anchor, then apply the real behavior below by function signature.
    let legacy_hover="if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}";
    let mut source=fs::read_to_string(&overlay).expect("read v1.21.4 generated overlays").replace("\r\n","\n");
    let count=source.matches(legacy_hover).count();
    assert!(count<=1,"v1.22.0 fix hover compatibility anchor unexpectedly matched {count} times");
    if count==0{
        source.push_str("\n/* v1.22.0 legacy hover anchor: ");
        source.push_str(legacy_hover);
        source.push_str(" */\n");
        fs::write(&overlay,&source).expect("write v1.22.0 hover compatibility anchor");
    }

    raid::apply(&out);

    let mut source=fs::read_to_string(&overlay).expect("read v1.22.0 raid generated overlays").replace("\r\n","\n");
    insert_fn_guard(
        &mut source,
        "unsafe fn hover_badge_at(",
        "if raid_active(state){return None;}",
        "raid badge hover guard",
    );
    fs::write(&overlay,source).expect("write v1.22.0 raid hover fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1220_fix.rs");
}
