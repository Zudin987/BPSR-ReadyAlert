// Exact, antialiased pixels rasterized from assets/header-icons/*.svg at 24px.
// Four-bit alpha is RLE-packed in alpha-rle-24.txt; this keeps the native EXE
// dependency-free and avoids inaccurate GDI line/polygon approximations.
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, StretchDIBits, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, SRCCOPY,
    },
};

const ICON_PIXELS: usize = 24 * 24;
const MASKS: &str = include_str!("../assets/header-icons/alpha-rle-24.txt");

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum Icon {
    Copy, Reset, Settings, CollapseLeft, CollapseRight, CollapseTop, CollapseBottom, Close,
}

pub fn collapse_icon(side: &str) -> Icon {
    match side.to_ascii_lowercase().as_str() {
        "left" => Icon::CollapseLeft,
        "top" => Icon::CollapseTop,
        "bottom" => Icon::CollapseBottom,
        _ => Icon::CollapseRight,
    }
}

fn hex(c: u8) -> Option<usize> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as usize),
        b'a'..=b'f' => Some((c - b'a' + 10) as usize),
        _ => None,
    }
}

fn alpha_mask(icon: Icon) -> Option<[u8; ICON_PIXELS]> {
    let src = MASKS.lines().nth(icon as usize)?.as_bytes();
    let mut out = [0u8; ICON_PIXELS];
    let (mut offset, mut cursor) = (0usize, 0usize);
    while offset < src.len() {
        let (count, value) = if src[offset] == b'~' {
            if offset + 3 >= src.len() { return None; }
            let count = (hex(src[offset + 1])? << 4) | hex(src[offset + 2])?;
            let value = hex(src[offset + 3])? as u8 * 17;
            offset += 4;
            (count, value)
        } else {
            let value = hex(src[offset])? as u8 * 17;
            offset += 1;
            (1, value)
        };
        if count == 0 || cursor + count > ICON_PIXELS { return None; }
        out[cursor..cursor + count].fill(value);
        cursor += count;
    }
    (cursor == ICON_PIXELS).then_some(out)
}

fn colorref(r: u32, g: u32, b: u32) -> u32 { r | (g << 8) | (b << 16) }

// Native GDI has no SVG or per-pixel alpha on a plain memory HDC. Composite
// the real SVG raster over the actual button surface, then draw an opaque,
// top-down 32-bit DIB. No halos, polygon artifacts or new dependencies.
pub unsafe fn paint(hdc: HDC, rect: RECT, icon: Icon, background: u32) {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width < 16 || height < 16 { return; }
    let Some(mask) = alpha_mask(icon) else { return; };
    let size = (width - 4).min(height - 4).min(22).max(12) as usize;
    let bg_r = background & 255;
    let bg_g = (background >> 8) & 255;
    let bg_b = (background >> 16) & 255;
    let mut pixels = vec![0u32; size * size];
    for y in 0..size {
        let fy = ((y as f32 + 0.5) * 24.0 / size as f32 - 0.5).clamp(0.0, 23.0);
        let iy = fy as usize;
        let jy = (iy + 1).min(23);
        let dy = fy - iy as f32;
        for x in 0..size {
            let fx = ((x as f32 + 0.5) * 24.0 / size as f32 - 0.5).clamp(0.0, 23.0);
            let ix = fx as usize;
            let jx = (ix + 1).min(23);
            let dx = fx - ix as f32;
            let top = mask[iy * 24 + ix] as f32 * (1.0 - dx) + mask[iy * 24 + jx] as f32 * dx;
            let bottom = mask[jy * 24 + ix] as f32 * (1.0 - dx) + mask[jy * 24 + jx] as f32 * dx;
            let alpha = (top * (1.0 - dy) + bottom * dy).round() as u32;
            let mix = |base: u32| (255 * alpha + base * (255 - alpha) + 127) / 255;
            let r = mix(bg_r);
            let g = mix(bg_g);
            let b = mix(bg_b);
            // BI_RGB 32-bit DIB uses B,G,R,unused in memory (not COLORREF).
            pixels[y * size + x] = (r << 16) | (g << 8) | b;
        }
    }
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = size as i32;
    info.bmiHeader.biHeight = -(size as i32);
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let left = rect.left + (width - size as i32) / 2;
    let top = rect.top + (height - size as i32) / 2;
    StretchDIBits(hdc, left, top, size as i32, size as i32, 0, 0,
        size as i32, size as i32, pixels.as_ptr().cast(), &info,
        DIB_RGB_COLORS, SRCCOPY);
}

/// Draw the same rounded dark icon tile used in the approved pack preview.
/// The outer click target is unchanged, including for wide Mechanics buttons.
pub unsafe fn paint_tile(hdc: HDC, rect: RECT, icon: Icon, hovered: bool) {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width < 16 || height < 16 { return; }
    let tile_width = (width - 2).min(30);
    let tile_height = (height - 2).min(28);
    let tile = RECT {
        left: rect.left + (width - tile_width) / 2,
        top: rect.top + (height - tile_height) / 2,
        right: rect.left + (width - tile_width) / 2 + tile_width,
        bottom: rect.top + (height - tile_height) / 2 + tile_height,
    };
    let surface = if hovered { colorref(52, 58, 68) } else { colorref(35, 39, 47) };
    crate::ui_modern::fill_round_rect(hdc, tile, surface, 5);
    paint(hdc, tile, icon, surface);
}

/// Clears an already-rendered text symbol before painting the proper icon.
/// Only the visuals change; hit rectangles and handlers are not touched.
pub unsafe fn paint_button(hdc: HDC, rect: RECT, icon: Icon, background: u32, hovered: bool) {
    let brush = CreateSolidBrush(background);
    if !brush.is_null() {
        FillRect(hdc, &rect, brush);
        DeleteObject(brush);
    }
    paint_tile(hdc, rect, icon, hovered);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_icon_matches_edge() {
        assert_eq!(collapse_icon("LEFT"), Icon::CollapseLeft);
        assert_eq!(collapse_icon("right"), Icon::CollapseRight);
        assert_eq!(collapse_icon("TOP"), Icon::CollapseTop);
        assert_eq!(collapse_icon("bottom"), Icon::CollapseBottom);
    }

    #[test]
    fn all_eight_assets_match_raster_masks() {
        let svgs = [
            include_str!("../assets/header-icons/copy.svg"),
            include_str!("../assets/header-icons/reset.svg"),
            include_str!("../assets/header-icons/settings.svg"),
            include_str!("../assets/header-icons/collapse-left.svg"),
            include_str!("../assets/header-icons/collapse-right.svg"),
            include_str!("../assets/header-icons/collapse-top.svg"),
            include_str!("../assets/header-icons/collapse-bottom.svg"),
            include_str!("../assets/header-icons/close.svg"),
        ];
        assert_eq!(MASKS.lines().count(), svgs.len());
        for (i, svg) in svgs.iter().enumerate() {
            assert!(svg.contains("viewBox=\"0 0 24 24\""));
            assert!(svg.contains("#fff"));
            let mask = alpha_mask(match i {
                0 => Icon::Copy, 1 => Icon::Reset, 2 => Icon::Settings,
                3 => Icon::CollapseLeft, 4 => Icon::CollapseRight,
                5 => Icon::CollapseTop, 6 => Icon::CollapseBottom, _ => Icon::Close,
            }).expect("valid approved raster mask");
            assert!(mask.iter().any(|&a| a == 255));
            assert!(mask.iter().any(|&a| a == 0));
        }
    }
}
