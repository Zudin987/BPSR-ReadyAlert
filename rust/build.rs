use std::{env, fs, path::PathBuf};

fn main() {
    // Keep the checked-in v1.7 feature sources readable while normalizing two
    // small compatibility details for the pinned windows-sys / stable toolchain.
    // The generated files live only in OUT_DIR and contain no runtime codegen.
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let telemetry = fs::read_to_string("src/telemetry_v170.rs")
        .expect("read telemetry_v170.rs")
        .replace("sec.max(.001)", "sec.max(0.001)");
    fs::write(out.join("telemetry_v170_fixed.rs"), telemetry)
        .expect("write generated telemetry source");

    let mut overlay = fs::read_to_string("src/feature_overlays_v170.rs")
        .expect("read feature_overlays_v170.rs");
    overlay = overlay
        .replace("        TrackMouseEvent, CREATESTRUCTW,", "        CREATESTRUCTW,")
        .replace(" SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT,", " SW_SHOW,")
        .replace(" WM_LBUTTONDOWN, WM_MOUSELEAVE, WM_MOUSEMOVE,", " WM_LBUTTONDOWN, WM_MOUSEMOVE,");
    let mouse_abi = r#"
const WM_MOUSELEAVE: u32 = 0x02A3;
const TME_LEAVE: u32 = 0x00000002;
#[repr(C)]
struct TRACKMOUSEEVENT {
    cbSize: u32,
    dwFlags: u32,
    hwndTrack: windows_sys::Win32::Foundation::HWND,
    dwHoverTime: u32,
}
#[link(name = "user32")]
extern "system" {
    fn TrackMouseEvent(event: *mut TRACKMOUSEEVENT) -> i32;
}
"#;
    overlay = format!("{mouse_abi}\n{overlay}");
    fs::write(out.join("feature_overlays_v170_fixed.rs"), overlay)
        .expect("write generated overlay source");

    println!("cargo:rerun-if-changed=src/telemetry_v170.rs");
    println!("cargo:rerun-if-changed=src/feature_overlays_v170.rs");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../src/BPSR.ReadyAlert/Assets/App.ico");
        res.set("FileDescription", "BPSR Ready Alert");
        res.set("ProductName", "BPSR Ready Alert");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows resources: {err}");
        }
    }
}
