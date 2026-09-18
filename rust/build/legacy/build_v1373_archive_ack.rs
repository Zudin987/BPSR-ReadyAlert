// Archive success must mean the saved record is also visible in regenerated
// Encounter History HTML. Report a partial save accurately if HTML refresh fails.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1372_capture_reconnect.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, from: &str, to: &str, label: &str) {
    let count = src.matches(from).count();
    assert_eq!(count, 1, "archive acknowledgement {label}: expected one anchor, found {count}");
    *src = src.replacen(from, to, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v1270_lifecycle.rs");
    let mut src = fs::read_to_string(&path).expect("generated benchmark telemetry").replace("\r\n", "\n");
    replace_once(&mut src,
        "                if let Some(tx) = job.completion_tx.as_ref() {\n                    let _ = tx.send(AppEvent::Alert(AlertEvent {\n                        kind: AlertKind::Ready,\n                        title: \"Benchmark complete\".into(),\n                        message: format!(\"Stop attacking. Result saved to Encounter History ({} seconds).\", record.context.benchmark_duration_ms / 1000),\n                    }));\n                }\n                // Keep the already-open offline page refreshable without putting",
        "                // Do not announce success before the HTML page is updated.\n                // Keep the already-open offline page refreshable without putting",
        "premature success notification");
    replace_once(&mut src,
        "                if let Err(err) = encounter_archive::generate(&root) {\n                    logging::write(format!(\"encounter-history: refresh page generation failed: {err}\"));\n                }",
        "                let refreshed = match encounter_archive::generate(&root) {\n                    Ok(_) => true,\n                    Err(err) => {\n                        logging::write(format!(\"encounter-history: refresh page generation failed: {err}\"));\n                        false\n                    }\n                };\n                if let Some(tx) = job.completion_tx.as_ref() {\n                    let alert = if refreshed {\n                        AlertEvent {\n                            kind: AlertKind::Ready,\n                            title: \"Benchmark complete\".into(),\n                            message: format!(\"Stop attacking. Result saved to Encounter History ({} seconds).\", record.context.benchmark_duration_ms / 1000),\n                        }\n                    } else {\n                        AlertEvent {\n                            kind: AlertKind::Error,\n                            title: \"Benchmark saved; history page needs refresh\".into(),\n                            message: \"Result was saved, but Encounter History HTML could not be refreshed. Check the log and reopen History after resolving the error.\".into(),\n                        }\n                    };\n                    let _ = tx.send(AppEvent::Alert(alert));\n                }",
        "archive and HTML outcome notification");
    fs::write(path, src).expect("write verified archive notifications");
    println!("cargo:rerun-if-changed=build/legacy/build_v1373_archive_ack.rs");
}
