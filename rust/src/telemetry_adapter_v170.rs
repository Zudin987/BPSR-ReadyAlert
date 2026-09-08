use crate::{model::{AppEvent, DpsRow, DpsSnapshot}, proto};
use std::{collections::HashSet, sync::mpsc::{self, Receiver, Sender}};

#[path = "telemetry_v170.rs"]
mod inner;

pub struct TelemetryRuntime {
    inner: inner::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    team_uids: HashSet<i64>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: inner::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            team_uids: HashSet::new(),
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
                    if let Some(id) = proto::get_varint_field(req, 1).and_then(valid_uid) {
                        self.team_uids.remove(&id);
                    }
                }
            }
            0x0d => self.team_uids.clear(),
            _ => {}
        }
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
                    enrich_snapshot(&mut snapshot, &self.team_uids);
                    AppEvent::Dps(snapshot)
                }
                other => other,
            };
            let _ = self.tx.send(event);
        }
    }
}

fn valid_uid(raw: u64) -> Option<i64> {
    if raw == 0 || raw > i64::MAX as u64 { return None; }
    let uid = raw as i64;
    (uid > 0 && uid < 10_000_000_000_000).then_some(uid)
}

fn enrich_snapshot(snapshot: &mut DpsSnapshot, team: &HashSet<i64>) {
    for row in &mut snapshot.rows {
        for imagine in &mut row.imagines {
            if imagine.name.eq_ignore_ascii_case("Tina") || imagine.icon_key.eq_ignore_ascii_case("TI") {
                imagine.icon_key = "TN".into();
            }
            if imagine.name.eq_ignore_ascii_case("Brigand Leader") {
                imagine.icon_key = "BL".into();
            }
        }
        if row.subprofession_name.trim().is_empty() {
            if let Some((sub_id, profession_id, name)) = infer_spec(row) {
                row.subprofession_id = sub_id;
                row.subprofession_name = name.into();
                if row.profession_id == 0 {
                    row.profession_id = profession_id;
                }
            }
        }
    }

    for uid in team.iter().copied() {
        if snapshot.rows.iter().any(|r| r.uid == uid) { continue; }
        snapshot.rows.push(DpsRow {
            actor_uuid: canonical_player_uuid(uid),
            uid,
            name: format!("Player {uid}"),
            ..DpsRow::default()
        });
    }
    snapshot.rows.sort_by(|a, b| {
        b.damage
            .cmp(&a.damage)
            .then_with(|| b.healing.cmp(&a.healing))
            .then_with(|| a.name.cmp(&b.name))
    });
}

fn infer_spec(row: &DpsRow) -> Option<(i32, i32, &'static str)> {
    for skill in &row.skills {
        let result = match skill.skill_id {
            1714 | 1734 => (1001, 1, "Iaido"),
            1715 | 1738 | 179906 => (1002, 1, "Moonstrike"),
            120901 | 120902 => (2001, 2, "Icicle"),
            1241 => (2002, 2, "Frostbeam"),
            160102 | 2208181 | 2208172 => (3001, 3, "Formless Expertise"),
            1606 | 1621 | 1622 | 35104 => (3002, 3, "Crimson Expertise"),
            1405 | 1418 => (4001, 4, "Vanguard"),
            1419 => (4002, 4, "Skyward"),
            1518 | 1541 | 21402 => (5001, 5, "Smite"),
            20301 => (5002, 5, "Lifebind"),
            1941 | 2201240 => (9001, 9, "Earthfort"),
            1930 | 1931 | 1934 | 1935 => (9002, 9, "Block"),
            2292 | 1700820 | 1700825 | 1700827 => (11001, 11, "Wildpack"),
            220112 | 2203622 | 220106 => (11002, 11, "Falconry"),
            2405 | 2411 | 2206401 => (12001, 12, "Recovery"),
            2406 | 55412 | 55417 => (12002, 12, "Shield"),
            2321 | 2335 => (13001, 13, "Dissonance"),
            2301 | 2336 | 2361 | 55302 => (13002, 13, "Concerto"),
            _ => continue,
        };
        return Some(result);
    }
    None
}

fn canonical_player_uuid(uid: i64) -> i64 {
    (uid << 16) | (10_i64 << 6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SkillBreakdown;

    #[test]
    fn team_join_extracts_member_ids() {
        let body = [0x0a, 0x09, 0x12, 0x02, 0x08, 0x7b, 0x12, 0x03, 0x08, 0xc8, 0x03];
        let (tx, _rx) = mpsc::channel();
        let mut t = TelemetryRuntime::new(tx);
        t.update_team(0x03, &body);
        assert!(t.team_uids.contains(&123));
        assert!(t.team_uids.contains(&456));
    }

    #[test]
    fn smite_signature_sets_spec() {
        let row = DpsRow { skills: vec![SkillBreakdown { skill_id: 1518, ..Default::default() }], ..Default::default() };
        assert_eq!(infer_spec(&row), Some((5001, 5, "Smite")));
    }

    #[test]
    fn tina_badge_is_tn() {
        let mut s = DpsSnapshot { rows: vec![DpsRow { imagines: vec![crate::model::ImagineBadge { name: "Tina".into(), icon_key: "TI".into(), ..Default::default() }], ..Default::default() }], ..Default::default() };
        enrich_snapshot(&mut s, &HashSet::new());
        assert_eq!(s.rows[0].imagines[0].icon_key, "TN");
    }
}
