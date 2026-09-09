use crate::{
    model::{AppEvent, DpsSnapshot},
    proto,
};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver, Sender},
};

mod previous {
    include!("telemetry_v181.rs");
}

const ACTIVE_GAP_MS: u64 = 3_000;

pub fn request_manual_reset() {
    previous::request_manual_reset();
}

#[derive(Clone, Debug, Default)]
struct MetricActivity {
    last_value: i64,
    first_change_ms: Option<u64>,
    last_change_ms: u64,
    inactive_ms: u64,
}

impl MetricActivity {
    fn observe(&mut self, raw_value: i64, encounter_ms: u64) -> f64 {
        let value = raw_value.max(0);
        if value < self.last_value {
            *self = Self::default();
        }

        if value > self.last_value {
            match self.first_change_ms {
                None => {
                    self.first_change_ms = Some(encounter_ms);
                    self.last_change_ms = encounter_ms;
                }
                Some(_) => {
                    let gap = encounter_ms.saturating_sub(self.last_change_ms);
                    if gap > ACTIVE_GAP_MS {
                        self.inactive_ms = self
                            .inactive_ms
                            .saturating_add(gap.saturating_sub(ACTIVE_GAP_MS));
                    }
                    self.last_change_ms = encounter_ms;
                }
            }
        }
        self.last_value = value;

        let Some(first) = self.first_change_ms else { return 0.0; };
        if value <= 0 {
            return 0.0;
        }
        let active_ms = self
            .last_change_ms
            .saturating_sub(first)
            .saturating_sub(self.inactive_ms)
            .max(1_000);
        value as f64 / (active_ms as f64 / 1_000.0)
    }
}

#[derive(Clone, Debug, Default)]
struct RowActivity {
    damage: MetricActivity,
    healing: MetricActivity,
    taken: MetricActivity,
}

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    team_uids: HashSet<i64>,
    activity: HashMap<i64, RowActivity>,
    last_totals: Option<(u64, i64, i64, i64)>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: previous::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            team_uids: HashSet::new(),
            activity: HashMap::new(),
            last_totals: None,
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service == proto::TEAM_SERVICE {
            self.update_team(method, body);
        }
        self.inner.handle_notify(service, method, body);
        self.forward_inner_events();
    }

    fn update_team(&mut self, method: u32, body: &[u8]) {
        match method {
            0x02 => {
                // TEAM_MEMBER_INFO is a full/authoritative roster snapshot. The
                // previous additive behavior left stale members behind when a
                // leave packet was missed, which made Party-only meter filters
                // include people from an older roster.
                self.begin_full_team_snapshot();
                if let Some(req) = proto::get_len_field(body, 1) {
                    for field in [5_u32, 6_u32] {
                        for member in proto::len_fields(req, field) {
                            self.add_char_id(proto::get_varint_field(member, 1));
                        }
                    }
                }
            }
            0x03 => {
                if let Some(req) = proto::get_len_field(body, 1) {
                    for member in proto::len_fields(req, 2) {
                        self.add_char_id(proto::get_varint_field(member, 1));
                    }
                    for entry in proto::len_fields(req, 6) {
                        self.add_char_id(proto::get_varint_field(entry, 1));
                        if let Some(value) = proto::get_len_field(entry, 2) {
                            self.add_char_id(proto::get_varint_field(value, 1));
                        }
                    }
                }
            }
            0x04 => {
                if let Some(req) = proto::get_len_field(body, 1) {
                    if let Some(uid) = proto::get_varint_field(req, 1).and_then(valid_uid) {
                        self.team_uids.remove(&uid);
                    }
                }
            }
            0x0d => self.team_uids.clear(),
            _ => {}
        }
    }

    fn begin_full_team_snapshot(&mut self) {
        self.team_uids.clear();
    }

    fn add_char_id(&mut self, value: Option<u64>) {
        if let Some(uid) = value.and_then(valid_uid) {
            self.team_uids.insert(uid);
        }
    }

    fn forward_inner_events(&mut self) {
        while let Ok(event) = self.inner_rx.try_recv() {
            let event = match event {
                AppEvent::Dps(mut snapshot) => {
                    self.enrich_dps(&mut snapshot);
                    AppEvent::Dps(snapshot)
                }
                other => other,
            };
            let _ = self.tx.send(event);
        }
    }

    fn enrich_dps(&mut self, snapshot: &mut DpsSnapshot) {
        let totals = (
            snapshot.encounter_ms,
            snapshot.total_damage,
            snapshot.total_healing,
            snapshot.total_damage_taken,
        );
        if self
            .last_totals
            .is_some_and(|previous| encounter_changed(previous, totals))
        {
            self.activity.clear();
        }

        let empty = snapshot.encounter_ms == 0
            && snapshot.total_damage == 0
            && snapshot.total_healing == 0
            && snapshot.total_damage_taken == 0;
        if empty {
            self.activity.clear();
            self.last_totals = None;
        } else {
            self.last_totals = Some(totals);
        }

        for row in &mut snapshot.rows {
            row.is_party = row.is_local || self.team_uids.contains(&row.uid);
            let key = if row.uid > 0 { row.uid } else { row.actor_uuid };
            let active = self.activity.entry(key).or_default();
            row.active_dps = active.damage.observe(row.damage, snapshot.encounter_ms);
            row.active_hps = active.healing.observe(row.healing, snapshot.encounter_ms);
            row.active_dtps = active.taken.observe(row.damage_taken, snapshot.encounter_ms);
        }
    }
}

fn encounter_changed(previous: (u64, i64, i64, i64), next: (u64, i64, i64, i64)) -> bool {
    next.0 + 250 < previous.0 || next.1 < previous.1 || next.2 < previous.2 || next.3 < previous.3
}

fn valid_uid(raw: u64) -> Option<i64> {
    if raw == 0 || raw > i64::MAX as u64 {
        return None;
    }
    let uid = raw as i64;
    (uid > 0 && uid < 10_000_000_000_000).then_some(uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_rate_excludes_late_start() {
        let mut metric = MetricActivity::default();
        assert_eq!(metric.observe(100, 10_000), 100.0);
        assert_eq!(metric.observe(200, 11_000), 200.0);
        let encounter_rate = 200.0 / 11.0;
        assert!(metric.observe(200, 20_000) > encounter_rate * 5.0);
    }

    #[test]
    fn active_rate_caps_long_idle_gap() {
        let mut metric = MetricActivity::default();
        metric.observe(100, 1_000);
        metric.observe(200, 2_000);
        let after_idle = metric.observe(300, 12_000);
        // 11 seconds elapsed from first to latest activity, but seven seconds of
        // the ten-second idle gap are removed by the three-second activity cap.
        assert!((after_idle - 75.0).abs() < 0.01);
    }

    #[test]
    fn encounter_regression_resets_activity_generation() {
        assert!(encounter_changed((12_000, 5_000, 2_000, 1_000), (500, 50, 0, 0)));
        assert!(!encounter_changed((12_000, 5_000, 2_000, 1_000), (13_000, 6_000, 2_500, 1_200)));
    }

    #[test]
    fn full_team_snapshot_discards_stale_members() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.team_uids.extend([111, 222, 333]);
        runtime.begin_full_team_snapshot();
        runtime.team_uids.extend([222, 444]);
        assert_eq!(runtime.team_uids.len(), 2);
        assert!(runtime.team_uids.contains(&222));
        assert!(runtime.team_uids.contains(&444));
        assert!(!runtime.team_uids.contains(&111));
        assert!(!runtime.team_uids.contains(&333));
    }
}
