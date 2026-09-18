// The generator writes the benchmark wrapper into OUT_DIR so production code
// is never rewritten in the working tree by Cargo. Every patch is anchor-checked.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1369_encounter_lifecycle.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, before: &str, after: &str, name: &str) {
    let n = src.matches(before).count();
    assert_eq!(n, 1, "benchmark lifecycle {name}: expected one anchor, got {n}");
    *src = src.replacen(before, after, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let mut src = fs::read_to_string(root.join("src/telemetry_v1270.rs")).expect("benchmark wrapper").replace("\r\n", "\n");
    replace_once(&mut src, "    include!(\"telemetry_v1110.rs\");", "    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/telemetry_v1110.rs\"));", "wrapper source path");
    replace_once(&mut src, "    model::{AppEvent, DpsSnapshot},", "    model::{AlertEvent, AlertKind, AppEvent, DpsSnapshot},", "notification model imports");
    replace_once(&mut src, "struct ArchiveJob {\n    snapshot: DpsSnapshot,\n    context: EncounterContextSnapshot,\n}", "struct ArchiveJob {\n    snapshot: DpsSnapshot,\n    context: EncounterContextSnapshot,\n    completion_tx: Option<Sender<AppEvent>>,\n}", "archive acknowledgement");
    replace_once(&mut src, "pub fn default_benchmark_seconds() -> u32 { DEFAULT_BENCHMARK_SECONDS }", "pub fn default_benchmark_seconds() -> u32 { DEFAULT_BENCHMARK_SECONDS }\n\n/// Queried by the capture watchdog, including when Npcap returns no packets.\npub fn benchmark_completion_pending() -> bool { EXPIRED_BENCHMARK_ID.load(Ordering::Acquire) != 0 }", "pending completion predicate");
    replace_once(&mut src, "        if let Some((snapshot, context)) = self.last.take() {\n            self.queue_archive(snapshot, context);\n        }\n        self.benchmark = None;\n        set_status(None);\n        logging::write(format!(\"benchmark: completed id={expired}\"));", "        // The underlying runtime will emit an unthrottled final snapshot and\n        // an empty snapshot when it consumes this reset, on this same tick.\n        // Do not archive the older throttled snapshot yet.\n        previous::request_manual_reset();\n        self.benchmark = None;\n        set_status(None);\n        self.completing = Some(expired);\n        logging::write(format!(\"benchmark: timer expired id={expired}; finalizing\"));", "defer archive until final snapshot");
    replace_once(&mut src, "    benchmark: Option<BenchmarkRun>,\n}", "    benchmark: Option<BenchmarkRun>,\n    completing: Option<u64>,\n}", "completion state");
    replace_once(&mut src, "            benchmark: None,\n        }", "            benchmark: None,\n            completing: None,\n        }", "completion initialization");
    replace_once(&mut src, "        while let Ok(event) = self.inner_rx.try_recv() {\n            if let AppEvent::Dps(snapshot) = &event {\n                self.observe_snapshot(snapshot);\n            }\n            let _ = self.tx.send(event);\n        }", "        while let Ok(event) = self.inner_rx.try_recv() {\n            if let AppEvent::Dps(snapshot) = &event {\n                if let Some(expired) = self.completing {\n                    if encounter_store::has_combat_data(snapshot) {\n                        // This unthrottled snapshot carries the last observed hit.\n                        if let Some((_, context)) = self.last.as_mut() {\n                            self.last = Some((snapshot.clone(), context.clone()));\n                        }\n                    } else {\n                        if let Some((snapshot, context)) = self.last.take() {\n                            self.queue_completed_archive(snapshot, context);\n                        } else {\n                            logging::write(format!(\"benchmark: id={expired} had no combat data to save\"));\n                        }\n                        self.completing = None;\n                    }\n                } else {\n                    self.observe_snapshot(snapshot);\n                }\n            }\n            let _ = self.tx.send(event);\n        }", "final-snapshot ordering");
    replace_once(&mut src, "        if let Err(err) = tx.send(ArchiveJob { snapshot, context }) {", "        if let Err(err) = tx.send(ArchiveJob { snapshot, context, completion_tx: None }) {", "ordinary archive job");
    replace_once(&mut src, "}\n\nfn mark_benchmark_context(context: &mut EncounterContextSnapshot, config: &BenchmarkConfig)", "    fn queue_completed_archive(&self, snapshot: DpsSnapshot, context: EncounterContextSnapshot) {\n        if let Some(tx) = self.archive_tx.as_ref() {\n            if let Err(err) = tx.send(ArchiveJob { snapshot, context, completion_tx: Some(self.tx.clone()) }) {\n                logging::write(format!(\"benchmark: completion archive queue failed: {err}\"));\n                let _ = self.tx.send(AppEvent::Alert(AlertEvent { kind: AlertKind::Error, title: \"Benchmark save failed\".into(), message: \"Benchmark ended, but Encounter History could not queue the result.\".into() }));\n            }\n        } else {\n            let _ = self.tx.send(AppEvent::Alert(AlertEvent { kind: AlertKind::Error, title: \"Benchmark save failed\".into(), message: \"Benchmark ended, but the Encounter History writer is unavailable.\".into() }));\n        }\n    }\n}\n\nfn mark_benchmark_context(context: &mut EncounterContextSnapshot, config: &BenchmarkConfig)", "completion archive helper");
    // The timer publishes expiry only. The capture watchdog owns all state
    // mutation, flushing and reset, so the archive cannot lag behind an empty UI.
    let old = "                EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);\n                previous::request_manual_reset();\n                set_status(None);\n                // Reset the visible meter exactly at expiry instead of requiring\n                // one additional combat packet to make the UI catch up.\n                let _ = worker_tx.send(AppEvent::Dps(DpsSnapshot::default()));";
    replace_once(&mut src, old, "                EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);\n                // Capture watchdog will finalize even when no packets arrive.\n                let _ = worker_tx.send(AppEvent::CaptureStatus(\"Benchmark timer expired; saving final result…\".into()));", "timer expiry handoff");
    replace_once(&mut src, "            EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);\n            previous::request_manual_reset();\n            set_status(None);\n            let _ = ui_tx.send(AppEvent::Dps(DpsSnapshot::default()));", "            EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);\n            logging::write(\"benchmark: timer worker failed; completing on capture watchdog\");\n            let _ = ui_tx.send(AppEvent::CaptureStatus(\"Benchmark timer unavailable; finalizing result…\".into()));", "timer spawn failure");
    replace_once(&mut src, "                // Keep the already-open offline page refreshable without putting", "                if let Some(tx) = job.completion_tx.as_ref() {\n                    let _ = tx.send(AppEvent::Alert(AlertEvent {\n                        kind: AlertKind::Ready,\n                        title: \"Benchmark complete\".into(),\n                        message: format!(\"Stop attacking. Result saved to Encounter History ({} seconds).\", record.context.benchmark_duration_ms / 1000),\n                    }));\n                }\n                // Keep the already-open offline page refreshable without putting", "success acknowledgement");
    replace_once(&mut src, "            Ok(None) => {}\n            Err(err) => logging::write(format!(\"encounter-history: archive failed: {err}\")),", "            Ok(None) => {\n                if let Some(tx) = job.completion_tx.as_ref() {\n                    let _ = tx.send(AppEvent::Alert(AlertEvent { kind: AlertKind::Error, title: \"Benchmark save failed\".into(), message: \"No benchmark data was archived.\".into() }));\n                }\n            }\n            Err(err) => {\n                logging::write(format!(\"encounter-history: archive failed: {err}\"));\n                if let Some(tx) = job.completion_tx.as_ref() {\n                    let _ = tx.send(AppEvent::Alert(AlertEvent { kind: AlertKind::Error, title: \"Benchmark save failed\".into(), message: format!(\"Could not save benchmark to Encounter History: {err}\") }));\n                }\n            },", "archive failure acknowledgement");
    // Deterministic zero-packet expiry test: no sleep and no incoming notify.
    src.push_str(r#"
#[cfg(test)]
mod benchmark_expiry_v1370_tests {
    use super::*;
    #[test]
    fn three_hundred_second_expiry_can_finalize_without_a_game_packet() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        let config = BenchmarkConfig { id: NEXT_BENCHMARK_ID.fetch_add(1, Ordering::Relaxed), name: "300s replay".into(), seconds: 300 };
        let id = config.id;
        runtime.benchmark = Some(BenchmarkRun::Running(config.clone()));
        ACTIVE_BENCHMARK_ID.store(id, Ordering::Release);
        runtime.last = Some((DpsSnapshot { encounter_ms: 300_000, total_damage: 42, ..Default::default() }, { let mut context = EncounterContextSnapshot::default(); mark_benchmark_context(&mut context, &config); context }));
        EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);
        runtime.handle_notify(0, 0, &[]); // capture watchdog, not a game packet
        assert!(runtime.benchmark.is_none());
        assert!(runtime.completing.is_none());
        assert!(!benchmark_completion_pending());
        runtime.last = None;
    }
}
"#);
    fs::write(out.join("telemetry_v1270_lifecycle.rs"), src).expect("write generated benchmark wrapper");
    let mut capture = fs::read_to_string(out.join("capture_v185.rs")).expect("generated capture").replace("\r\n", "\n");
    replace_once(&mut capture, "                if last_watchdog.elapsed() >= Duration::from_secs(1) {\n                    last_watchdog = Instant::now();", "                if last_watchdog.elapsed() >= Duration::from_secs(1) {\n                    last_watchdog = Instant::now();\n                    // Delivers benchmark expiry even with zero subsequent packets.\n                    if crate::telemetry::benchmark_completion_pending() {\n                        processor.telemetry.handle_notify(0, 0, &[]);\n                    }", "packet-independent completion poll");
    fs::write(out.join("capture_v185.rs"), capture).expect("write capture watchdog");
    let mut win = fs::read_to_string(out.join("win_v182_fixed.rs")).expect("generated main window").replace("\r\n", "\n");
    replace_once(&mut win, "let enabled=alert.kind==AlertKind::Error||alert_enabled(&snapshot,alert.kind);", "let is_benchmark=alert.title==\"Benchmark complete\";let enabled=is_benchmark||alert.kind==AlertKind::Error||alert_enabled(&snapshot,alert.kind);", "benchmark alert enablement");
    replace_once(&mut win, "if snapshot.desktop_notification||alert.kind==AlertKind::Error{balloon(", "if is_benchmark||snapshot.desktop_notification||alert.kind==AlertKind::Error{balloon(", "benchmark completion balloon");
    fs::write(out.join("win_v182_fixed.rs"), win).expect("write benchmark notification UI");
    println!("cargo:rerun-if-changed=build/legacy/build_v1370_benchmark_lifecycle.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v1270.rs");
}
