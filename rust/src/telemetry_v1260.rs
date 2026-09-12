use crate::{
    encounter_context::{self, EncounterContextSnapshot},
    encounter_store,
    logging,
    model::{AppEvent, DpsSnapshot},
};
use std::sync::mpsc::{self, Receiver, Sender};

mod previous {
    include!("telemetry_v1110.rs");
}

pub fn request_manual_reset() {
    previous::request_manual_reset();
}

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    last: Option<(DpsSnapshot, EncounterContextSnapshot)>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: previous::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            last: None,
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
        if let Some((previous, context)) = self.last.as_ref() {
            if encounter_store::encounter_rolled(previous, next) {
                self.archive(previous, context);
                self.last = None;
            }
        }
        if encounter_store::has_combat_data(next) {
            self.last = Some((next.clone(), encounter_context::snapshot()));
        }
    }

    fn archive(&self, snapshot: &DpsSnapshot, context: &EncounterContextSnapshot) {
        let root = encounter_store::default_root();
        match encounter_store::archive(&root, snapshot, context, encounter_store::DEFAULT_LIMIT) {
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
        // final empty DPS snapshot. Persist the latest non-empty segment here.
        if let Some((snapshot, context)) = self.last.take() {
            self.archive(&snapshot, &context);
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
