use crate::logging;
use libloading::Library;
use std::{
    env,
    ffi::{c_char, c_int, c_uchar, c_void, CStr, CString},
    path::PathBuf,
    ptr,
    slice,
    sync::Arc,
};

pub const DLT_NULL: i32 = 0;
pub const DLT_EN10MB: i32 = 1;
pub const DLT_RAW: i32 = 12;
pub const DLT_LOOP: i32 = 108;
pub const DLT_IPV4: i32 = 228;
pub const DLT_IPV6: i32 = 229;

const ERRBUF: usize = 256;

#[repr(C)]
struct PcapIf {
    next: *mut PcapIf,
    name: *mut c_char,
    description: *mut c_char,
    addresses: *mut c_void,
    flags: u32,
}

#[repr(C)]
struct TimeVal {
    tv_sec: i32,
    tv_usec: i32,
}

#[repr(C)]
struct PcapPkthdr {
    ts: TimeVal,
    caplen: u32,
    len: u32,
}

#[repr(C)]
struct BpfProgram {
    bf_len: u32,
    bf_insns: *mut c_void,
}

type PcapCreate = unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_void;
type PcapActivate = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcapSetInt = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type PcapSetImmediate = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type PcapDatalink = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcapNextEx = unsafe extern "C" fn(*mut c_void, *mut *const PcapPkthdr, *mut *const c_uchar) -> c_int;
type PcapClose = unsafe extern "C" fn(*mut c_void);
type PcapGetErr = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type PcapCompile = unsafe extern "C" fn(*mut c_void, *mut BpfProgram, *const c_char, c_int, u32) -> c_int;
type PcapSetFilter = unsafe extern "C" fn(*mut c_void, *mut BpfProgram) -> c_int;
type PcapFreeCode = unsafe extern "C" fn(*mut BpfProgram);
type PcapFindAllDevs = unsafe extern "C" fn(*mut *mut PcapIf, *mut c_char) -> c_int;
type PcapFreeAllDevs = unsafe extern "C" fn(*mut PcapIf);
type PcapLibVersion = unsafe extern "C" fn() -> *const c_char;

pub struct PcapApi {
    _lib: Library,
    create: PcapCreate,
    activate: PcapActivate,
    set_snaplen: PcapSetInt,
    set_promisc: PcapSetInt,
    set_timeout: PcapSetInt,
    set_buffer_size: PcapSetInt,
    set_immediate: Option<PcapSetImmediate>,
    datalink: PcapDatalink,
    next_ex: PcapNextEx,
    close: PcapClose,
    geterr: PcapGetErr,
    compile: PcapCompile,
    setfilter: PcapSetFilter,
    freecode: PcapFreeCode,
    findalldevs: PcapFindAllDevs,
    freealldevs: PcapFreeAllDevs,
    lib_version: Option<PcapLibVersion>,
}

unsafe impl Send for PcapApi {}
unsafe impl Sync for PcapApi {}

#[derive(Clone, Debug)]
pub struct NpcapDevice {
    pub name: String,
    pub description: String,
}

impl PcapApi {
    pub fn load() -> Result<Arc<Self>, String> {
        let mut candidates = Vec::<PathBuf>::new();
        if let Some(windir) = env::var_os("WINDIR") {
            let root = PathBuf::from(windir);
            candidates.push(root.join("System32").join("Npcap").join("wpcap.dll"));
            candidates.push(root.join("SysWOW64").join("Npcap").join("wpcap.dll"));
        }
        candidates.push(PathBuf::from("wpcap.dll"));

        let mut last_error = String::new();
        for path in candidates {
            let lib = match unsafe { Library::new(&path) } {
                Ok(lib) => lib,
                Err(err) => { last_error = format!("{}: {err}", path.display()); continue; }
            };
            unsafe {
                macro_rules! req {
                    ($name:literal, $ty:ty) => {{
                        match lib.get::<$ty>(concat!($name, "\0").as_bytes()) {
                            Ok(v) => *v,
                            Err(err) => { last_error = format!("{} missing {}: {err}", path.display(), $name); continue; }
                        }
                    }};
                }
                let create = req!("pcap_create", PcapCreate);
                let activate = req!("pcap_activate", PcapActivate);
                let set_snaplen = req!("pcap_set_snaplen", PcapSetInt);
                let set_promisc = req!("pcap_set_promisc", PcapSetInt);
                let set_timeout = req!("pcap_set_timeout", PcapSetInt);
                let set_buffer_size = req!("pcap_set_buffer_size", PcapSetInt);
                let datalink = req!("pcap_datalink", PcapDatalink);
                let next_ex = req!("pcap_next_ex", PcapNextEx);
                let close = req!("pcap_close", PcapClose);
                let geterr = req!("pcap_geterr", PcapGetErr);
                let compile = req!("pcap_compile", PcapCompile);
                let setfilter = req!("pcap_setfilter", PcapSetFilter);
                let freecode = req!("pcap_freecode", PcapFreeCode);
                let findalldevs = req!("pcap_findalldevs", PcapFindAllDevs);
                let freealldevs = req!("pcap_freealldevs", PcapFreeAllDevs);
                let set_immediate = lib.get::<PcapSetImmediate>(b"pcap_set_immediate_mode\0").ok().map(|x| *x);
                let lib_version = lib.get::<PcapLibVersion>(b"pcap_lib_version\0").ok().map(|x| *x);
                logging::write(format!("npcap: loaded {}", path.display()));
                return Ok(Arc::new(Self {
                    _lib: lib,
                    create, activate, set_snaplen, set_promisc, set_timeout, set_buffer_size,
                    set_immediate, datalink, next_ex, close, geterr, compile, setfilter,
                    freecode, findalldevs, freealldevs, lib_version,
                }));
            }
        }
        Err(format!("Npcap wpcap.dll could not be loaded. {last_error}"))
    }

    pub fn version(&self) -> String {
        unsafe {
            self.lib_version
                .and_then(|f| { let p = f(); if p.is_null() { None } else { CStr::from_ptr(p).to_str().ok().map(ToOwned::to_owned) } })
                .unwrap_or_else(|| "Npcap/libpcap".into())
        }
    }

    pub fn devices(&self) -> Result<Vec<NpcapDevice>, String> {
        let mut all: *mut PcapIf = ptr::null_mut();
        let mut err = [0i8; ERRBUF];
        let code = unsafe { (self.findalldevs)(&mut all, err.as_mut_ptr()) };
        if code == -1 { return Err(c_error(&err)); }
        let mut devices = Vec::new();
        unsafe {
            let mut cur = all;
            while !cur.is_null() {
                let item = &*cur;
                if !item.name.is_null() {
                    let name = CStr::from_ptr(item.name).to_string_lossy().into_owned();
                    let description = if item.description.is_null() { name.clone() }
                    else { CStr::from_ptr(item.description).to_string_lossy().into_owned() };
                    if !name.trim().is_empty() { devices.push(NpcapDevice { name, description }); }
                }
                cur = item.next;
            }
            if !all.is_null() { (self.freealldevs)(all); }
        }
        Ok(devices)
    }

    fn error(&self, handle: *mut c_void) -> String {
        unsafe {
            let ptr = (self.geterr)(handle);
            if ptr.is_null() { "unknown Npcap error".into() }
            else { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
        }
    }
}

pub struct CaptureHandle {
    api: Arc<PcapApi>,
    handle: *mut c_void,
    pub datalink: i32,
    pub device_name: String,
}

unsafe impl Send for CaptureHandle {}

impl CaptureHandle {
    pub fn open(api: Arc<PcapApi>, device_name: &str) -> Result<Self, String> {
        let name = CString::new(device_name).map_err(|_| "Npcap device name contains NUL".to_string())?;
        let mut err = [0i8; ERRBUF];
        let handle = unsafe { (api.create)(name.as_ptr(), err.as_mut_ptr()) };
        if handle.is_null() { return Err(format!("pcap_create failed: {}", c_error(&err))); }

        let configure = (|| {
            check(&api, handle, unsafe { (api.set_snaplen)(handle, 65_536) }, "pcap_set_snaplen")?;
            check(&api, handle, unsafe { (api.set_promisc)(handle, 1) }, "pcap_set_promisc")?;
            check(&api, handle, unsafe { (api.set_timeout)(handle, 1) }, "pcap_set_timeout")?;
            check(&api, handle, unsafe { (api.set_buffer_size)(handle, 16 * 1024 * 1024) }, "pcap_set_buffer_size")?;
            if let Some(set_immediate) = api.set_immediate {
                let code = unsafe { set_immediate(handle, 1) };
                if code != 0 { logging::write(format!("npcap: immediate mode unavailable code={code}; using 1ms timeout")); }
            }
            let activated = unsafe { (api.activate)(handle) };
            if activated < 0 { return Err(format!("pcap_activate failed ({activated}): {}", api.error(handle))); }
            if activated > 0 { logging::write(format!("npcap: activation warning={activated}: {}", api.error(handle))); }

            let expression = CString::new("tcp and not portrange 0-1000").unwrap();
            let mut program = BpfProgram { bf_len: 0, bf_insns: ptr::null_mut() };
            if unsafe { (api.compile)(handle, &mut program, expression.as_ptr(), 1, u32::MAX) } != 0 {
                return Err(format!("pcap_compile failed: {}", api.error(handle)));
            }
            let filter_result = unsafe { (api.setfilter)(handle, &mut program) };
            unsafe { (api.freecode)(&mut program); }
            if filter_result != 0 { return Err(format!("pcap_setfilter failed: {}", api.error(handle))); }
            Ok(unsafe { (api.datalink)(handle) })
        })();

        match configure {
            Ok(datalink) => Ok(Self { api, handle, datalink, device_name: device_name.to_string() }),
            Err(err) => {
                unsafe { (api.close)(handle); }
                Err(err)
            }
        }
    }

    pub fn next_packet(&mut self) -> Result<Option<&[u8]>, String> {
        let mut header: *const PcapPkthdr = ptr::null();
        let mut data: *const c_uchar = ptr::null();
        let code = unsafe { (self.api.next_ex)(self.handle, &mut header, &mut data) };
        match code {
            1 => unsafe {
                if header.is_null() || data.is_null() { return Ok(None); }
                let len = (*header).caplen as usize;
                if len == 0 { return Ok(None); }
                Ok(Some(slice::from_raw_parts(data, len)))
            },
            0 | -2 => Ok(None),
            -1 => Err(format!("pcap_next_ex failed: {}", self.api.error(self.handle))),
            other => Err(format!("unexpected pcap_next_ex result {other}")),
        }
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { (self.api.close)(self.handle); }
            self.handle = ptr::null_mut();
        }
    }
}

fn check(api: &PcapApi, handle: *mut c_void, code: i32, name: &str) -> Result<(), String> {
    if code == 0 { Ok(()) } else { Err(format!("{name} failed ({code}): {}", api.error(handle))) }
}

fn c_error(err: &[i8; ERRBUF]) -> String {
    unsafe { CStr::from_ptr(err.as_ptr()).to_string_lossy().into_owned() }
}
