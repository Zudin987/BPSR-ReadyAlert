#[path = "ui_audit_v1302.rs"]
pub(crate) mod ui_audit_v1302;

pub mod benchmark_ui {
    include!(concat!(env!("OUT_DIR"), "/benchmark_ui_v1302.rs"));
}

#[path = "game_names_future.rs"]
pub(crate) mod game_names_v1300;

use std::sync::atomic::{AtomicBool, Ordering};

// A late-starting/reconnected capture cannot reconstruct earlier combat/death
// evidence. Keep its current encounter marked partial until a real boundary.
static CAPTURE_PARTIAL: AtomicBool = AtomicBool::new(false);
// History consumes the wipe boundary immediately; the overlay holds the previous
// attempt until a new pull. This flag is consumed once by its empty snapshot.
static WIPE_DISPLAY_HOLD: AtomicBool = AtomicBool::new(false);
// Generic mechanics must distinguish a real scene replacement from a repeated
// late EnterScene packet without duplicating telemetry's run correlation logic.
static SCENE_REPLACED: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_capture_partial() { CAPTURE_PARTIAL.store(true, Ordering::Release); }
pub(crate) fn clear_capture_partial() { CAPTURE_PARTIAL.store(false, Ordering::Release); }
pub(crate) fn capture_partial() -> bool { CAPTURE_PARTIAL.load(Ordering::Acquire) }
pub(crate) fn mark_wipe_display_hold() { WIPE_DISPLAY_HOLD.store(true, Ordering::Release); }
pub(crate) fn take_wipe_display_hold() -> bool { WIPE_DISPLAY_HOLD.swap(false, Ordering::AcqRel) }
pub(crate) fn mark_scene_replaced() { SCENE_REPLACED.store(true, Ordering::Release); }
pub(crate) fn take_scene_replaced() -> bool { SCENE_REPLACED.swap(false, Ordering::AcqRel) }

#[path = "attribute_history.rs"]
mod attribute_history;
pub use attribute_history::*;
