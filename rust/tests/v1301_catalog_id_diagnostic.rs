const CATALOG_PARTS: [&[u8]; 14] = [
    include_bytes!("../data/game_names_v1300.tsv.zst.001"),
    include_bytes!("../data/game_names_v1300.tsv.zst.002"),
    include_bytes!("../data/game_names_v1300.tsv.zst.003"),
    include_bytes!("../data/game_names_v1300.tsv.zst.004"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005a"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005b"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005c"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005d"),
    include_bytes!("../data/game_names_v1300.tsv.zst.006"),
    include_bytes!("../data/game_names_v1300.tsv.zst.007"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008a"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008b"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008c"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008d"),
];

#[test]
fn diagnose_skill_ids_rejected_by_i32_parser() {
    let compressed_len: usize = CATALOG_PARTS.iter().map(|part| part.len()).sum();
    let mut compressed = Vec::with_capacity(compressed_len);
    for part in CATALOG_PARTS {
        compressed.extend_from_slice(part);
    }
    assert_eq!(compressed.len(), 78_485);

    let decoded = zstd::stream::decode_all(compressed.as_slice()).expect("decode catalog");
    let text = String::from_utf8(decoded).expect("catalog UTF-8");

    let mut skill_rows = 0usize;
    let mut i32_ok = 0usize;
    let mut u32_only = Vec::new();
    let mut invalid = Vec::new();

    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let Some(kind) = parts.next() else { continue };
        let Some(raw_id) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        if kind != "S" {
            continue;
        }
        skill_rows += 1;
        if raw_id.parse::<i32>().is_ok() {
            i32_ok += 1;
        } else if let Ok(id) = raw_id.parse::<u32>() {
            if u32_only.len() < 20 {
                u32_only.push((raw_id.to_owned(), id as i32, name.to_owned()));
            } else {
                u32_only.push((String::new(), 0, String::new()));
            }
        } else {
            invalid.push((raw_id.to_owned(), name.to_owned()));
        }
    }

    eprintln!("skill_rows={skill_rows} i32_ok={i32_ok} u32_only={} invalid={}", u32_only.len(), invalid.len());
    eprintln!("u32_only_samples={:?}", &u32_only[..u32_only.len().min(20)]);
    eprintln!("invalid_samples={:?}", &invalid[..invalid.len().min(20)]);

    assert_eq!(skill_rows, 8_457, "raw skill row count changed");
    assert_eq!(u32_only.len(), 0, "skill IDs outside signed i32 are being silently dropped");
    assert!(invalid.is_empty(), "non-numeric skill IDs found");
}
