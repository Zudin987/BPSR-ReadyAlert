fn main() {
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
