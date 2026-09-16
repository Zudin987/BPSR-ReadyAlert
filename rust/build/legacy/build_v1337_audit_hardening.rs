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
        .expect("read generated capture for bounded decompression")
        .replace("\r\n", "\n");
    let unsafe_call = "zstd::stream::decode_all(Cursor::new(";
    let count = source.matches(unsafe_call).count();
    assert_eq!(count, 2, "expected two unbounded capture decompression sites, found {count}; inspect the generated capture before changing this patch");
    source = source.replace(unsafe_call, "decode_zstd_bounded(Cursor::new(");
    source.push_str(r#"

// This cap applies to BOTH compressed Notify bodies and recursively nested
// bundled messages. The frame-size limit alone does not constrain expansion.
const MAX_DECOMPRESSED_CAPTURE_BYTES: u64 = 16 * 1024 * 1024;

fn decode_zstd_bounded<R: std::io::Read>(input: R) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;
    let decoder = zstd::stream::read::Decoder::new(input)?;
    let mut limited = decoder.take(MAX_DECOMPRESSED_CAPTURE_BYTES + 1);
    let mut output = Vec::new();
    limited.read_to_end(&mut output)?;
    if output.len() as u64 > MAX_DECOMPRESSED_CAPTURE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "compressed game message exceeded the 16 MiB decoded limit",
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod audit_bounded_capture_tests {
    use super::*;

    #[test]
    fn accepts_ordinary_compressed_notify() {
        let payload = b"ReadyAlert game message".repeat(100);
        let compressed = zstd::stream::encode_all(Cursor::new(&payload), 1).unwrap();
        assert_eq!(decode_zstd_bounded(Cursor::new(&compressed)).unwrap(), payload);
    }

    #[test]
    fn rejects_compressed_expansion_bomb() {
        let payload = vec![0u8; MAX_DECOMPRESSED_CAPTURE_BYTES as usize + 1];
        let compressed = zstd::stream::encode_all(Cursor::new(&payload), 1).unwrap();
        assert!(decode_zstd_bounded(Cursor::new(&compressed)).is_err());
    }
}
"#);
    fs::write(path, source).expect("write bounded generated capture");
    println!("cargo:rerun-if-changed=build/legacy/build_v1337_audit_hardening.rs");
}
