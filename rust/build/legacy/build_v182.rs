use std::{env, fs, path::{Path, PathBuf}, process::Command};

mod previous {
    include!("build.rs");
    pub fn run() { main(); }
}

fn patch_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.8.2 patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    assert_eq!(start_count, 1, "v1.8.2 patch `{label}` start expected one match, found {start_count}");
    let begin = source.find(start).expect("start checked");
    let tail = &source[begin..];
    let rel_end = tail.find(end).unwrap_or_else(|| panic!("v1.8.2 patch `{label}` end anchor missing"));
    let finish = begin + rel_end;
    source.replace_range(begin..finish, replacement);
}

fn ps_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn prepare_imagine_assets(out: &Path) {
    if !cfg!(windows) { return; }
    let commit = "bd71d2dfd3c7289e6398c4cd042f4357d4f35721";
    let assets = [
        (3903, "018", 27_864u64),
        (3920, "013", 27_447u64),
        (3921, "016", 28_051u64),
        (3944, "048", 29_088u64),
    ];
    for (skill, icon, expected_size) in assets {
        let png = out.join(format!("imagine_{skill}.png"));
        let bmp = out.join(format!("imagine_{skill}.bmp"));
        let url = format!("https://raw.githubusercontent.com/fudiyangjin/resonance-logs-cn/{commit}/static/images/resonance_skill/skill_aoyi_skill_icon_{icon}.png");
        let script = format!(r#"
$ErrorActionPreference='Stop'
$url='{url}'
$png='{png}'
$bmp='{bmp}'
Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $png
$bytes=[IO.File]::ReadAllBytes($png)
if ($bytes.Length -ne {expected_size}) {{ throw "Imagine source size mismatch: $($bytes.Length)" }}
if ($bytes.Length -lt 24 -or $bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50 -or $bytes[2] -ne 0x4e -or $bytes[3] -ne 0x47) {{ throw 'Imagine source is not PNG' }}
Add-Type -AssemblyName System.Drawing
$src=[System.Drawing.Image]::FromFile($png)
try {{
  $dst=[System.Drawing.Bitmap]::new(24,24,[System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  try {{
    $g=[System.Drawing.Graphics]::FromImage($dst)
    try {{
      $g.Clear([System.Drawing.Color]::FromArgb(30,35,43))
      $g.InterpolationMode=[System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $g.PixelOffsetMode=[System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
      $g.DrawImage($src,0,0,24,24)
    }} finally {{ $g.Dispose() }}
    $dst.Save($bmp,[System.Drawing.Imaging.ImageFormat]::Bmp)
  }} finally {{ $dst.Dispose() }}
}} finally {{ $src.Dispose() }}
$b=[IO.File]::ReadAllBytes($bmp)
if ($b.Length -lt 54 -or $b[0] -ne 0x42 -or $b[1] -ne 0x4d) {{ throw 'Generated Imagine asset is not BMP' }}
"#,
            url = url,
            png = ps_quote(&png),
            bmp = ps_quote(&bmp),
            expected_size = expected_size,
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .unwrap_or_else(|err| panic!("failed to launch PowerShell for Imagine {skill}: {err}"));
        assert!(status.success(), "failed to prepare game Imagine asset for skill {skill}");
    }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    prepare_imagine_assets(&out);

    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read v1.8.1 generated telemetry");
    patch_once(
        &mut telemetry,
        "const ACTOR_STATE_DEAD: i64 = 9;",
        "const ACTOR_STATE_DEAD: i64 = 9;\nconst ACTOR_STATE_TELEPORT: i64 = 13;\nconst ACTOR_STATE_RESURRECTION: i64 = 27;",
        "actor wipe states",
    );
    patch_once(
        &mut telemetry,
        "consumable_instances: HashMap<(i64, i32), ConsumableKind>,",
        "consumable_instances: HashMap<(i64, i32), (ConsumableKind, ConsumableStatus)>,",
        "remember consumable status per buff instance",
    );
    patch_once(
        &mut telemetry,
        "            if let Some(kind) = self.consumable_instances.remove(&(host, buff_uuid)) {\n                match kind {\n                    ConsumableKind::Food => self.food = None,\n                    ConsumableKind::Serum => self.serum = None,\n                }\n                self.emit_mechanics(true);\n            }",
        "            if let Some((kind, _removed)) = self.consumable_instances.remove(&(host, buff_uuid)) {\n                let now = now_ms();\n                let replacement = self.consumable_instances.iter()\n                    .filter(|((entry_host, _), (entry_kind, status))| {\n                        *entry_host == host && *entry_kind == kind\n                            && (status.expires_unix_ms <= 0 || status.expires_unix_ms > now)\n                    })\n                    .map(|(_, (_, status))| status)\n                    .max_by_key(|status| status.expires_unix_ms)\n                    .cloned();\n                match kind {\n                    ConsumableKind::Food => self.food = replacement,\n                    ConsumableKind::Serum => self.serum = replacement,\n                }\n                self.emit_mechanics(true);\n            }",
        "do not clear food when companion buff disappears",
    );
    patch_once(
        &mut telemetry,
        "        self.consumable_instances.insert((host, buff_uuid), kind);",
        "        self.consumable_instances.insert((host, buff_uuid), (kind, status.clone()));",
        "store consumable instance status",
    );
    patch_once(
        &mut telemetry,
        "        let old_food = self.food.is_some();",
        "        self.consumable_instances.retain(|_, (_, status)| status.expires_unix_ms <= 0 || status.expires_unix_ms > now);\n        let old_food = self.food.is_some();",
        "prune expired consumable instances",
    );
    replace_between(
        &mut telemetry,
        "    fn apply_player_actor_state(&mut self, uid: i64, old_state: i64, new_state: i64) {",
        "    fn apply_damage(&mut self, target_uuid: i64, damage: &[u8]) {",
        r#"    fn apply_player_actor_state(&mut self, uid: i64, old_state: i64, new_state: i64) {
        let combat = self.combat.entry(uid).or_default();
        let was_down = combat.is_dead;
        if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD {
            combat.deaths = combat.deaths.saturating_add(1);
        }
        // ZDPS treats Dead -> Resurrection -> TelePort and Dead -> TelePort as
        // wipe-state sequences. Keep a player down through that transition;
        // ordinary living states clear it.
        combat.is_dead = match new_state {
            ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION => true,
            ACTOR_STATE_TELEPORT if was_down || matches!(old_state, ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION) => true,
            _ => false,
        };
        if combat.is_dead && self.party_is_wiped() {
            self.arm_boundary();
        }
    }

    fn party_is_wiped(&self) -> bool {
        let mut roster: HashSet<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 { roster.insert(self.local_uid); }
        // Team packets can be partial. Include players that actually participated
        // in this encounter so one missing roster packet cannot turn a single
        // death into a false full-party wipe.
        for (uid, actor) in &self.combat {
            if actor.damage > 0 || actor.healing > 0 || actor.damage_taken > 0 || actor.hits > 0 || actor.deaths > 0 {
                roster.insert(*uid);
            }
        }
        // Be conservative when only one player is known. In party content this
        // avoids the v1.8.1 false reset when the roster sync is incomplete; solo
        // users can still use manual Reset or scene re-entry.
        if roster.len() < 2 { return false; }
        roster.iter().all(|uid| {
            self.combat.get(uid).is_some_and(|actor| actor.is_dead)
                || self.players.get(uid).is_some_and(|player| {
                    matches!(player.actor_state, ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION)
                        || (player.actor_state == ACTOR_STATE_TELEPORT && player.hp <= 0)
                })
        })
    }

"#,
        "conservative ZDPS-style wipe validation",
    );
    replace_between(
        &mut telemetry,
        "    fn arm_boundary(&mut self) {",
        "    fn apply_skill(&mut self, uid: i64, skill: i32, value: i64, heal: bool, lucky: bool, crit: bool) {",
        r#"    fn arm_boundary(&mut self) {
        // The CN 510072 marker is useful evidence, but it also appears around
        // individual deaths. Never arm an automatic reset unless the observed
        // encounter roster is actually down.
        if !self.party_is_wiped() { return; }
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }

"#,
        "guard every automatic wipe boundary",
    );
    fs::write(&telemetry_path, telemetry).expect("write v1.8.2 telemetry");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read v1.8.1 generated overlay");
    patch_once(
        &mut overlay,
        "BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,\nInvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, HDC, PAINTSTRUCT,\nTRANSPARENT,",
        "BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,\nInvalidateRect, SelectObject, SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HDC, HFONT, PAINTSTRUCT, SRCCOPY,\nTRANSPARENT,",
        "double-buffer/font/icon GDI imports",
    );
    patch_once(
        &mut overlay,
        "settings_hwnd: HWND,\nhover_text: Option<String>,",
        "settings_hwnd: HWND,\nfont: HFONT,\nhover_text: Option<String>,",
        "overlay font state",
    );
    patch_once(
        &mut overlay,
        "struct DetailState {\nrow: DpsRow,\nscroll: usize,\n}",
        "struct DetailState {\nrow: DpsRow,\nscroll: usize,\nfont: HFONT,\n}",
        "detail font state",
    );
    patch_once(
        &mut overlay,
        "detail_hwnd:null_mut(),detail_uid:0,settings_hwnd:null_mut(),hover_text:None,hover_x:0,hover_y:0}",
        "detail_hwnd:null_mut(),detail_uid:0,settings_hwnd:null_mut(),font:create_overlay_font(kind==Kind::Dps),hover_text:None,hover_x:0,hover_y:0}",
        "create overlay font",
    );
    patch_once(
        &mut overlay,
        "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){with_state(hwnd,|state|{if snapshot.encounter_ms<state.dps.encounter_ms&&!state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}if !state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0&&state.detail_uid!=0{if let Some(row)=snapshot.rows.iter().find(|row|row.uid==state.detail_uid).cloned(){update_detail(state.detail_hwnd,row);}}state.dps=snapshot;state.scroll=state.scroll.min(state.dps.rows.len().saturating_sub(1));});InvalidateRect(hwnd,null(),0);}",
        "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){with_state(hwnd,|state|{if snapshot.encounter_ms<state.dps.encounter_ms&&!state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}if !state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0&&state.detail_uid!=0{if let Some(row)=snapshot.rows.iter().find(|row|row.uid==state.detail_uid).cloned(){update_detail(state.detail_hwnd,row);}}state.dps=snapshot;state.scroll=state.scroll.min(state.dps.rows.len().saturating_sub(1));});}",
        "DPS snapshot does not force extra repaint",
    );
    patch_once(
        &mut overlay,
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|state.mechanics=snapshot);InvalidateRect(hwnd,null(),0);}",
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|state.mechanics=snapshot);}",
        "mechanics snapshot does not force extra repaint",
    );
    replace_between(
        &mut overlay,
        "unsafe fn paint(hwnd:HWND,state:&mut State){",
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){",
        r#"unsafe fn paint(hwnd:HWND,state:&mut State){
let mut ps:PAINTSTRUCT=std::mem::zeroed();let screen=BeginPaint(hwnd,&mut ps);if screen.is_null(){return;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let width=rc.right.max(1);let height=rc.bottom.max(1);let mem=CreateCompatibleDC(screen);let bitmap=if !mem.is_null(){CreateCompatibleBitmap(screen,width,height)}else{null_mut()};let old_bitmap=if !mem.is_null()&&!bitmap.is_null(){SelectObject(mem,bitmap)}else{null_mut()};let hdc=if !mem.is_null()&&!bitmap.is_null(){mem}else{screen};fill(hdc,&rc,rgb(18,22,27));let font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};SelectObject(hdc,font);SetBkMode(hdc,TRANSPARENT as i32);if state.collapsed{SetTextColor(hdc,rgb(99,199,255));draw(hdc,if state.kind==Kind::Dps{"D"}else{"M"},rc,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}else{paint_toolbar(hdc,rc,state);match state.kind{Kind::Dps=>paint_dps(hdc,rc,state),Kind::Mechanics=>paint_mechanics(hdc,rc,state)}paint_hover(hdc,rc,state);}if hdc==mem{BitBlt(screen,0,0,width,height,mem,0,0,SRCCOPY);SelectObject(mem,old_bitmap);DeleteObject(bitmap);DeleteDC(mem);}EndPaint(hwnd,&ps);}
"#,
        "double buffered overlay paint",
    );
    patch_once(
        &mut overlay,
        "let ptr=Box::into_raw(Box::new(DetailState{row:row.clone(),scroll:0}));",
        "let ptr=Box::into_raw(Box::new(DetailState{row:row.clone(),scroll:0,font:create_overlay_font(true)}));",
        "bold detail font",
    );
    patch_once(
        &mut overlay,
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){let state=&mut*ptr;for child in[state.detail_hwnd,state.settings_hwnd]{if !child.is_null()&&IsWindow(child)!=0{DestroyWindow(child);}}drop(Box::from_raw(ptr));}",
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){let state=&mut*ptr;for child in[state.detail_hwnd,state.settings_hwnd]{if !child.is_null()&&IsWindow(child)!=0{DestroyWindow(child);}}if !state.font.is_null(){DeleteObject(state.font);}drop(Box::from_raw(ptr));}",
        "delete overlay font",
    );
    patch_once(
        &mut overlay,
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},_=>DefWindowProcW(hwnd,msg,wparam,lparam)}}\nunsafe fn paint_detail",
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){if !(*ptr).font.is_null(){DeleteObject((*ptr).font);}drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},_=>DefWindowProcW(hwnd,msg,wparam,lparam)}}\nunsafe fn paint_detail",
        "delete detail font",
    );
    patch_once(
        &mut overlay,
        "fill(hdc,&rc,rgb(18,22,27));SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetBkMode(hdc,TRANSPARENT as i32);paint_popup_toolbar(hdc,rc,&format!(\"Skill Distribution — {}\",state.row.name));",
        "fill(hdc,&rc,rgb(18,22,27));let font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};SelectObject(hdc,font);SetBkMode(hdc,TRANSPARENT as i32);paint_popup_toolbar(hdc,rc,&format!(\"Skill Distribution — {}\",state.row.name));",
        "use bold font in DPS detail",
    );
    replace_between(
        &mut overlay,
        "unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){",
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        r#"unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){let r=RECT{left:x,top:y,right:x+BADGE_W,bottom:y+23};if !draw_imagine_asset(hdc,r,badge.skill_id){fill(hdc,&r,badge_color(&badge.icon_key));SetTextColor(hdc,rgb(248,250,252));let short=if badge.icon_key.trim().is_empty()||badge.icon_key.eq_ignore_ascii_case("BI"){"BI"}else{badge.icon_key.as_str()};draw(hdc,short,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if badge.tier>0{let tier=RECT{left:r.right-11,top:r.bottom-11,right:r.right,bottom:r.bottom};fill(hdc,&tier,rgb(8,11,15));SetTextColor(hdc,rgb(255,255,255));draw(hdc,&badge.tier.to_string(),tier,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}
fn imagine_asset(skill_id:i32)->Option<&'static [u8]>{match skill_id{3903=>Some(include_bytes!(concat!(env!("OUT_DIR"),"/imagine_3903.bmp"))),3920=>Some(include_bytes!(concat!(env!("OUT_DIR"),"/imagine_3920.bmp"))),3921=>Some(include_bytes!(concat!(env!("OUT_DIR"),"/imagine_3921.bmp"))),3944=>Some(include_bytes!(concat!(env!("OUT_DIR"),"/imagine_3944.bmp"))),_=>None}}
unsafe fn draw_imagine_asset(hdc:HDC,r:RECT,skill_id:i32)->bool{let Some(bytes)=imagine_asset(skill_id)else{return false;};if bytes.len()<54||&bytes[0..2]!=b"BM"{return false;}let offset=u32::from_le_bytes([bytes[10],bytes[11],bytes[12],bytes[13]])as usize;let width=i32::from_le_bytes([bytes[18],bytes[19],bytes[20],bytes[21]]);let height=i32::from_le_bytes([bytes[22],bytes[23],bytes[24],bytes[25]]);let bpp=u16::from_le_bytes([bytes[28],bytes[29]]);if offset>=bytes.len()||width<=0||height==0||!matches!(bpp,24|32){return false;}let mut info:BITMAPINFO=std::mem::zeroed();info.bmiHeader.biSize=std::mem::size_of_val(&info.bmiHeader)as u32;info.bmiHeader.biWidth=width;info.bmiHeader.biHeight=height;info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=bpp;let result=StretchDIBits(hdc,r.left,r.top,r.right-r.left,r.bottom-r.top,0,0,width,height.abs(),bytes[offset..].as_ptr().cast(),&info,DIB_RGB_COLORS,SRCCOPY);result!=0}
fn attr_color(id:i32)->u32{match id{feature_settings::ATTR_CRIT=>rgb(255,112,96),feature_settings::ATTR_LUCKY=>rgb(255,205,86),feature_settings::ATTR_HASTE=>rgb(92,205,255),feature_settings::ATTR_MASTERY=>rgb(98,225,170),feature_settings::ATTR_VERSATILITY=>rgb(150,185,255),feature_settings::ATTR_FIGHT_POINT=>rgb(255,222,125),feature_settings::ATTR_SEASON_STRENGTH=>rgb(255,170,92),feature_settings::ATTR_CURRENT_HP|feature_settings::ATTR_MAX_HP=>rgb(105,225,145),_=>rgb(218,227,238)}}
unsafe fn create_overlay_font(bold:bool)->HFONT{let face=wide("Segoe UI");CreateFontW(-14,0,0,0,if bold{700}else{400},0,0,0,1,0,0,5,0,face.as_ptr())}
"#,
        "game Imagine assets and attribute colors",
    );
    patch_once(
        &mut overlay,
        "SetTextColor(hdc,rgb(218,227,238));draw(hdc,&format!(\"{} {}\",attr.label,format_attr_value(attr.attr_id, attr.value))",
        "SetTextColor(hdc,attr_color(attr.attr_id));draw(hdc,&format!(\"{} {}\",attr.label.trim_end_matches(\" %\"),format_attr_value(attr.attr_id, attr.value))",
        "color tracked attributes",
    );
    fs::write(&overlay_path, overlay).expect("write v1.8.2 overlay");

    let mut win = fs::read_to_string("src/win_v160.rs").expect("read win_v160.rs").replace("\r\n", "\n");
    patch_once(&mut win, "SetTimer(hwnd,TIMER_ID,50,None);", "SetTimer(hwnd,TIMER_ID,33,None);", "30fps UI timer");
    patch_once(
        &mut win,
        "feature_overlays::tick((*ptr).mechanics_overlay);",
        "feature_overlays::tick((*ptr).dps_overlay);feature_overlays::tick((*ptr).mechanics_overlay);",
        "fixed-rate DPS and mechanics repaint",
    );
    fs::write(out.join("win_v182_fixed.rs"), win).expect("write v1.8.2 win source");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_v182.rs");
    println!("cargo:rerun-if-changed=src/win_v160.rs");
}
