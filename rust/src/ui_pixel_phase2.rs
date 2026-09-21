// Phase 2: finish OS-owned/native surfaces independently of client palette tokens.
// Retain the lightweight Win32/GDI window architecture and input semantics.

#[link(name = "kernel32")]
extern "system" {
    fn GetProcAddress(module: *mut c_void, proc_name: *const u8) -> *mut c_void;
}

type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemesFn = unsafe extern "system" fn();
type AllowDarkModeForWindowFn = unsafe extern "system" fn(HWND, i32) -> i32;

unsafe fn uxtheme_ordinal(ordinal: usize) -> *mut c_void {
    let dll = wide("uxtheme.dll");
    let module = GetModuleHandleW(dll.as_ptr());
    if module.is_null() { return null_mut(); }
    GetProcAddress(module, ordinal as *const u8)
}

/// Opportunistic legacy UxTheme menu integration. Unsupported exports must
/// never prevent using a window, combo, menu or scrollbar.
pub unsafe fn enable_dark_system_surfaces() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        unsafe {
            let preferred = uxtheme_ordinal(135);
            if !preferred.is_null() {
                let set_preferred: SetPreferredAppModeFn = std::mem::transmute(preferred);
                let _ = set_preferred(2);
            }
            let flush = uxtheme_ordinal(136);
            if !flush.is_null() {
                let flush_menu_themes: FlushMenuThemesFn = std::mem::transmute(flush);
                flush_menu_themes();
            }
        }
    });
}

/// Applies the real native theme, then the documented DWM dark-caption and
/// border attributes from ui_pixel_core::style_titlebar. Applying DWM *last*
/// avoids a theme change wiping out caption colours. The independent factory
/// can subsequently choose mist_titlebar where the shell uses Mist colours.
pub unsafe fn finish_native_window(hwnd: HWND) {
    if hwnd.is_null() { return; }
    enable_dark_system_surfaces();
    let allow = uxtheme_ordinal(133);
    if !allow.is_null() {
        let allow_dark: AllowDarkModeForWindowFn = std::mem::transmute(allow);
        let _ = allow_dark(hwnd, 1);
    }
    let theme = wide("DarkMode_Explorer");
    let _ = SetWindowTheme(hwnd, theme.as_ptr(), null());
    dark_titlebar(hwnd);
}

pub fn mix_color(base: u32, tint: u32, tint_percent: u32) -> u32 {
    let p = tint_percent.min(100);
    let keep = 100 - p;
    let channel = |shift: u32| -> u32 {
        let a = (base >> shift) & 0xff;
        let b = (tint >> shift) & 0xff;
        (a * keep + b * p + 50) / 100
    };
    channel(0) | (channel(8) << 8) | (channel(16) << 16)
}

pub fn tonal_class_surface(class_color: u32) -> u32 {
    mix_color(DARK_RAISED, class_color, 26)
}

pub fn tonal_danger_surface() -> u32 {
    mix_color(DARK_RAISED, BPSR_DANGER, 30)
}

#[cfg(test)]
mod phase2_tests {
    use super::*;

    #[test]
    fn class_surface_is_tonal_not_flat_class_color() {
        let class = crate::ui_theme::rgb(82, 108, 63);
        let surface = tonal_class_surface(class);
        assert_ne!(surface, class);
        assert_ne!(surface, DARK_RAISED);
    }

    #[test]
    fn color_mix_clamps_percent() {
        assert_eq!(mix_color(DARK_BG, BPSR_ACCENT, 100), BPSR_ACCENT);
        assert_eq!(mix_color(DARK_BG, BPSR_ACCENT, 140), BPSR_ACCENT);
        assert_eq!(mix_color(DARK_BG, BPSR_ACCENT, 0), DARK_BG);
    }
}
