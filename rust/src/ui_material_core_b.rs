fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn make_font(height: i32, weight: i32) -> usize {
    unsafe {
        let face = wide("Segoe UI Variable Text");
        CreateFontW(height, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) as usize
    }
}

fn body_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 400)) as HFONT
}

fn medium_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 600)) as HFONT
}

fn heading_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-18, 600)) as HFONT
}

fn family_bg(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BG, SurfaceFamily::Dark => DARK_BG }
}
fn family_surface(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SURFACE, SurfaceFamily::Dark => DARK_SURFACE }
}
fn family_raised(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_RAISED, SurfaceFamily::Dark => DARK_RAISED }
}
fn family_hover(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_HOVER, SurfaceFamily::Dark => DARK_HOVER }
}
fn family_pressed(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_PRESSED, SurfaceFamily::Dark => DARK_PRESSED }
}
fn family_input(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_INPUT, SurfaceFamily::Dark => DARK_INPUT }
}
fn family_border(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BORDER, SurfaceFamily::Dark => DARK_BORDER }
}
fn family_border_strong(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BORDER_STRONG, SurfaceFamily::Dark => DARK_BORDER_STRONG }
}
fn family_selected(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SELECTED, SurfaceFamily::Dark => SELECTED_SURFACE }
}
fn family_selected_hover(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SELECTED_HOVER, SurfaceFamily::Dark => SELECTED_SURFACE_HOVER }
}
