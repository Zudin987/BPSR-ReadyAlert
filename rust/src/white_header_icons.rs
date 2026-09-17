// Native Win32 GDI redraws of assets/header-icons/*.svg (24x24 viewBox).
// The overlays cannot render SVG directly; this needs no new runtime dependency.
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, Ellipse, FillRect, GetStockObject,
        LineTo, MoveToEx, Polygon, SelectObject, HDC, NULL_BRUSH, NULL_PEN, PS_SOLID,
    },
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

struct Canvas { x: i32, y: i32, size: i32 }
impl Canvas {
    fn pt(&self, x: f32, y: f32) -> POINT {
        POINT {
            x: self.x + (x * self.size as f32 / 24.0).round() as i32,
            y: self.y + (y * self.size as f32 / 24.0).round() as i32,
        }
    }
    unsafe fn stroke(&self, hdc: HDC, points: &[(f32, f32)]) {
        if let Some((&(x, y), rest)) = points.split_first() {
            let start = self.pt(x, y);
            MoveToEx(hdc, start.x, start.y, std::ptr::null_mut());
            for &(x, y) in rest {
                let p = self.pt(x, y);
                LineTo(hdc, p.x, p.y);
            }
        }
    }
    unsafe fn polygon(&self, hdc: HDC, points: &[(f32, f32)]) {
        let coords: Vec<POINT> = points.iter().map(|&(x, y)| self.pt(x, y)).collect();
        Polygon(hdc, coords.as_ptr(), coords.len() as i32);
    }
}

/// Paint one transparent white icon inside an existing button's hit rectangle.
pub unsafe fn paint(hdc: HDC, rect: RECT, icon: Icon, background: u32) {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width < 12 || height < 12 { return; }
    let size = (width - 6).min(height - 6).min(22).max(12);
    let canvas = Canvas {
        x: rect.left + (width - size) / 2,
        y: rect.top + (height - size) / 2,
        size,
    };
    let white = 0x00ff_ffff;
    let pen = CreatePen(PS_SOLID, (size / 11).max(2), white);
    if pen.is_null() { return; }
    let brush = CreateSolidBrush(white);
    if brush.is_null() { DeleteObject(pen); return; }
    let old_pen = SelectObject(hdc, pen);
    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    match icon {
        Icon::Copy => {
            canvas.stroke(hdc, &[(7.0,17.5),(4.5,17.5),(2.5,15.5),(2.5,4.5),(4.5,2.5),(13.5,2.5),(15.5,4.5),(15.5,7.0)]);
            canvas.stroke(hdc, &[(9.0,7.0),(16.7,7.0),(21.0,11.3),(21.0,19.5),(19.0,21.5),(9.0,21.5),(7.0,19.5),(7.0,9.0),(9.0,7.0)]);
            canvas.stroke(hdc, &[(16.7,7.0),(16.7,11.3),(21.0,11.3)]);
            canvas.stroke(hdc, &[(10.5,14.7),(17.2,14.7)]);
            canvas.stroke(hdc, &[(10.5,18.0),(17.2,18.0)]);
        }
        Icon::Reset => {
            canvas.stroke(hdc, &[(19.2,8.2),(17.3,5.9),(14.0,4.0),(10.1,3.7),(6.4,5.3),(4.1,8.5),(3.7,12.5),(5.3,16.3),(8.4,19.1),(12.6,20.3),(16.3,19.0),(19.0,16.8),(20.1,15.3)]);
            SelectObject(hdc, brush);
            canvas.polygon(hdc, &[(20.7,4.3),(20.7,10.5),(14.6,9.2)]);
            SelectObject(hdc, GetStockObject(NULL_BRUSH));
        }
        Icon::Settings => {
            const GEAR: &[(f32,f32)] = &[
                (9.03,4.84),(9.93,4.27),(9.98,1.8),(14.02,1.8),
                (14.07,4.27),(14.97,4.84),(16.0,5.07),(17.79,3.36),
                (20.64,6.21),(18.93,8.0),(19.16,9.03),(19.73,9.93),
                (22.2,9.98),(22.2,14.02),(19.73,14.07),(19.16,14.97),
                (18.93,16.0),(20.64,17.79),(17.79,20.64),(16.0,18.93),
                (14.97,19.16),(14.07,19.73),(14.02,22.2),(9.98,22.2),
                (9.93,19.73),(9.03,19.16),(8.0,18.93),(6.21,20.64),
                (3.36,17.79),(5.07,16.0),(4.84,14.97),(4.27,14.07),
                (1.8,14.02),(1.8,9.98),(4.27,9.93),(4.84,9.03),
                (5.07,8.0),(3.36,6.21),(6.21,3.36),(8.0,5.07),
            ];
            SelectObject(hdc, brush);
            canvas.polygon(hdc, GEAR);
            let hole = CreateSolidBrush(background);
            if !hole.is_null() {
                SelectObject(hdc, GetStockObject(NULL_PEN));
                SelectObject(hdc, hole);
                let a=canvas.pt(8.45,8.45);
                let b=canvas.pt(15.55,15.55);
                Ellipse(hdc,a.x,a.y,b.x,b.y);
                SelectObject(hdc, brush);
                SelectObject(hdc, pen);
                DeleteObject(hole);
            }
        }
        Icon::CollapseLeft => {
            canvas.stroke(hdc,&[(5.4,3.2),(5.4,20.8)]);
            canvas.stroke(hdc,&[(17.6,6.3),(11.8,12.0),(17.6,17.7)]);
        }
        Icon::CollapseRight => {
            canvas.stroke(hdc,&[(18.6,3.2),(18.6,20.8)]);
            canvas.stroke(hdc,&[(6.4,6.3),(12.2,12.0),(6.4,17.7)]);
        }
        Icon::CollapseTop => {
            canvas.stroke(hdc,&[(3.2,5.4),(20.8,5.4)]);
            canvas.stroke(hdc,&[(6.3,17.6),(12.0,11.8),(17.7,17.6)]);
        }
        Icon::CollapseBottom => {
            canvas.stroke(hdc,&[(3.2,18.6),(20.8,18.6)]);
            canvas.stroke(hdc,&[(6.3,6.4),(12.0,12.2),(17.7,6.4)]);
        }
        Icon::Close => {
            canvas.stroke(hdc,&[(5.0,5.0),(19.0,19.0)]);
            canvas.stroke(hdc,&[(19.0,5.0),(5.0,19.0)]);
        }
    }
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush);
    DeleteObject(pen);
}

/// Clear the old text glyph first, preserving header geometry and click actions.
pub unsafe fn paint_button(hdc: HDC, rect: RECT, icon: Icon, background: u32) {
    let brush = CreateSolidBrush(background);
    if !brush.is_null() {
        FillRect(hdc, &rect, brush);
        DeleteObject(brush);
    }
    paint(hdc, rect, icon, background);
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
    fn all_eight_svg_assets_are_available() {
        for svg in [
            include_str!("../assets/header-icons/copy.svg"),
            include_str!("../assets/header-icons/reset.svg"),
            include_str!("../assets/header-icons/settings.svg"),
            include_str!("../assets/header-icons/collapse-left.svg"),
            include_str!("../assets/header-icons/collapse-right.svg"),
            include_str!("../assets/header-icons/collapse-top.svg"),
            include_str!("../assets/header-icons/collapse-bottom.svg"),
            include_str!("../assets/header-icons/close.svg"),
        ] {
            assert!(svg.contains("viewBox=\"0 0 24 24\""));
            assert!(svg.contains("#fff"));
        }
    }
}
