#[path = "ui_audit_v1302.rs"]
pub(crate) mod ui_audit_v1302;

pub mod benchmark_ui {
    include!(concat!(env!("OUT_DIR"), "/benchmark_ui_v1302.rs"));
}

#[path = "game_names_future.rs"]
pub(crate) mod game_names_v1300;

use std::sync::atomic::{AtomicBool, Ordering};

// A late-starting/reconnected packet capture cannot retroactively observe the
// fight's earlier damage or death signals. Mark the current attempt as partial
// until a trustworthy new scene or confirmed wipe boundary starts a new one.
static CAPTURE_PARTIAL: AtomicBool = AtomicBool::new(false);
// Archive the empty wipe boundary normally, but keep the last completed attempt
// visible until a new pull; this is consumed by the native DPS overlay once.
static WIPE_DISPLAY_HOLD: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_capture_partial() {
    CAPTURE_PARTIAL.store(true, Ordering::Release);
}
pub(crate) fn clear_capture_partial() {
    CAPTURE_PARTIAL.store(false, Ordering::Release);
}
pub(crate) fn capture_partial() -> bool {
    CAPTURE_PARTIAL.load(Ordering::Acquire)
}
pub(crate) fn mark_wipe_display_hold() {
    WIPE_DISPLAY_HOLD.store(true, Ordering::Release);
}
pub(crate) fn take_wipe_display_hold() -> bool {
    WIPE_DISPLAY_HOLD.swap(false, Ordering::AcqRel)
}

include!("telemetry_v1321_fix.rs");
