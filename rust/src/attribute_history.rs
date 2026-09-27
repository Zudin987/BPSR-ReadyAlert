//! Per-encounter character-sheet stat sampling. No damage coefficients or
//! season-specific build recommendations are involved.
//!
//! The encounter-history layer owns the Recorder: it must annotate a snapshot
//! BEFORE observe_snapshot clones it for persistence. The outer telemetry layer
//! only forwards the already-annotated event to the UI.
use crate::{
    feature_settings,
    model::{AppEvent, DpsRow, DpsSnapshot, EncounterAttributeSummary, TrackedAttribute},
};
use std::{collections::{BTreeMap, HashMap}, sync::mpsc::{self, Receiver, Sender}};

mod previous {
    include!("telemetry_v1321_fix.rs");
}
pub use previous::{
    arm_benchmark, benchmark_status, benchmark_completion_pending,
    default_benchmark_seconds, request_manual_reset, BenchmarkStatus,
};

#[derive(Default)]
struct Sample {
    label: String,
    initial: Option<i64>,
    final_value: Option<i64>,
    last_value: Option<i64>,
    last_at_ms: u64,
    weighted_sum: i128,
    observed_ms: u64,
}
impl Sample {
    fn new(id: i32, initial: Option<i64>) -> Self {
        Self {
            label: feature_settings::ATTRIBUTE_CATALOG.iter()
                .find(|(key, _)| *key == id)
                .map(|(_, label)| (*label).to_owned())
                .unwrap_or_else(|| format!("Attribute {id}")),
            initial,
            final_value: initial,
            last_value: initial,
            ..Default::default()
        }
    }
    fn advance(&mut self, ms: u64) {
        if ms < self.last_at_ms { return; }
        let elapsed = ms.saturating_sub(self.last_at_ms);
        if let Some(value) = self.last_value {
            self.weighted_sum = self.weighted_sum.saturating_add(
                i128::from(value).saturating_mul(i128::from(elapsed))
            );
            self.observed_ms = self.observed_ms.saturating_add(elapsed);
        }
        self.last_at_ms = ms;
    }
    fn observe(&mut self, value: Option<i64>, ms: u64) {
        self.advance(ms);
        self.last_value = value;
        if let Some(value) = value { self.final_value = Some(value); }
    }
    fn summary(&self, id: i32) -> EncounterAttributeSummary {
        EncounterAttributeSummary {
            attr_id: id,
            label: self.label.clone(),
            initial: self.initial,
            average: (self.observed_ms > 0).then(||
                self.weighted_sum as f64 / self.observed_ms as f64
            ).filter(|value| value.is_finite()),
            final_value: self.final_value,
            observed_ms: self.observed_ms,
        }
    }
}

#[derive(Default)]
struct Player { attrs: BTreeMap<i32, Sample> }
#[derive(Default)]
pub(crate) struct Recorder {
    players: HashMap<i64, Player>,
    /// Attribute values observed in the waiting/empty state of this encounter.
    precombat: HashMap<i64, BTreeMap<i32, i64>>,
    previous_totals: Option<(u64, i64, i64, i64)>,
}
fn observed_attributes(row: &DpsRow) -> BTreeMap<i32, i64> {
    row.attributes.iter()
        // The earlier freshness sanitizer zeroes stats absent from this scene.
        // Conservatively treat that sentinel as unavailable, NOT observed zero.
        .filter(|attr| attr.value > 0 && feature_settings::is_trackable_attr(attr.attr_id))
        .map(|attr: &TrackedAttribute| (attr.attr_id, attr.value))
        .collect()
}
fn empty(snapshot: &DpsSnapshot) -> bool {
    snapshot.encounter_ms == 0 && snapshot.total_damage == 0
        && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0
}
impl Recorder {
    pub(crate) fn update(&mut self, snapshot: &mut DpsSnapshot, partial: bool) {
        if empty(snapshot) {
            self.players.clear();
            self.previous_totals = None;
            self.precombat = snapshot.rows.iter()
                .filter(|row| row.uid > 0)
                .map(|row| (row.uid, observed_attributes(row)))
                .collect();
            return;
        }
        let now = snapshot.encounter_ms;
        if now == 0 { return; }
        let totals = (now, snapshot.total_damage, snapshot.total_healing, snapshot.total_damage_taken);
        let rolled = self.previous_totals.is_some_and(|old| {
            now.saturating_add(250) < old.0
                || totals.1 < old.1 || totals.2 < old.2 || totals.3 < old.3
        });
        if rolled {
            // Never carry previous-fight stats into a new attempt when the
            // underlying meter skipped an explicit empty snapshot.
            self.players.clear();
            self.precombat.clear();
        }
        self.previous_totals = Some(totals);
        for row in &mut snapshot.rows {
            if row.uid <= 0 { continue; }
            let observed = observed_attributes(row);
            let player = self.players.entry(row.uid).or_default();
            if player.attrs.is_empty() {
                if let Some(before) = self.precombat.get(&row.uid) {
                    if !partial {
                        for (&id, &value) in before {
                            player.attrs.insert(id, Sample::new(id, Some(value)));
                        }
                    }
                }
            }
            // Only an immediate, non-partial reading may substitute for a
            // missing pre-pull snapshot. Never invent an initial value for late capture.
            for (&id, &value) in &observed {
                player.attrs.entry(id).or_insert_with(|| {
                    let initial = (!partial && now <= 1_000).then_some(value);
                    let mut sample = Sample::new(id, initial);
                    if initial.is_none() { sample.last_at_ms = now; }
                    sample
                });
            }
            for (&id, sample) in &mut player.attrs {
                sample.observe(observed.get(&id).copied(), now);
            }
            row.encounter_attributes = player.attrs.iter()
                .map(|(&id, sample)| sample.summary(id))
                .collect();
        }
        // A vanished roster member cannot be assumed to retain previous stats.
        for (&uid, player) in &mut self.players {
            if !snapshot.rows.iter().any(|row| row.uid == uid) {
                for sample in player.attrs.values_mut() {
                    sample.last_value = None;
                    sample.last_at_ms = now;
                }
            }
        }
    }
}

/// The inner archive stage already decorates DPS events before cloning them.
/// Forward the exact same snapshot to the overlay; never sample twice.
pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
}
impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, rx) = mpsc::channel();
        Self { inner: previous::TelemetryRuntime::new(inner_tx), rx, tx }
    }
    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        self.inner.handle_notify(service, method, body);
        while let Ok(event) = self.rx.try_recv() {
            let _ = self.tx.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(ms: u64, damage: i64, stat: i64) -> DpsSnapshot {
        DpsSnapshot {
            encounter_ms: ms, total_damage: damage,
            rows: vec![DpsRow { uid: 7, damage,
                attributes: vec![TrackedAttribute { attr_id: feature_settings::ATTR_LUCKY,
                    label: "Luck".into(), value: stat }], ..Default::default() }],
            ..Default::default()
        }
    }
    fn attr(s: &DpsSnapshot) -> &EncounterAttributeSummary { &s.rows[0].encounter_attributes[0] }
    #[test]
    fn uses_time_weighting_not_number_of_packets() {
        let mut r = Recorder::default();
        let mut first = snapshot(100, 1, 2_000);
        r.update(&mut first, false);
        let mut many = snapshot(200, 2, 2_000);
        r.update(&mut many, false);
        r.update(&mut many, false);
        let mut last = snapshot(1_000, 3, 4_000);
        r.update(&mut last, false);
        assert_eq!(attr(&last).initial, Some(2_000));
        assert_eq!(attr(&last).average, Some(2_000.0));
        let mut final_snapshot = snapshot(2_000, 4, 4_000);
        r.update(&mut final_snapshot, false);
        assert_eq!(attr(&final_snapshot).average, Some(3_000.0));
        assert_eq!(attr(&final_snapshot).final_value, Some(4_000));
    }
    #[test]
    fn late_capture_does_not_invent_initial_or_unknown_zero() {
        let mut r = Recorder::default();
        let mut first = snapshot(5_000, 1, 2_000);
        r.update(&mut first, true);
        assert_eq!(attr(&first).initial, None);
        assert_eq!(attr(&first).average, None);
        let mut unknown = snapshot(6_000, 2, 0);
        r.update(&mut unknown, true);
        assert_eq!(attr(&unknown).final_value, Some(2_000));
        assert_eq!(attr(&unknown).observed_ms, 1_000);
        let mut known = snapshot(7_000, 3, 4_000);
        r.update(&mut known, true);
        assert_eq!(attr(&known).observed_ms, 1_000);
    }
    #[test]
    fn rollover_and_empty_reset_never_mix_attempts() {
        let mut r = Recorder::default();
        let mut old = snapshot(8_000, 10_000, 2_000);
        r.update(&mut old, false);
        let mut new = snapshot(500, 100, 3_000);
        r.update(&mut new, false);
        assert_eq!(attr(&new).initial, Some(3_000));
        assert_eq!(attr(&new).observed_ms, 500);
        r.update(&mut DpsSnapshot::default(), false);
        let mut next = snapshot(8_000, 10_000, 4_000);
        r.update(&mut next, false);
        assert_eq!(attr(&next).initial, None);
        assert_eq!(attr(&next).observed_ms, 0);
    }
}
