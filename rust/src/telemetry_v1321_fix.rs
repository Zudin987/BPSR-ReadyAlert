use crate::{model::{AppEvent, MechanicSnapshot}, proto};
use std::sync::mpsc::{self, Receiver, Sender};

mod previous {
    include!("telemetry_v1320.rs");
}

pub use previous::{
    arm_benchmark, benchmark_status, benchmark_completion_pending, default_benchmark_seconds, request_manual_reset, BenchmarkStatus,
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

fn incoming_scene_id(body: &[u8]) -> Option<i32> {
    let info = proto::get_len_field(body, 1)?;
    let attrs = proto::get_len_field(info, 1)?;
    for attr in proto::len_fields(attrs, 2) {
        if proto::get_varint_field(attr, 1) != Some(0x155) { continue; }
        let data = proto::get_len_field(attr, 2)?;
        let mut offset = 0;
        return proto::read_varint(data, &mut offset)
            .and_then(|id| i32::try_from(id).ok())
            .filter(|id| *id > 0);
    }
    None
}

/// Merge future mechanic observations into the same stream as combat telemetry.
/// Incomplete or repeated scene notifications must not blank the tracker.
pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    base_mechanics: MechanicSnapshot,
    future: future_mechanics::Runtime,
    current_scene_id: i32,
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
            current_scene_id: 0,
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        let mut mechanics_changed = false;
        if service == proto::WORLD_SERVICE && method == proto::ENTER_SCENE_METHOD {
            if let Some(scene) = incoming_scene_id(body) {
                if scene != self.current_scene_id {
                    mechanics_changed |= !self.base_mechanics.rows.is_empty()
                        || !self.base_mechanics.tracked_attributes.is_empty()
                        || self.base_mechanics.food.is_some()
                        || self.base_mechanics.serum.is_some();
                    self.base_mechanics = MechanicSnapshot::default();
                    self.current_scene_id = scene;
                }
            }
        }

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
