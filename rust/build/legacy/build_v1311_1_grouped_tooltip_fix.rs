use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1311_party_tracker_meter.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read grouped mechanic overlay for tooltip fix: {e}"))
        .replace("\r\n", "\n");

    let old = "MechanicDisplayRow::Tracker(row)=>format!(\"{} • {}\",row.label,row.detail),MechanicDisplayRow::Mechanic(row)=>format!(\"{} {}\",row.label,row.target.as_deref().unwrap_or(\"\"))";
    let new = "MechanicDisplayRow::Tracker(row)=>format!(\"{} • {}\",row.label,row.detail),MechanicDisplayRow::TrackerGroup(items)=>format!(\"Tracker • {}\",items.iter().map(|(name,_)|name.as_str()).collect::<Vec<_>>().join(\", \")),MechanicDisplayRow::Mechanic(row)=>format!(\"{} {}\",row.label,row.targets.join(\", \"))";
    let count = source.matches(old).count();
    assert_eq!(count, 1, "v1.31.1 grouped mechanic tooltip target expected once, found {count}");
    source = source.replacen(old, new, 1);

    fs::write(path, source).unwrap_or_else(|e| panic!("write grouped mechanic tooltip fix: {e}"));
    println!("cargo:rerun-if-changed=build/legacy/build_v1311_1_grouped_tooltip_fix.rs");
}
