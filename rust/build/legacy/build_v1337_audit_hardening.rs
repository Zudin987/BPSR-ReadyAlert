use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1335_stronger_wipe_detection.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("capture_v185.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated capture for decompression safety regression")
        .replace("\r\n", "\n");

    // Generated capture already has a bounded decoder, used by both compressed
    // Notify bodies and nested bundles. Fail closed if a future upstream build
    // step removes that protection rather than adding a duplicate decoder.
    assert!(source.contains("const MAX_DECODED_PACKET: usize = 32 * 1024 * 1024;"),
        "generated capture decoded packet limit changed: review decompression safety");
    assert_eq!(source.matches("fn decode_zstd_limited(").count(), 1,
        "expected exactly one bounded capture decoder");
    assert!(source.contains("decode_zstd_limited(&payload[4..])")
        && source.contains("decode_zstd_limited(raw)"),
        "both compressed message types must use the bounded decoder");
    assert!(!source.contains("zstd::stream::decode_all("),
        "generated capture introduced unbounded zstd decompression");

    source.push_str(r#"

#[cfg(test)]
mod audit_capture_decoder_tests {
    use super::*;

    #[test]
    fn rejects_compressed_output_past_limit() {
        let payload = vec![0u8; MAX_DECODED_PACKET + 1];
        let compressed = zstd::stream::encode_all(Cursor::new(&payload), 1).unwrap();
        assert!(decode_zstd_limited(&compressed).is_err());
    }
}
"#);
    fs::write(path, source).expect("write capture decompression regression test");

    // Keep the original updater source untouched, as older build stages
    // generate and transform the active implementation in OUT_DIR.
    let updater_path = out.join("updater_v1241.rs");
    let mut updater = fs::read_to_string(&updater_path)
        .expect("read generated updater for automatic download consent")
        .replace("\r\n", "\n");
    let from = "auto_download: true,";
    assert_eq!(updater.matches(from).count(), 1,
        "expected one updater auto-download default; review consent behavior");
    updater = updater.replacen(from, "auto_download: false,", 1);
    updater.push_str(r#"

#[cfg(test)]
mod audit_updater_privacy_tests {
    use super::*;

    #[test]
    fn new_installations_do_not_download_updates_without_consent() {
        let prefs = UpdatePreferences::default();
        assert!(prefs.auto_check);
        assert!(!prefs.auto_download);
    }
}
"#);
    fs::write(updater_path, updater).expect("write safer updater default and regression test");
    println!("cargo:rerun-if-changed=build/legacy/build_v1337_audit_hardening.rs");
}
