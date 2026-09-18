// Npcap adapter recovery must not destroy an in-flight 300-second benchmark.
// Maintain one processor per capture session across adapter reopens, clear only
// stale TCP reassembly on reconnect, and service expiry even during backoff.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1371_identity_bootstrap.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, before: &str, after: &str, label: &str) {
    let count = src.matches(before).count();
    assert_eq!(count, 1, "capture reconnect {label}: expected one anchor, got {count}");
    *src = src.replacen(before, after, 1);
}
fn main() {
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path=out.join("capture_v185.rs");
    let mut src=fs::read_to_string(&path).expect("generated capture").replace("\r\n","\n");
    replace_once(&mut src,
        "        let mut failures = 0u32;\n        while !stop.load(Ordering::Relaxed) {",
        "        let mut failures = 0u32;\n        // Do not recreate TelemetryRuntime when a silent game stream, network\n        // adapter change or transient Npcap read error reopens capture.\n        let mut processor = CaptureProcessor::new(tx.clone(), identity.clone(), chat_runtime.clone());\n        while !stop.load(Ordering::Relaxed) {",
        "persistent processor");
    replace_once(&mut src,
        "            let mut filter = GamePacketFilter::new();\n            let mut processor = CaptureProcessor::new(tx.clone(), identity.clone(), chat_runtime.clone());",
        "            let mut filter = GamePacketFilter::new();\n            // Existing encounter/benchmark/identity remain valid, but partial\n            // TCP frames from an old capture handle are not safe to reuse.\n            processor.reset_flows();",
        "TCP-only reset on new handle");
    let old="sleep_interruptible(&stop, retry_delay(failures));";
    let count=src.matches(old).count();
    assert_eq!(count,3,"expected retry backoff in select/open/read recovery, got {count}");
    src=src.replace(old,"sleep_capture_retry(&stop, retry_delay(failures), &mut processor);");
    replace_once(&mut src,
        "fn sleep_interruptible(stop:&AtomicBool, duration:Duration) { let step=Duration::from_millis(100); let start=Instant::now(); while start.elapsed()<duration && !stop.load(Ordering::Relaxed) { thread::sleep(step); } }",
        "fn sleep_capture_retry(stop:&AtomicBool, duration:Duration, processor:&mut CaptureProcessor) {\n    let step=Duration::from_millis(100);\n    let start=Instant::now();\n    while start.elapsed()<duration && !stop.load(Ordering::Relaxed) {\n        if crate::telemetry::benchmark_completion_pending() {\n            processor.telemetry.handle_notify(0,0,&[]);\n        }\n        thread::sleep(step);\n    }\n}",
        "watchdog runs in adapter backoff");
    fs::write(path,src).expect("write reconnect-safe capture");
    println!("cargo:rerun-if-changed=build/legacy/build_v1372_capture_reconnect.rs");
}
