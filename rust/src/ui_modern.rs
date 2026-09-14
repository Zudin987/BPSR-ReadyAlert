//! ReadyAlert's Pixel-inspired native presentation system.
//!
//! The application remains lightweight Rust + Win32.  These modules replace visual
//! primitives only; HWND ownership, layout, command IDs, shortcuts, telemetry and
//! feature behavior remain in their existing modules.
include!("ui_pixel_core.rs");

// Keep these two basic User32 queries on the same lightweight raw-FFI path already
// used by the Pixel core instead of widening windows-sys feature dependencies.
#[link(name = "user32")]
extern "system" {
    fn GetFocus() -> windows_sys::Win32::Foundation::HWND;
    fn IsWindowEnabled(hwnd: windows_sys::Win32::Foundation::HWND) -> i32;
}

// The build-stage copies contain only presentation fixes and are generated from the
// checked-in Pixel sources with strict replacement assertions. This keeps all native
// dropdowns and checkbox renderers on one verified implementation.
include!(concat!(env!("OUT_DIR"), "/ui_pixel_controls_v1316.rs"));
include!("ui_pixel_dialog.rs");
include!("ui_pixel_phase2.rs");
include!(concat!(env!("OUT_DIR"), "/ui_pixel_qa_v1316.rs"));
include!("ui_pixel_menu.rs");
