//! ReadyAlert's Pixel-inspired native presentation system.
//!
//! The application remains lightweight Rust + Win32.  These modules replace visual
//! primitives only; HWND ownership, layout, command IDs, shortcuts, telemetry and
//! feature behavior remain in their existing modules.
include!("ui_pixel_core.rs");
include!("ui_pixel_controls.rs");
include!("ui_pixel_dialog.rs");
