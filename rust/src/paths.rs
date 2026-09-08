use std::{env, fs, io, path::PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub settings: PathBuf,
    pub log: PathBuf,
    pub chat_logs: PathBuf,
}

impl AppPaths {
    pub fn create() -> io::Result<Self> {
        let local = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
            .unwrap_or_else(|| env::temp_dir());
        let root = local.join("BPSR-ReadyAlert");
        let chat_logs = root.join("ChatLogs");
        fs::create_dir_all(&chat_logs)?;
        Ok(Self {
            settings: root.join("settings.json"),
            log: root.join("readyalert.log"),
            root,
            chat_logs,
        })
    }
}
