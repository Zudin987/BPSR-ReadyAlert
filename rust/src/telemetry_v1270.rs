use crate::{
    encounter_archive,
    encounter_context::{self, EncounterContextSnapshot},
    encounter_store,
    logging,
    model::{AppEvent, DpsSnapshot},
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
        Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

mod previous {
    include!("telemetry_v1110.rs");
}

const DEFAULT_BENCHMARK_SECONDS: u32 = 100;
const MAX_BENCHMARK_SECONDS: u32 = 3_600;
const MAX_BENCHMARK_NAME_CHARS: usize = 64;

#[derive(Clone, Debug)]
struct BenchmarkConfig {
    id: u64,
    name: String,
    seconds: u32,
}

#[derive(Clone, Debug)]
enum BenchmarkCommand {
    Arm(BenchmarkConfig),
    Cancel,
}

#[derive(Clone, Debug)]
enum BenchmarkRun {
    Armed(BenchmarkConfig),
    Running(BenchmarkConfig),
}

#[derive(Clone, Debug, Default)]
pub struct BenchmarkStatus {
    pub armed: bool,
    pub running: bool,
    pub name: String,
    pub seconds: u32,
}

static NEXT_BENCHMARK_ID: AtomicU64 = AtomicU64::new(1);
static ACTIVE_BENCHMARK_ID: AtomicU64 = AtomicU64::new(0);
static EXPIRED_BENCHMARK_ID: AtomicU64 = AtomicU64::new(0);
static BENCHMARK_COMMAND: OnceLock<Mutex<Option<BenchmarkCommand>>> = OnceLock::new();
static BENCHMARK_STATUS: OnceLock<Mutex<BenchmarkStatus>> = OnceLock::new();

fn command_slot() -> &'static Mutex<Option<BenchmarkCommand>> {
    BENCHMARK_COMMAND.get_or_init(|| Mutex::new(None))
}

fn status_slot() -> &'static Mutex<BenchmarkStatus> {
    BENCHMARK_STATUS.get_or_init(|| Mutex::new(BenchmarkStatus::default()))
}

fn set_status(run: Option<&BenchmarkRun>) {
    let Ok(mut status) = status_slot().lock() else { return; };
    *status = match run {
        Some(BenchmarkRun::Armed(config)) => BenchmarkStatus {
            armed: true,
            running: false,
            name: config.name.clone(),
            seconds: config.seconds,
        },
        Some(BenchmarkRun::Running(config)) => BenchmarkStatus {
            armed: false,
            running: true,
            name: config.name.clone(),
            seconds: config.seconds,
        },
        None => BenchmarkStatus::default(),
    };
}

pub fn benchmark_status() -> BenchmarkStatus {
    status_slot().lock().map(|status| status.clone()).unwrap_or_default()
}

pub fn arm_benchmark(name: String, seconds: u32) {
    let id = NEXT_BENCHMARK_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let config = BenchmarkConfig {
        id,
        name: normalize_benchmark_name(&name),
        seconds: normalize_benchmark_seconds(seconds),
    };
    ACTIVE_BENCHMARK_ID.store(id, Ordering::Release);
    EXPIRED_BENCHMARK_ID.store(0, Ordering::Release);
    if let Ok(mut slot) = command_slot().lock() {
        *slot = Some(BenchmarkCommand::Arm(config.clone()));
    }
    set_status(Some(&BenchmarkRun::Armed(config)));
    // Start from a clean meter, but the benchmark clock remains armed until the
    // local player produces an outgoing combat contribution.
    previous::request_manual_reset();
}

pub fn request_manual_reset() {
    ACTIVE_BENCHMARK_ID.store(0, Ordering::Release);
    EXPIRED_BENCHMARK_ID.store(0, Ordering::Release);
    if let Ok(mut slot) = command_slot().lock() {
        *slot = Some(BenchmarkCommand::Cancel);
    }
    set_status(None);
    previous::request_manual_reset();
}

pub fn default_benchmark_seconds() -> u32 { DEFAULT_BENCHMARK_SECONDS }

fn normalize_benchmark_seconds(seconds: u32) -> u32 {
    seconds.clamp(1, MAX_BENCHMARK_SECONDS)
}

fn normalize_benchmark_name(name: &str) -> String {
    let cleaned: String = name
        .replace(['\r', '\n', '\0'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_BENCHMARK_NAME_CHARS)
        .collect();
    if cleaned.is_empty() { "Unnamed".into() } else { cleaned }
}

fn has_benchmark_activity(snapshot: &DpsSnapshot) -> bool {
    if snapshot.encounter_ms == 0 {
        return false;
    }
    snapshot
        .rows
        .iter()
        .find(|row| row.is_local)
        .is_some_and(|row| row.damage > 0 || row.healing > 0)
}

#[derive(Debug)]
struct ArchiveJob {
    snapshot: DpsSnapshot,
    context: EncounterContextSnapshot,
}

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    last: Option<(DpsSnapshot, EncounterContextSnapshot)>,
    archive_tx: Option<Sender<ArchiveJob>>,
    archive_worker: Option<JoinHandle<()>>,
    benchmark: Option<BenchmarkRun>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        let (archive_tx, archive_rx) = mpsc::channel();
        let archive_worker = match thread::Builder::new()
            .name("readyalert-encounter-archive".into())
            .spawn(move || archive_loop(archive_rx))
        {
            Ok(worker) => Some(worker),
            Err(err) => {
                logging::write(format!("encounter-history: archive worker unavailable: {err}"));
                None
            }
        };
        let archive_tx = archive_worker.as_ref().map(|_| archive_tx);
        Self {
            inner: previous::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            last: None,
            archive_tx,
            archive_worker,
            benchmark: None,
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        self.consume_benchmark_command();
        self.finish_expired_benchmark();
        encounter_context::observe_notify(service, method, body);
        self.inner.handle_notify(service, method, body);
        while let Ok(event) = self.inner_rx.try_recv() {
            if let AppEvent::Dps(snapshot) = &event {
                self.observe_snapshot(snapshot);
            }
            let _ = self.tx.send(event);
        }
    }

    fn consume_benchmark_command(&mut self) {
        let command = command_slot().lock().ok().and_then(|mut slot| slot.take());
        match command {
            Some(BenchmarkCommand::Arm(config)) => {
                if let Some((snapshot, context)) = self.last.take() {
                    self.queue_archive(snapshot, context);
                }
                self.benchmark = Some(BenchmarkRun::Armed(config));
                set_status(self.benchmark.as_ref());
            }
            Some(BenchmarkCommand::Cancel) => {
                ACTIVE_BENCHMARK_ID.store(0, Ordering::Release);
                self.benchmark = None;
                set_status(None);
            }
            None => {}
        }
    }

    fn finish_expired_benchmark(&mut self) {
        let expired = EXPIRED_BENCHMARK_ID.swap(0, Ordering::AcqRel);
        if expired == 0 {
            return;
        }
        let matches = matches!(
            self.benchmark.as_ref(),
            Some(BenchmarkRun::Running(config)) if config.id == expired
        );
        if !matches {
            return;
        }
        if let Some((snapshot, context)) = self.last.take() {
            self.queue_archive(snapshot, context);
        }
        self.benchmark = None;
        set_status(None);
        logging::write(format!("benchmark: completed id={expired}"));
    }

    fn observe_snapshot(&mut self, next: &DpsSnapshot) {
        match self.benchmark.clone() {
            Some(BenchmarkRun::Armed(config)) => {
                if !has_benchmark_activity(next) {
                    return;
                }
                let mut context = encounter_context::snapshot();
                mark_benchmark_context(&mut context, &config);
                self.last = Some((next.clone(), context));
                self.benchmark = Some(BenchmarkRun::Running(config.clone()));
                set_status(self.benchmark.as_ref());
                start_benchmark_timer(config, self.tx.clone());
                return;
            }
            Some(BenchmarkRun::Running(config)) => {
                let rolled = self.last.as_ref()
                    .is_some_and(|(previous, _)| encounter_store::encounter_rolled(previous, next));
                if rolled {
                    if let Some((snapshot, context)) = self.last.take() {
                        self.queue_archive(snapshot, context);
                    }
                    let _ = ACTIVE_BENCHMARK_ID.compare_exchange(
                        config.id,
                        0,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                    self.benchmark = None;
                    set_status(None);
                    if encounter_store::has_combat_data(next) {
                        self.last = Some((next.clone(), encounter_context::snapshot()));
                    }
                    return;
                }
                if encounter_store::has_combat_data(next) {
                    let mut context = encounter_context::snapshot();
                    mark_benchmark_context(&mut context, &config);
                    self.last = Some((next.clone(), context));
                }
                return;
            }
            None => {}
        }

        let rolled = self.last.as_ref()
            .is_some_and(|(previous, _)| encounter_store::encounter_rolled(previous, next));
        if rolled {
            if let Some((snapshot, context)) = self.last.take() {
                self.queue_archive(snapshot, context);
            }
        }
        if encounter_store::has_combat_data(next) {
            self.last = Some((next.clone(), encounter_context::snapshot()));
        }
    }

    fn queue_archive(&self, snapshot: DpsSnapshot, context: EncounterContextSnapshot) {
        let Some(tx) = self.archive_tx.as_ref() else {
            logging::write("encounter-history: archive skipped because writer thread is unavailable");
            return;
        };
        if let Err(err) = tx.send(ArchiveJob { snapshot, context }) {
            logging::write(format!("encounter-history: archive queue failed: {err}"));
        }
    }
}

fn mark_benchmark_context(context: &mut EncounterContextSnapshot, config: &BenchmarkConfig) {
    context.benchmark_name = config.name.clone();
    context.benchmark_duration_ms = u64::from(config.seconds).saturating_mul(1_000);
}

fn start_benchmark_timer(config: BenchmarkConfig, ui_tx: Sender<AppEvent>) {
    let id = config.id;
    let seconds = config.seconds;
    logging::write(format!(
        "benchmark: started id={} name={} duration={}s",
        id, config.name, seconds
    ));
    let worker_tx = ui_tx.clone();
    let result = thread::Builder::new()
        .name("readyalert-benchmark-timer".into())
        .spawn(move || {
            thread::sleep(Duration::from_secs(u64::from(seconds)));
            // A stale timer must never reset a newer benchmark. Winning this CAS
            // makes this generation the sole owner of expiry/reset.
            if ACTIVE_BENCHMARK_ID
                .compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);
                previous::request_manual_reset();
                set_status(None);
                // Reset the visible meter exactly at expiry instead of requiring
                // one additional combat packet to make the UI catch up.
                let _ = worker_tx.send(AppEvent::Dps(DpsSnapshot::default()));
            }
        });
    if let Err(err) = result {
        logging::write(format!("benchmark: timer thread unavailable: {err}"));
        if ACTIVE_BENCHMARK_ID
            .compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            EXPIRED_BENCHMARK_ID.store(id, Ordering::Release);
            previous::request_manual_reset();
            set_status(None);
            let _ = ui_tx.send(AppEvent::Dps(DpsSnapshot::default()));
        }
    }
}

fn archive_loop(rx: Receiver<ArchiveJob>) {
    let root = encounter_store::default_root();
    while let Ok(job) = rx.recv() {
        match encounter_store::archive(
            &root,
            &job.snapshot,
            &job.context,
            encounter_store::DEFAULT_LIMIT,
        ) {
            Ok(Some(record)) => {
                logging::write(format!(
                    "encounter-history: archived id={} scene={} target={} duration={}ms players={}",
                    record.id,
                    if record.context.scene_name.is_empty() { "unknown" } else { record.context.scene_name.as_str() },
                    record.target_name,
                    record.snapshot.encounter_ms,
                    record.snapshot.rows.len(),
                ));
                // Keep the already-open offline page refreshable without putting
                // decompression/HTML generation on the capture thread.
                if let Err(err) = encounter_archive::generate(&root) {
                    logging::write(format!("encounter-history: refresh page generation failed: {err}"));
                }
            }
            Ok(None) => {}
            Err(err) => logging::write(format!("encounter-history: archive failed: {err}")),
        }
    }
}

impl Drop for TelemetryRuntime {
    fn drop(&mut self) {
        if let Some(run) = self.benchmark.as_ref() {
            let id = match run {
                BenchmarkRun::Armed(config) | BenchmarkRun::Running(config) => config.id,
            };
            let _ = ACTIVE_BENCHMARK_ID.compare_exchange(
                id,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        if let Some((snapshot, context)) = self.last.take() {
            self.queue_archive(snapshot, context);
        }
        self.archive_tx.take();
        if let Some(worker) = self.archive_worker.take() {
            if worker.join().is_err() {
                logging::write("encounter-history: archive worker panicked during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DpsRow, TargetSnapshot};

    #[test]
    fn benchmark_defaults_and_labels_are_stable() {
        assert_eq!(default_benchmark_seconds(), 100);
        assert_eq!(normalize_benchmark_seconds(0), 1);
        assert_eq!(normalize_benchmark_seconds(9_999), 3_600);
        assert_eq!(normalize_benchmark_name("  frost\nmage  "), "frost mage");
        assert_eq!(normalize_benchmark_name("   "), "Unnamed");
    }

    #[test]
    fn benchmark_waits_for_local_outgoing_activity() {
        let idle = DpsSnapshot {
            encounter_ms: 1_000,
            total_damage_taken: 10_000,
            rows: vec![DpsRow { is_local: true, damage_taken: 10_000, ..Default::default() }],
            ..Default::default()
        };
        assert!(!has_benchmark_activity(&idle));
        let active = DpsSnapshot {
            encounter_ms: 1_000,
            total_damage: 5_000,
            rows: vec![DpsRow { is_local: true, damage: 5_000, ..Default::default() }],
            ..Default::default()
        };
        assert!(has_benchmark_activity(&active));
    }

    #[test]
    fn benchmark_context_is_explicit_and_version_compatible() {
        let config = BenchmarkConfig { id: 7, name: "Frostmage".into(), seconds: 100 };
        let mut context = EncounterContextSnapshot::default();
        mark_benchmark_context(&mut context, &config);
        assert_eq!(context.benchmark_name, "Frostmage");
        assert_eq!(context.benchmark_duration_ms, 100_000);
    }

    #[test]
    fn wrapper_keeps_latest_snapshot_for_shutdown_flush() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        let snapshot = DpsSnapshot {
            encounter_ms: 1_000,
            total_damage: 10_000,
            target: Some(TargetSnapshot { name: "Boss".into(), ..Default::default() }),
            ..Default::default()
        };
        runtime.observe_snapshot(&snapshot);
        assert!(runtime.last.is_some());
        runtime.last = None;
    }
}
