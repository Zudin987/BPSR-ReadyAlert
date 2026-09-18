// Keep legacy regression assertions consistent with the intentionally safer
// default selection introduced by the final UI audit. The contrast pass
// operates on generated source and preserves all approved DPS geometry.
use std::{env,fs,path::PathBuf};
mod previous {
    include!("build_v1368_audit_contrast.rs");
    pub fn run(){main();}
}
fn main(){
    previous::run();
    let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("ui_pixel_qa_v1316.rs");
    let mut source=fs::read_to_string(&path).expect("QA source");
    let old="assert_eq!(buttons[2].id, QA_IDCANCEL);";
    assert_eq!(source.matches(old).count(),1,"legacy default-action assertion moved");
    source=source.replacen(old,"assert_eq!(buttons[0].id, QA_IDCANCEL);",1);
    fs::write(path,source).expect("write QA regression");
    println!("cargo:rerun-if-changed=build/legacy/build_v1368_audit_test_alignment.rs");
}
