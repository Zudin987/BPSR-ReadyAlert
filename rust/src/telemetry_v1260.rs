#[path = "ui_audit_v1302.rs"]
pub(crate) mod ui_audit_v1302;

pub mod benchmark_ui {
    include!(concat!(env!("OUT_DIR"), "/benchmark_ui_v1302.rs"));
}

#[path = "game_names_future.rs"]
pub(crate) mod game_names_v1300;

include!("telemetry_v1321_fix.rs");
