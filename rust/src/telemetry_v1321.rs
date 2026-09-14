use crate::{model::{AppEvent, MechanicSnapshot}, proto};
use std::sync::mpsc::{self, Receiver, Sender};

mod previous {
    include!("telemetry_v1320.rs");
}

pub use previous::{
    arm_benchmark, benchmark_status, default_benchmark_seconds, request_manual_reset, BenchmarkStatus,
};

mod future_mechanics {
    include!("future_mechanics.rs");
}

/// Final freeze wrapper. Existing combat telemetry remains authoritative; this
/// layer only merges generic, future-facing mechanic observations into the same
/// Mechanics stream consumed by the native overlay.
pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    base_mechanics: MechanicSnapshot,
    future: future_mechanics::Runtime,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: previous::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            base_mechanics: MechanicSnapshot::default(),
            future: future_mechanics::Runtime::new(),
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        let mut mechanics_changed = false;
        if service == proto::WORLD_SERVICE && method == proto::ENTER_SCENE_METHOD {
            mechanics_changed |= !self.base_mechanics.rows.is_empty()
                || !self.base_mechanics.tracked_attributes.is_empty()
                || self.base_mechanics.food.is_some()
                || self.base_mechanics.serum.is_some();
            self.base_mechanics = MechanicSnapshot::default();
        }

        // Let the proven telemetry implementation consume the packet first. In
        // particular, EnterScene clears its legacy tracker state before this
        // wrapper seeds fresh Attribute observations from that same packet.
        self.inner.handle_notify(service, method, body);
        while let Ok(event) = self.inner_rx.try_recv() {
            match event {
                AppEvent::Mechanics(snapshot) => {
                    self.base_mechanics = snapshot;
                    mechanics_changed = true;
                }
                other => {
                    let _ = self.tx.send(other);
                }
            }
        }

        mechanics_changed |= self.future.observe_notify(service, method, body);
        if mechanics_changed {
            let _ = self.tx.send(AppEvent::Mechanics(self.merged_mechanics()));
        }
    }

    fn merged_mechanics(&mut self) -> MechanicSnapshot {
        let mut merged = self.base_mechanics.clone();
        for row in self.future.rows() {
            merged.rows.retain(|existing| existing.key != row.key);
            merged.rows.push(row);
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_wrapper_keeps_previous_benchmark_exports() {
        assert!(default_benchmark_seconds() > 0);
        let _ = benchmark_status();
    }
}
