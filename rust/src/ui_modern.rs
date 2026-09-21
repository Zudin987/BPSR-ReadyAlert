//! Native Win32 presentation and the two application-wide 96-DPI shape tokens.
//! Build-time transformations of the historical Pixel core remain assertion guarded.
include!(concat!(env!("OUT_DIR"), "/ui_pixel_core_shape_v1394.rs"));

#[link(name = "user32")]
extern "system" {
    fn GetFocus() -> windows_sys::Win32::Foundation::HWND;
    fn IsWindowEnabled(hwnd: windows_sys::Win32::Foundation::HWND) -> i32;
    fn GetDpiForWindow(hwnd: windows_sys::Win32::Foundation::HWND) -> u32;
    fn SetWindowRgn(hwnd: windows_sys::Win32::Foundation::HWND, region: windows_sys::Win32::Graphics::Gdi::HRGN, redraw: i32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, ellipse_width: i32, ellipse_height: i32) -> windows_sys::Win32::Graphics::Gdi::HRGN;
}

/// This is a logical-pixel design token, never a physical-pixel or UI-scale token.
pub const OVERLAY_FRAME_RADIUS_PX: i32 = SHAPE_RADIUS_CONTAINER;

fn physical_radius(dpi: u32, logical_radius: i32) -> i32 {
    ((logical_radius * dpi.max(96) as i32 + 48) / 96).max(1)
}

fn overlay_frame_diameter(width: i32, height: i32, dpi: u32) -> i32 {
    (physical_radius(dpi, OVERLAY_FRAME_RADIUS_PX) * 2)
        .min(width.max(1)).min(height.max(1))
}

/// The drawing helper already compensates for the GDI viewport transform used
/// by the meter: UI scale must not create a third design radius.
pub fn overlay_frame_radius_logical(_scale_percent: i32) -> i32 {
    SHAPE_RADIUS_CONTAINER
}

/// Clip the actual HWND, including the border and its input region, to the
/// four-logical-pixel shape. Windows owns the region after successful assignment.
pub unsafe fn apply_overlay_frame_region(hwnd: windows_sys::Win32::Foundation::HWND) {
    if hwnd.is_null() { return; }
    let mut bounds: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
    if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut bounds) == 0 { return; }
    let width = bounds.right.saturating_sub(bounds.left);
    let height = bounds.bottom.saturating_sub(bounds.top);
    if width <= 0 || height <= 0 { return; }
    let dpi = GetDpiForWindow(hwnd).max(96);
    let diameter = overlay_frame_diameter(width, height, dpi);
    let region = CreateRoundRectRgn(0, 0, width.saturating_add(1), height.saturating_add(1), diameter, diameter);
    if region.is_null() { return; }
    if SetWindowRgn(hwnd, region, 1) == 0 { DeleteObject(region); }
}

#[cfg(test)]
mod unified_overlay_frame_tests {
    use super::*;

    #[test]
    fn only_two_tokens_and_exact_dpi_conversion() {
        assert_eq!((SHAPE_RADIUS_CONTROL, SHAPE_RADIUS_CONTAINER), (2, 4));
        assert_eq!(OVERLAY_FRAME_RADIUS_PX, 4);
        assert_eq!(overlay_frame_diameter(650, 300, 96), 8);
        assert_eq!(overlay_frame_diameter(650, 300, 144), 12);
        assert_eq!(overlay_frame_diameter(650, 300, 192), 16);
        assert_eq!(overlay_frame_diameter(10, 6, 96), 6);
        for scale in [60, 70, 80, 90, 100, 125, 150, 175, 200] {
            assert_eq!(overlay_frame_radius_logical(scale), 4);
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/ui_pixel_controls_v1316.rs"));
include!(concat!(env!("OUT_DIR"), "/ui_pixel_dialog_shape_v1394.rs"));
include!("ui_pixel_phase2.rs");
include!("ui_pixel_chrome.rs");
include!(concat!(env!("OUT_DIR"), "/ui_pixel_qa_v1316.rs"));
include!("ui_pixel_menu.rs");
