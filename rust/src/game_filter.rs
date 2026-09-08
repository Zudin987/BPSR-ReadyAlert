use crate::{logging, npcap::{DLT_EN10MB, DLT_IPV4, DLT_IPV6, DLT_LOOP, DLT_NULL, DLT_RAW}};
use std::{collections::HashSet, ffi::c_void, mem, ptr, time::{Duration, Instant}};
use windows_sys::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS},
};

const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const GAME_NAMES: &[&str] = &["BPSR", "BPSR_STEAM", "BPSR_EPIC", "StarSEA", "StarASIA", "StarSEA_STEAM", "StarASIA_STEAM", "Star"];

#[link(name = "iphlpapi")]
extern "system" {
    fn GetExtendedTcpTable(
        table: *mut c_void,
        size: *mut u32,
        order: i32,
        family: u32,
        table_class: u32,
        reserved: u32,
    ) -> u32;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint {
    ip_version: u8,
    address: [u8; 16],
    port: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct PacketTcpInfo {
    pub source: Endpoint,
    pub destination: Endpoint,
    pub flags: u8,
    pub seq: u32,
    pub payload_offset: usize,
    pub payload_len: usize,
    pub packet_end: usize,
}

pub struct GamePacketFilter {
    pids: HashSet<u32>,
    endpoints: HashSet<Endpoint>,
    last_pid_refresh: Instant,
    last_endpoint_refresh: Instant,
    last_forced_refresh: Instant,
    last_summary: String,
}

impl GamePacketFilter {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            pids: HashSet::new(),
            endpoints: HashSet::new(),
            last_pid_refresh: now.checked_sub(Duration::from_secs(10)).unwrap_or(now),
            last_endpoint_refresh: now.checked_sub(Duration::from_secs(10)).unwrap_or(now),
            last_forced_refresh: now.checked_sub(Duration::from_secs(10)).unwrap_or(now),
            last_summary: String::new(),
        }
    }

    pub fn game_running(&mut self) -> bool {
        self.refresh(false);
        !self.pids.is_empty()
    }

    pub fn is_game_packet(&mut self, packet: &[u8], datalink: i32) -> bool {
        let Some(info) = parse_tcp(packet, datalink) else { return false; };
        self.refresh(false);
        if self.endpoints.contains(&info.source) || self.endpoints.contains(&info.destination) {
            return true;
        }
        if info.flags & 0x02 == 0 || self.last_forced_refresh.elapsed() < Duration::from_millis(50) {
            return false;
        }
        self.last_forced_refresh = Instant::now();
        self.refresh(true);
        self.endpoints.contains(&info.source) || self.endpoints.contains(&info.destination)
    }

    fn refresh(&mut self, force: bool) {
        let pid_interval = if self.pids.is_empty() { Duration::from_millis(100) } else { Duration::from_secs(2) };
        if force || self.last_pid_refresh.elapsed() >= pid_interval {
            self.last_pid_refresh = Instant::now();
            self.pids = find_game_pids();
        }
        if !force && self.last_endpoint_refresh.elapsed() < Duration::from_millis(100) { return; }
        self.last_endpoint_refresh = Instant::now();
        let mut endpoints = HashSet::new();
        if !self.pids.is_empty() {
            if let Ok(rows) = read_ipv4_rows() {
                for (endpoint, pid) in rows { if self.pids.contains(&pid) { endpoints.insert(endpoint); } }
            }
            if let Ok(rows) = read_ipv6_rows() {
                for (endpoint, pid) in rows { if self.pids.contains(&pid) { endpoints.insert(endpoint); } }
            }
        }
        self.endpoints = endpoints;
        let mut pid_list: Vec<_> = self.pids.iter().copied().collect();
        pid_list.sort_unstable();
        let summary = format!("pids={pid_list:?} endpoints={}", self.endpoints.len());
        if summary != self.last_summary {
            self.last_summary = summary.clone();
            logging::write(format!("game-filter: {summary}"));
        }
    }
}

pub fn parse_tcp(packet: &[u8], datalink: i32) -> Option<PacketTcpInfo> {
    let (offset, forced_version) = match datalink {
        DLT_RAW => (0usize, 0u8),
        DLT_IPV4 => (0, 4),
        DLT_IPV6 => (0, 6),
        DLT_NULL | DLT_LOOP => (4, 0),
        DLT_EN10MB => ethernet_offset(packet)?,
        _ => return None,
    };
    if offset >= packet.len() { return None; }
    let version = if forced_version != 0 { forced_version } else { packet[offset] >> 4 };
    match version {
        4 => parse_ipv4(packet, offset),
        6 => parse_ipv6(packet, offset),
        _ => None,
    }
}

fn ethernet_offset(packet: &[u8]) -> Option<(usize, u8)> {
    if packet.len() < 14 { return None; }
    let mut ether = be16(packet, 12)?;
    let mut cursor = 14usize;
    let mut depth = 0;
    while matches!(ether, 0x8100 | 0x88A8 | 0x9100) {
        depth += 1;
        if depth > 2 || cursor + 4 > packet.len() { return None; }
        ether = be16(packet, cursor + 2)?;
        cursor += 4;
    }
    match ether { 0x0800 => Some((cursor, 4)), 0x86DD => Some((cursor, 6)), _ => None }
}

fn parse_ipv4(packet: &[u8], ip: usize) -> Option<PacketTcpInfo> {
    if ip + 20 > packet.len() || packet[ip] >> 4 != 4 { return None; }
    let ip_header = usize::from(packet[ip] & 0x0f) * 4;
    if ip_header < 20 || ip + ip_header + 20 > packet.len() || packet[ip + 9] != 6 { return None; }
    let total = usize::from(be16(packet, ip + 2)?);
    let packet_end = if total >= ip_header { (ip + total).min(packet.len()) } else { packet.len() };
    let tcp = ip + ip_header;
    let tcp_header = usize::from((packet[tcp + 12] >> 4) & 0x0f) * 4;
    if tcp_header < 20 || tcp + tcp_header > packet_end { return None; }
    let src_port = be16(packet, tcp)?;
    let dst_port = be16(packet, tcp + 2)?;
    if src_port <= 1000 || dst_port <= 1000 { return None; }
    let source = Endpoint::v4(packet[ip+12..ip+16].try_into().ok()?, src_port);
    let destination = Endpoint::v4(packet[ip+16..ip+20].try_into().ok()?, dst_port);
    let payload_offset = tcp + tcp_header;
    Some(PacketTcpInfo {
        source, destination,
        flags: packet[tcp + 13],
        seq: be32(packet, tcp + 4)?,
        payload_offset,
        payload_len: packet_end.saturating_sub(payload_offset),
        packet_end,
    })
}

fn parse_ipv6(packet: &[u8], ip: usize) -> Option<PacketTcpInfo> {
    if ip + 40 > packet.len() || packet[ip] >> 4 != 6 { return None; }
    let payload_length = usize::from(be16(packet, ip + 4)?);
    let packet_end = if payload_length == 0 { packet.len() } else { (ip + 40 + payload_length).min(packet.len()) };
    let mut next = packet[ip + 6];
    let mut cursor = ip + 40;
    for _ in 0..8 {
        if next == 6 { break; }
        if cursor + 2 > packet_end { return None; }
        match next {
            0 | 43 | 60 => {
                next = packet[cursor];
                let len = (usize::from(packet[cursor + 1]) + 1) * 8;
                if len < 8 || cursor + len > packet_end { return None; }
                cursor += len;
            }
            44 => {
                if cursor + 8 > packet_end { return None; }
                next = packet[cursor];
                if be16(packet, cursor + 2)? & 0xfff8 != 0 { return None; }
                cursor += 8;
            }
            51 => {
                next = packet[cursor];
                let len = (usize::from(packet[cursor + 1]) + 2) * 4;
                if len < 8 || cursor + len > packet_end { return None; }
                cursor += len;
            }
            _ => return None,
        }
    }
    if next != 6 || cursor + 20 > packet_end { return None; }
    let tcp_header = usize::from((packet[cursor + 12] >> 4) & 0x0f) * 4;
    if tcp_header < 20 || cursor + tcp_header > packet_end { return None; }
    let src_port = be16(packet, cursor)?;
    let dst_port = be16(packet, cursor + 2)?;
    if src_port <= 1000 || dst_port <= 1000 { return None; }
    let source = Endpoint::v6(packet[ip+8..ip+24].try_into().ok()?, src_port);
    let destination = Endpoint::v6(packet[ip+24..ip+40].try_into().ok()?, dst_port);
    let payload_offset = cursor + tcp_header;
    Some(PacketTcpInfo {
        source, destination,
        flags: packet[cursor + 13],
        seq: be32(packet, cursor + 4)?,
        payload_offset,
        payload_len: packet_end.saturating_sub(payload_offset),
        packet_end,
    })
}

impl Endpoint {
    fn v4(v: [u8;4], port: u16) -> Self {
        let mut address = [0u8;16]; address[12..].copy_from_slice(&v);
        Self { ip_version: 4, address, port }
    }
    fn v6(v: [u8;16], port: u16) -> Self { Self { ip_version: 6, address: v, port } }
}

fn find_game_pids() -> HashSet<u32> {
    let mut result = HashSet::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE { return result; }
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            let end = entry.szExeFile.iter().position(|x| *x == 0).unwrap_or(entry.szExeFile.len());
            let mut name = String::from_utf16_lossy(&entry.szExeFile[..end]);
            if name.to_ascii_lowercase().ends_with(".exe") { name.truncate(name.len().saturating_sub(4)); }
            if GAME_NAMES.iter().any(|candidate| candidate.eq_ignore_ascii_case(&name)) {
                result.insert(entry.th32ProcessID);
            }
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TcpRow4 { state:u32, local_addr:u32, local_port:u32, remote_addr:u32, remote_port:u32, pid:u32 }
#[repr(C)]
#[derive(Clone, Copy)]
struct TcpRow6 { local_addr:[u8;16], local_scope:u32, local_port:u32, remote_addr:[u8;16], remote_scope:u32, remote_port:u32, state:u32, pid:u32 }

fn read_ipv4_rows() -> Result<Vec<(Endpoint,u32)>, String> {
    read_table::<TcpRow4>(AF_INET).map(|rows| rows.into_iter().map(|row| {
        let port = decode_port(row.local_port);
        (Endpoint::v4(row.local_addr.to_ne_bytes(), port), row.pid)
    }).collect())
}

fn read_ipv6_rows() -> Result<Vec<(Endpoint,u32)>, String> {
    read_table::<TcpRow6>(AF_INET6).map(|rows| rows.into_iter().map(|row| {
        let port = decode_port(row.local_port);
        (Endpoint::v6(row.local_addr, port), row.pid)
    }).collect())
}

fn read_table<T: Copy>(family: u32) -> Result<Vec<T>, String> {
    unsafe {
        let mut size = 0u32;
        let first = GetExtendedTcpTable(ptr::null_mut(), &mut size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0);
        if first != ERROR_INSUFFICIENT_BUFFER && first != 0 { return Err(format!("GetExtendedTcpTable(size)={first}")); }
        if size <= 4 { return Ok(Vec::new()); }
        let mut bytes = vec![0u8; size as usize];
        let code = GetExtendedTcpTable(bytes.as_mut_ptr().cast(), &mut size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0);
        if code != 0 { return Err(format!("GetExtendedTcpTable(data)={code}")); }
        let count = u32::from_ne_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let row_size = mem::size_of::<T>();
        let available = bytes.len().saturating_sub(4) / row_size;
        let count = count.min(available);
        let base = bytes.as_ptr().add(4) as *const T;
        Ok((0..count).map(|i| ptr::read_unaligned(base.add(i))).collect())
    }
}

fn decode_port(native: u32) -> u16 { u16::from_be((native & 0xffff) as u16) }
fn be16(data: &[u8], at: usize) -> Option<u16> { Some(u16::from_be_bytes(data.get(at..at+2)?.try_into().ok()?)) }
fn be32(data: &[u8], at: usize) -> Option<u32> { Some(u32::from_be_bytes(data.get(at..at+4)?.try_into().ok()?)) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ethernet_ipv4_tcp_parse() {
        let mut p = vec![0u8; 14 + 20 + 20 + 3];
        p[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        let ip=14; p[ip]=0x45; p[ip+2..ip+4].copy_from_slice(&(43u16).to_be_bytes()); p[ip+9]=6;
        p[ip+12..ip+16].copy_from_slice(&[10,0,0,1]); p[ip+16..ip+20].copy_from_slice(&[1,2,3,4]);
        let tcp=34; p[tcp..tcp+2].copy_from_slice(&40000u16.to_be_bytes()); p[tcp+2..tcp+4].copy_from_slice(&50000u16.to_be_bytes()); p[tcp+12]=0x50; p[tcp+13]=0x18;
        let info=parse_tcp(&p, DLT_EN10MB).unwrap(); assert_eq!(info.payload_len,3); assert_eq!(info.source.port,40000);
    }
}
