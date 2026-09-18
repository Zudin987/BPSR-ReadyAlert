// Never assume a nearby player or party member is the owner. EnterScene or
// this client's SyncContainerData establishes the UID; subsequent names must
// belong to that verified UID. No guesses or cross-character name caching.
use crate::{model::{AppEvent, PlayerIdentity}, proto};
use std::sync::{mpsc::Sender, Arc, RwLock};
const SYNC_CONTAINER_DATA: u32 = 0x15;
const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;
const TEAM_INFO: u32 = 0x01;
const TEAM_MEMBER_INFO: u32 = 0x02;
const TEAM_JOIN: u32 = 0x03;

#[derive(Default)]
pub struct OwnerBootstrap { uid: i64, name: String }
impl OwnerBootstrap {
    pub fn observe(&mut self, service: u64, method: u32, body: &[u8], identity: &Arc<RwLock<Option<PlayerIdentity>>>, tx: &Sender<AppEvent>) {
        if service == proto::WORLD_SERVICE && method == proto::ENTER_SCENE_METHOD {
            if let Some(player) = proto::get_len_field(body, 1).and_then(|info|proto::get_len_field(info, 2)) {
                if let Some(uid) = proto::get_varint_field(player, 1).and_then(|raw| i64::try_from(raw).ok()).map(|uuid| uuid >> 16).filter(|uid| *uid > 0) {
                    self.observe_uid(uid, identity, tx);
                    if let Some(name) = entity_name(proto::get_len_field(player, 3)) { self.observe_name(name, identity, tx); }
                }
            }
        } else if service == proto::WORLD_SERVICE && method == SYNC_CONTAINER_DATA {
            if let Some(uid) = proto::get_len_field(body, 1).and_then(|data|proto::get_varint_field(data, 1))
                .and_then(|raw| i64::try_from(raw).ok()).filter(|uid| *uid > 0) { self.observe_uid(uid, identity, tx); }
        } else if service == proto::WORLD_SERVICE && self.uid > 0 && method == SYNC_NEAR_ENTITIES {
            for entity in proto::len_fields(body, 1) { self.observe_entity(entity, 3, identity, tx); }
        } else if service == proto::WORLD_SERVICE && self.uid > 0 && method == SYNC_NEAR_DELTA_INFO {
            for delta in proto::len_fields(body, 1) { self.observe_entity(delta, 2, identity, tx); }
        } else if service == proto::WORLD_SERVICE && self.uid > 0 && method == SYNC_TO_ME_DELTA_INFO {
            if let Some(delta) = proto::get_len_field(body, 1).and_then(|wrapper|proto::get_len_field(wrapper, 1)) {
                self.observe_entity(delta, 2, identity, tx);
            }
        } else if service == proto::TEAM_SERVICE && self.uid > 0 && matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {
            if let Some(name) = matching_team_name(body, self.uid, 0) { self.observe_name(name, identity, tx); }
        }
    }
    fn observe_uid(&mut self, uid: i64, identity: &Arc<RwLock<Option<PlayerIdentity>>>, tx: &Sender<AppEvent>) {
        if self.uid == uid { return; }
        self.uid = uid;
        if let Some(existing) = identity.read().ok().and_then(|owner|owner.as_ref().filter(|id| id.uid == uid).cloned()) {
            self.name = existing.name;
            return;
        }
        self.name.clear();
        // A character switch invalidates the previous owner's name until a
        // new owner-specific name is observed.
        if let Ok(mut owner) = identity.write() { *owner = None; }
        let _ = tx.send(AppEvent::CaptureStatus("Character UID found; waiting for character name sync".into()));
    }
    fn observe_name(&mut self, name: String, identity: &Arc<RwLock<Option<PlayerIdentity>>>, tx: &Sender<AppEvent>) {
        if self.uid <= 0 || name.is_empty() || self.name == name { return; }
        self.name = name.clone();
        let next = PlayerIdentity { name, uid: self.uid };
        if let Ok(mut owner) = identity.write() { *owner = Some(next.clone()); }
        let _ = tx.send(AppEvent::Identity(next));
    }
    fn observe_entity(&mut self, entity: &[u8], attrs_field: u32, identity: &Arc<RwLock<Option<PlayerIdentity>>>, tx: &Sender<AppEvent>) {
        let Some(uuid) = proto::get_varint_field(entity, 1).and_then(|raw| i64::try_from(raw).ok()) else { return; };
        // Actor UUID layout is (uid << 16) | (entity_kind << 6).
        if uuid >> 16 != self.uid || (uuid >> 6) & 0x1f != 10 { return; }
        if let Some(name) = entity_name(proto::get_len_field(entity, attrs_field)) {
            self.observe_name(name, identity, tx);
        }
    }
}
fn entity_name(attrs: Option<&[u8]>) -> Option<String> {
    for attr in proto::len_fields(attrs?, 2) {
        if proto::get_varint_field(attr, 1) != Some(1) { continue; }
        let bytes = proto::get_len_field(attr, 2)?;
        let mut pos = 0;
        let length = usize::try_from(proto::read_varint(bytes, &mut pos)?).ok()?;
        if length == 0 || length > 512 || pos.checked_add(length)? != bytes.len() { return None; }
        return safe_name(&bytes[pos..]);
    }
    None
}
fn safe_name(bytes: &[u8]) -> Option<String> {
    let value: String = std::str::from_utf8(bytes).ok()?.replace(['\r','\n','\0'], " ").trim().chars().take(128).collect();
    (!value.is_empty()).then_some(value)
}
fn matching_team_name(body: &[u8], uid: i64, depth: usize) -> Option<String> {
    if depth > 5 || body.len() > 1024 * 1024 { return None; }
    if proto::get_varint_field(body, 1).and_then(|raw| i64::try_from(raw).ok()) == Some(uid)
        && (proto::get_varint_field(body, 4).unwrap_or(0) > 0 || proto::get_varint_field(body, 9).unwrap_or(0) > 0) {
        if let Some(raw) = proto::get_len_field(body, 3) {
            if let Some(name) = safe_name(raw) { return Some(name); }
        }
    }
    for field in 1..=24 {
        for child in proto::len_fields(body, field) {
            if child.len() < body.len() {
                if let Some(name) = matching_team_name(child, uid, depth+1) { return Some(name); }
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    fn field(n: u64, mut value: u64) -> Vec<u8> {
        let mut buf = vec![(n << 3) as u8];
        while value >= 128 { buf.push((value as u8) | 128); value >>= 7; }
        buf.push(value as u8); buf
    }
    fn len(n: u64, bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![((n << 3)|2) as u8];
        out.push(bytes.len() as u8); out.extend_from_slice(bytes); out
    }
    #[test]
    fn nearby_player_cannot_be_assumed_to_be_owner() {
        let (tx, rx) = std::sync::mpsc::channel();
        let identity = Arc::new(RwLock::new(None));
        let mut resolver = OwnerBootstrap::default();
        resolver.observe(proto::WORLD_SERVICE, SYNC_NEAR_ENTITIES, &len(1, &field(1, (22<<16)|(10<<6))), &identity, &tx);
        assert!(identity.read().unwrap().is_none());
        assert!(rx.try_iter().next().is_none());
    }
    #[test]
    fn container_uid_and_matching_near_name_recover_mid_session_owner() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let identity = Arc::new(RwLock::new(None));
        let mut resolver = OwnerBootstrap::default();
        resolver.observe(proto::WORLD_SERVICE, SYNC_CONTAINER_DATA, &len(1, &field(1, 42)), &identity, &tx);
        assert!(identity.read().unwrap().is_none());
        let mut name = vec![4]; name.extend_from_slice(b"Mage");
        let attr = [field(1,1), len(2,&name)].concat();
        let entity = [field(1,(42<<16)|(10<<6)), len(3,&len(2,&attr))].concat();
        resolver.observe(proto::WORLD_SERVICE, SYNC_NEAR_ENTITIES, &len(1, &entity), &identity, &tx);
        let value = identity.read().unwrap().clone().unwrap();
        assert_eq!(value.uid, 42); assert_eq!(value.name, "Mage");
    }
    #[test]
    fn character_switch_invalidates_stale_identity() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let identity = Arc::new(RwLock::new(Some(PlayerIdentity { uid: 3, name: "Old".into() })));
        let mut resolver = OwnerBootstrap::default();
        resolver.observe(proto::WORLD_SERVICE, SYNC_CONTAINER_DATA, &len(1, &field(1, 4)), &identity, &tx);
        assert!(identity.read().unwrap().is_none());
    }
    #[test]
    fn capture_reconnect_preserves_same_verified_character() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let identity = Arc::new(RwLock::new(Some(PlayerIdentity { uid: 3, name: "Same".into() })));
        let mut resolver = OwnerBootstrap::default();
        resolver.observe(proto::WORLD_SERVICE, SYNC_CONTAINER_DATA, &len(1, &field(1, 3)), &identity, &tx);
        assert_eq!(identity.read().unwrap().as_ref().unwrap().name, "Same");
    }
}
