use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1321_freeze_hardening_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.1 future mechanics fix {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let input = manifest.join("src/future_mechanics.rs");
    let mut source = fs::read_to_string(&input)
        .expect("read future_mechanics.rs for v1.32.1 edge fixes")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "                    let tip_id = ints.first().copied().map(|value| value as i32).unwrap_or(0);\n                    let text = event_strings(event).join(\" | \");",
        "                    let strings = event_strings(event);\n                    let tip_id = ints.first().copied().map(|value| value as i32).filter(|id| *id > 0)\n                        .or_else(|| strings.iter().find_map(|value| value.trim().parse::<i32>().ok().filter(|id| *id > 0)))\n                        .unwrap_or(0);\n                    let text = strings.into_iter()\n                        .filter(|value| value.trim().parse::<i32>().ok() != Some(tip_id))\n                        .collect::<Vec<_>>()\n                        .join(\" | \");",
        "NoticeTip numeric strParams fallback",
    );

    replace_once(
        &mut source,
        "        let duration_ms = rule\n            .as_ref()\n            .map(|rule| rule.duration_ms)\n            .filter(|duration| *duration > 0)\n            .unwrap_or(packet_ms)\n            .clamp(1, MAX_TRANSIENT_MS);",
        "        let duration_ms = rule\n            .as_ref()\n            .map(|rule| rule.duration_ms)\n            .filter(|duration| *duration > 0)\n            .or_else(|| (packet_ms > 0).then_some(packet_ms))\n            .unwrap_or(DEFAULT_TRANSIENT_MS)\n            .clamp(1, MAX_TRANSIENT_MS);",
        "zero-duration DBM fallback",
    );
    replace_once(
        &mut source,
        "            expires_unix_ms: now.saturating_add(duration_ms.max(DEFAULT_TRANSIENT_MS.min(duration_ms))),",
        "            expires_unix_ms: now.saturating_add(duration_ms),",
        "DBM expiry expression",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1321_future_mechanic_edge_tests {
    use super::*;

    fn varint(mut value: u64, out: &mut Vec<u8>) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            out.push(byte);
            if value == 0 { break; }
        }
    }

    fn len_field(field: u32, payload: &[u8], out: &mut Vec<u8>) {
        varint(u64::from(field) << 3 | 2, out);
        varint(payload.len() as u64, out);
        out.extend_from_slice(payload);
    }

    fn runtime() -> Runtime {
        Runtime {
            root: None,
            rules: Vec::new(),
            current_scene_id: 0,
            local_uid: 0,
            rows: HashMap::new(),
            attribute_values: HashMap::new(),
            notice_seen: HashSet::new(),
        }
    }

    #[test]
    fn zero_duration_dbm_uses_safe_transient_default() {
        let mut runtime = runtime();
        assert!(runtime.observe_dbm(12_345_601, 0, 0));
        let row = runtime.rows.values().next().expect("DBM row");
        assert_eq!(row.expires_unix_ms - row.created_unix_ms, DEFAULT_TRANSIENT_MS);
    }

    #[test]
    fn notice_tip_accepts_numeric_id_from_string_params() {
        let mut runtime = runtime();
        runtime.rules.push(Rule {
            scene_id: 0,
            kind: TriggerKind::NoticeTip,
            event_id: 12_345,
            label: "Move away".into(),
            duration_ms: 1_000,
            priority: 3,
        });

        let mut event = vec![0x08, EVENT_NOTICE_TIP as u8];
        len_field(5, b"12345", &mut event);
        len_field(5, b"Move now", &mut event);
        let mut list = Vec::new();
        len_field(2, &event, &mut list);
        let mut body = Vec::new();
        len_field(1, &list, &mut body);

        assert!(runtime.observe_scene_events(&body));
        let row = runtime.rows.values().next().expect("NoticeTip row");
        assert_eq!(row.label, "Move away");
        assert_eq!(row.target.as_deref(), Some("Move now"));
    }
}
"#);

    fs::write(out.join("future_mechanics_v1321_fixed.rs"), source)
        .expect("write generated v1.32.1 future mechanics");
    println!("cargo:rerun-if-changed={}", input.display());
    println!("cargo:rerun-if-changed=build/legacy/build_v1321_future_mechanics_fix.rs");
}
