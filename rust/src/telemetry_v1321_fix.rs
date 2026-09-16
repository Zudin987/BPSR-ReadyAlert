use crate::{model::{AppEvent, MechanicSnapshot}, proto};
use std::sync::mpsc::{self, Receiver, Sender};

mod previous {
    include!("telemetry_v1320.rs");
}

pub use previous::{
    arm_benchmark, benchmark_status, default_benchmark_seconds, request_manual_reset, BenchmarkStatus,
};

mod future_mechanics {
    include!(concat!(env!("OUT_DIR"), "/future_mechanics_v1321_fixed.rs"));
}

/// Hide unresolved diagnostic placeholders while retaining mechanic labels that
/// only happen to begin with similar words (for example, a user-defined name).
fn unresolved_mechanic_label(label: &str) -> bool {
    let normalized = label.trim().to_ascii_lowercase();
    if normalized.is_empty() || matches!(normalized.as_str(), "unknown" | "unknown mech" | "unknown mechanic") {
        return true;
    }
    // The tracker prints unresolved numeric mechanic IDs in this format.
    // Do not use starts_with("unknown mech") or starts_with("boss mechanic #"):
    // either would hide a meaningful label such as "Boss mechanic #1 - dodge".
    for prefix in ["unknown mech #", "unknown mechanic #", "boss mechanic #"] {
        if let Some(id) = normalized.strip_prefix(prefix) {
            if !id.is_empty() && id.chars().all(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
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
            let merged = self.merged_mechanics();
            let _ = self.tx.send(AppEvent::Mechanics(merged));
        }
    }

    fn merged_mechanics(&mut self) -> MechanicSnapshot {
        let mut merged = self.base_mechanics.clone();
        merged.rows.retain(|row| !unresolved_mechanic_label(&row.label));
        for row in self.future.rows() {
            if unresolved_mechanic_label(&row.label) {
                continue;
            }
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

    #[test]
    fn unresolved_mechanic_placeholders_are_hidden_without_masking_real_labels() {
        for label in ["", "Unknown", "Unknown Mech", "Unknown Mechanic", "Boss mechanic #123456", "Unknown mech #42", "Unknown mechanic #9"] {
            assert!(unresolved_mechanic_label(label), "expected placeholder to be hidden: {label}");
        }
        for label in ["Correct portal", "Pizza danger - FAST", "Move away", "Skill ID 123456", "Unknown Mechanics Training", "Boss mechanic #1 - dodge", "Unknown mech #42 - move"] {
            assert!(!unresolved_mechanic_label(label), "expected real label to remain visible: {label}");
        }
    }
}
