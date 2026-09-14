use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1321_future_mechanics_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.1 regression fix {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("event_tracker_v1321_generated.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated v1.32.1 event tracker for regression fix")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "        assert_eq!(s.rules.len(), 24); assert_eq!(s.max_visible, 12);\n        assert_eq!(s.rules.iter().map(|r| r.rule_id).collect::<HashSet<_>>().len(), 24);",
        "        let expected = MAX_RULES.min(30);\n        assert_eq!(s.rules.len(), expected); assert_eq!(s.max_visible, 12);\n        assert_eq!(s.rules.iter().map(|r| r.rule_id).collect::<HashSet<_>>().len(), expected);",
        "event tracker cap regression expectation",
    );

    fs::write(path, source).expect("write v1.32.1 regression-fixed event tracker");
}
