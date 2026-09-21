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
    fn SetWindowRgn(hwnd: windows_sys::Win32::Foundation::HWND, region: windows_sys::Win32::Graphics::Gdi::HRGN, redraw: i32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, ellipse_width: i32, ellipse_height: i32) -> windows_sys::Win32::Graphics::Gdi::HRGN;
}

/// Outer window frame radius is a physical pixel specification, independent of
/// the DPS/tracker text scale. Do not change individual tab/button radii here.
pub const OVERLAY_FRAME_RADIUS_PX: i32 = 8;

fn overlay_frame_diameter(width: i32, height: i32) -> i32 {
    (OVERLAY_FRAME_RADIUS_PX * 2).min(width.max(1)).min(height.max(1))
}

pub fn overlay_frame_radius_logical(scale_percent: i32) -> i32 {
    let scale = scale_percent.max(1);
    ((OVERLAY_FRAME_RADIUS_PX * 100 + scale / 2) / scale).max(1)
}

/// A window region clips the *actual* layered HWND, not just its paint buffer:
/// background, toolbar and hit area all have the same rounded outer boundary.
/// SetWindowRgn owns the region on success; release it only if assignment fails.
/// Call at creation and on WM_SIZE, including collapsed/expanded transitions.
pub unsafe fn apply_overlay_frame_region(hwnd: windows_sys::Win32::Foundation::HWND) {
    if hwnd.is_null() { return; }
    let mut bounds: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
    if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut bounds) == 0 { return; }
    let width = bounds.right.saturating_sub(bounds.left);
    let height = bounds.bottom.saturating_sub(bounds.top);
    if width <= 0 || height <= 0 { return; }
    let diameter = overlay_frame_diameter(width, height);
    // Region right/bottom are exclusive. Include the outer edge without shrinking
    // the existing resizable client rectangle or its interactive controls.
    let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
    if region.is_null() { return; }
    if SetWindowRgn(hwnd, region, 1) == 0 { DeleteObject(region); }
}

#[cfg(test)]
mod unified_overlay_frame_tests {
    use super::*;

    #[test]
    fn all_three_overlays_share_exactly_eight_physical_pixels() {
        assert_eq!(OVERLAY_FRAME_RADIUS_PX, 8);
        assert_eq!(overlay_frame_diameter(650, 300), 16);
        assert_eq!(overlay_frame_diameter(420, 260), 16);
        assert_eq!(overlay_frame_diameter(25, 25), 16);
        assert_eq!(overlay_frame_diameter(10, 6), 6);
        for scale in [60, 70, 80, 90, 100, 125, 150, 175, 200] {
            let logical = overlay_frame_radius_logical(scale);
            let physical = (logical * scale + 50) / 100;
            assert!((physical - OVERLAY_FRAME_RADIUS_PX).abs() <= 1,
                "{scale}% must paint the same physical corner: {physical}px");
        }
    }
}

// The build-stage copies contain only presentation fixes and are generated from the
// checked-in Pixel sources with strict replacement assertions. This keeps all native
// dropdowns and checkbox renderers on one verified implementation.
include!(concat!(env!("OUT_DIR"), "/ui_pixel_controls_v1316.rs"));
include!("ui_pixel_dialog.rs");
include!("ui_pixel_phase2.rs");
include!(concat!(env!("OUT_DIR"), "/ui_pixel_qa_v1316.rs"));
include!("ui_pixel_menu.rs");