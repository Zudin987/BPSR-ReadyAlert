use crate::{
    encounter_context::{self, EncounterContextSnapshot},
    encounter_store,
    logging,
    model::{AppEvent, DpsSnapshot},
};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

mod previous {
    include!("telemetry_v1110.rs");
}

pub fn request_manual_reset() {
    previous::request_manual_reset();
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
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        // Observe only protocol fields that are authoritative and useful to
        // offline history. The live DPS pipeline remains untouched.
        encounter_context::observe_notify(service, method, body);
        self.inner.handle_notify(service, method, body);
        while let Ok(event) = self.inner_rx.try_recv() {
            if let AppEvent::Dps(snapshot) = &event {
                self.observe_snapshot(snapshot);
            }
            let _ = self.tx.send(event);
        }
    }

    fn observe_snapshot(&mut self, next: &DpsSnapshot) {
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

fn archive_loop(rx: Receiver<ArchiveJob>) {
    let root = encounter_store::default_root();
    while let Ok(job) = rx.recv() {
        match encounter_store::archive(
            &root,
            &job.snapshot,
            &job.context,
            encounter_store::DEFAULT_LIMIT,
        ) {
            Ok(Some(record)) => logging::write(format!(
                "encounter-history: archived id={} scene={} target={} duration={}ms players={}",
                record.id,
                if record.context.scene_name.is_empty() { "unknown" } else { record.context.scene_name.as_str() },
                record.target_name,
                record.snapshot.encounter_ms,
                record.snapshot.rows.len(),
            )),
            Ok(None) => {}
            Err(err) => logging::write(format!("encounter-history: archive failed: {err}")),
        }
    }
}

impl Drop for TelemetryRuntime {
    fn drop(&mut self) {
        // Capture restarts and normal app shutdown otherwise have no guaranteed
        // final empty DPS snapshot. Queue the latest non-empty segment, then
        // close/join the writer so shutdown cannot lose the final archive.
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
    use crate::model::TargetSnapshot;

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
        // Avoid writing a test archive through Drop.
        runtime.last = None;
    }
}
