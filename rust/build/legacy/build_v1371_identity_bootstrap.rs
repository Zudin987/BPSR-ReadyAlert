// Conservative owner bootstrap from observed self character/container sync.
// No dependency on speculative dungeon flow/end signals.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1370_benchmark_lifecycle.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, before: &str, after: &str, label: &str) {
    let count = src.matches(before).count();
    assert_eq!(count, 1, "identity bootstrap {label}: expected one anchor; found {count}");
    *src = src.replacen(before, after, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("capture_v185.rs");
    let mut src = fs::read_to_string(&path).expect("generated capture").replace("\r\n", "\n");
    replace_once(&mut src,
        "const MAX_GAME_FRAME: usize = 8 * 1024 * 1024;",
        "mod owner_bootstrap {\n    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/identity_bootstrap.rs\"));\n}\n\nconst MAX_GAME_FRAME: usize = 8 * 1024 * 1024;",
        "self-owner decoder module");
    replace_once(&mut src,
        "    tx:Sender<AppEvent>, identity:Arc<RwLock<Option<PlayerIdentity>>>, chat:ChatRuntime, telemetry:TelemetryRuntime,",
        "    tx:Sender<AppEvent>, identity:Arc<RwLock<Option<PlayerIdentity>>>, chat:ChatRuntime, telemetry:TelemetryRuntime, owner_bootstrap:owner_bootstrap::OwnerBootstrap,",
        "capture owner state");
    replace_once(&mut src,
        "        Self { tx,identity,chat,telemetry,flows:HashMap::new(),",
        "        Self { tx,identity,chat,telemetry,owner_bootstrap:owner_bootstrap::OwnerBootstrap::default(),flows:HashMap::new(),",
        "capture owner initialization");
    replace_once(&mut src,
        "        if service==proto::WORLD_SERVICE&&method==proto::ENTER_SCENE_METHOD{if let Some(id)=proto::parse_identity(body){let changed=self.identity.read().ok().and_then(|g|g.clone()).map(|x|x.uid!=id.uid||x.name!=id.name).unwrap_or(true);if changed{if let Ok(mut g)=self.identity.write(){*g=Some(id.clone());}let _=self.tx.send(AppEvent::Identity(id));}}}",
        "        // Capture may begin after EnterScene. Recover only from an own\n        // character UID and a matching observed name; never guess from roster.\n        self.owner_bootstrap.observe(service,method,body,&self.identity,&self.tx);",
        "replace EnterScene-only owner identification");
    fs::write(path, src).expect("write owner-aware generated capture");
    println!("cargo:rerun-if-changed=build/legacy/build_v1371_identity_bootstrap.rs");
    println!("cargo:rerun-if-changed=src/identity_bootstrap.rs");
}
