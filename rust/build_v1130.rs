use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1120.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.13.0 patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // ---- Training / sharing controls --------------------------------------------
    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read generated v1.12 overlay");

    replace_once(
        &mut overlay,
        "history::{self, HistoryEncounter},\n};",
        "history::{self, HistoryEncounter},\nsharing::{self, ExportFormat, ViewMode},\n};",
        "sharing imports",
    );
    replace_once(
        &mut overlay,
        "history_index: Option<usize>,\nhover_text: Option<String>,",
        "history_index: Option<usize>,\nshare_notice: Option<(String, i64)>,\nhover_text: Option<String>,",
        "sharing notice state",
    );
    replace_once(
        &mut overlay,
        "history:history_records,history_index:None,hover_text:None,hover_x:0,hover_y:0}",
        "history:history_records,history_index:None,share_notice:None,hover_text:None,hover_x:0,hover_y:0}",
        "sharing notice initialization",
    );

    replace_once(
        &mut overlay,
        "fn history_newer(state:&mut State){state.history_index=match state.history_index{None=>None,Some(0)=>None,Some(index)=>Some(index-1)};state.scroll=0;}",
        r#"fn history_newer(state:&mut State){state.history_index=match state.history_index{None=>None,Some(0)=>None,Some(index)=>Some(index-1)};state.scroll=0;}
fn share_view_mode(mode:SortMode)->ViewMode{match mode{SortMode::Damage=>ViewMode::Damage,SortMode::Heal=>ViewMode::Heal,SortMode::Tank=>ViewMode::Tank}}
fn set_share_notice(state:&mut State,text:String){state.share_notice=Some((text,now_ms().saturating_add(3000)));}
fn copy_view(state:&mut State){let mode=share_view_mode(state.sort_mode);let snapshot=view_snapshot(state).clone();let text=sharing::encounter_summary(&snapshot,mode);match sharing::copy_text(&text){Ok(())=>set_share_notice(state,format!("Copied {} summary",mode.label())),Err(err)=>{crate::logging::write(format!("sharing: clipboard failed: {err}"));set_share_notice(state,"Copy failed".into());}}}
fn export_view(state:&mut State,format:ExportFormat){let snapshot=view_snapshot(state).clone();match sharing::export_encounter(&state.paths.root,&snapshot,format){Ok(path)=>{let name=path.file_name().and_then(|x|x.to_str()).unwrap_or("export").to_string();crate::logging::write(format!("sharing: exported {}",path.display()));set_share_notice(state,format!("Saved {name}"));},Err(err)=>{crate::logging::write(format!("sharing: export failed: {err}"));set_share_notice(state,"Export failed".into());}}}
fn personal_best_text(state:&State)->Option<String>{let mode=share_view_mode(state.sort_mode);if mode==ViewMode::Tank{return None;}let snapshot=view_snapshot(state);let exclude=state.history_index.and_then(|index|state.history.get(index).map(|record|record.id));let pb=sharing::personal_best(&state.history,snapshot,exclude,mode)?;if pb.is_new_record{Some(format!("NEW PB {}  ({:+.1}%)",compact(pb.current_rate),pb.delta_percent))}else{Some(format!("PB {}  ({:+.1}% | {} runs)",compact(pb.previous_best_rate),pb.delta_percent,pb.samples))}}
"#,
        "sharing actions and PB comparison",
    );

    replace_once(
        &mut overlay,
        "400..=432=>history_newer(state),_=>{}",
        "400..=432=>history_newer(state),438..=491=>copy_view(state),496..=540=>export_view(state,ExportFormat::Csv),545..=593=>export_view(state,ExportFormat::Json),_=>{}",
        "sharing control click arms",
    );

    replace_once(
        &mut overlay,
        r#"fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!("Dungeon Mechanics  |  {}",state.capture_status);}let snapshot=view_snapshot(state);let label=match state.history_index{Some(index)=>format!("History {}/{}",index+1,state.history.len()),None=>"DPS Meter".into()};let show_target=state.features.read().map(|f|f.meter.show_target).unwrap_or(true);if !show_target{return format!("{}  {}",label,format_time(snapshot.encounter_ms));}let Some(target)=snapshot.target.as_ref()else{return format!("{}  {}",label,format_time(snapshot.encounter_ms));};let hp=if target.max_hp>0{format!("{:.1}%",(target.hp.max(0)as f64*100.0/target.max_hp as f64).clamp(0.0,100.0))}else{"?%".into()};format!("{}  {}  |  {} {}",label,format_time(snapshot.encounter_ms),target.name,hp)}"#,
        r#"fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!("Dungeon Mechanics  |  {}",state.capture_status);}if let Some((notice,until))=state.share_notice.as_ref(){if *until>now_ms(){return format!("DPS Meter  |  {notice}");}}let snapshot=view_snapshot(state);let label=match state.history_index{Some(index)=>format!("History {}/{}",index+1,state.history.len()),None=>"DPS Meter".into()};let show_target=state.features.read().map(|f|f.meter.show_target).unwrap_or(true);let mut title=if !show_target{format!("{}  {}",label,format_time(snapshot.encounter_ms))}else if let Some(target)=snapshot.target.as_ref(){let hp=if target.max_hp>0{format!("{:.1}%",(target.hp.max(0)as f64*100.0/target.max_hp as f64).clamp(0.0,100.0))}else{"?%".into()};format!("{}  {}  |  {} {}",label,format_time(snapshot.encounter_ms),target.name,hp)}else{format!("{}  {}",label,format_time(snapshot.encounter_ms))};if let Some(pb)=personal_best_text(state){title.push_str("  |  ");title.push_str(&pb);}title}"#,
        "PB and sharing toolbar feedback",
    );

    replace_once(
        &mut overlay,
        r#"paint_tab(hdc,400,control_top,32,">",false);SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("D {}  H {}  T {}",compact(snapshot.total_damage as f64),compact(snapshot.total_healing as f64),compact(snapshot.total_damage_taken as f64)),RECT{left:438,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);"#,
        r#"paint_tab(hdc,400,control_top,32,">",false);paint_tab(hdc,438,control_top,54,"Copy",false);paint_tab(hdc,496,control_top,45,"CSV",false);paint_tab(hdc,545,control_top,48,"JSON",false);if rc.right>=760{SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("D {}  H {}  T {}",compact(snapshot.total_damage as f64),compact(snapshot.total_healing as f64),compact(snapshot.total_damage_taken as f64)),RECT{left:599,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        "copy and export toolbar controls",
    );

    fs::write(&overlay_path, overlay).expect("write v1.13 generated overlay");

    // ---- Global combat-overlay hotkey -------------------------------------------
    // Ctrl+Shift+F10 hides/shows DPS + Mechanics together. Chat deliberately
    // remains independent so the shortcut is useful during gameplay/screenshots.
    let win_path = out.join("win_v182_fixed.rs");
    let mut win = fs::read_to_string(&win_path).expect("read generated v1.12 win source");
    replace_once(
        &mut win,
        "PostQuitMessage,RegisterClassW,SetForegroundWindow,SetTimer,SetWindowLongPtrW,ShowWindow,TranslateMessage,",
        "PostQuitMessage,RegisterClassW,RegisterHotKey,SetForegroundWindow,SetTimer,SetWindowLongPtrW,ShowWindow,TranslateMessage,UnregisterHotKey,",
        "hotkey function imports",
    );
    replace_once(
        &mut win,
        "WM_COMMAND,WM_DESTROY,WM_LBUTTONDBLCLK,WM_NCCREATE,WM_RBUTTONUP,WM_TIMER,",
        "WM_COMMAND,WM_DESTROY,WM_HOTKEY,WM_LBUTTONDBLCLK,WM_NCCREATE,WM_RBUTTONUP,WM_TIMER,",
        "hotkey message import",
    );
    replace_once(
        &mut win,
        "const WM_TRAY:u32=WM_APP+17;const TRAY_ID:u32=1;const TIMER_ID:usize=1;",
        "const WM_TRAY:u32=WM_APP+17;const TRAY_ID:u32=1;const TIMER_ID:usize=1;const HOTKEY_COMBAT_ID:i32=1130;const MOD_CONTROL_:u32=0x0002;const MOD_SHIFT_:u32=0x0004;const VK_F10_:u32=0x79;",
        "combat hotkey constants",
    );
    replace_once(
        &mut win,
        "if hwnd.is_null(){return Err(format!(\"CreateWindowExW(main) failed: {}\",GetLastError()));}",
        "if hwnd.is_null(){return Err(format!(\"CreateWindowExW(main) failed: {}\",GetLastError()));}if RegisterHotKey(hwnd,HOTKEY_COMBAT_ID,MOD_CONTROL_|MOD_SHIFT_,VK_F10_)==0{logging::write(format!(\"hotkey: Ctrl+Shift+F10 unavailable error={}\",GetLastError()));}else{logging::write(\"hotkey: Ctrl+Shift+F10 toggles combat overlays\");}",
        "register combat hotkey",
    );
    replace_once(
        &mut win,
        "WM_COMMAND=>{if !ptr.is_null(){handle_command(hwnd,&mut*ptr,(wparam as u32)&0xffff,lparam);}0}",
        "WM_HOTKEY if wparam as i32==HOTKEY_COMBAT_ID=>{if !ptr.is_null(){toggle_combat_overlays(&mut*ptr);}0}\n        WM_COMMAND=>{if !ptr.is_null(){handle_command(hwnd,&mut*ptr,(wparam as u32)&0xffff,lparam);}0}",
        "handle combat hotkey",
    );
    replace_once(
        &mut win,
        "remove_tray(hwnd);PostQuitMessage(0);0}",
        "let _=UnregisterHotKey(hwnd,HOTKEY_COMBAT_ID);remove_tray(hwnd);PostQuitMessage(0);0}",
        "unregister combat hotkey",
    );
    replace_once(
        &mut win,
        "fn select_adapter(state:&UiState,device:Option<String>){",
        "unsafe fn toggle_combat_overlays(state:&mut UiState){let hide=state.features.read().map(|f|f.dps_overlay_enabled||f.mechanics_overlay_enabled).unwrap_or(true);update_features(state,|f|{f.dps_overlay_enabled=!hide;f.mechanics_overlay_enabled=!hide;});apply_visibility(state);logging::write(if hide{\"hotkey: combat overlays hidden\"}else{\"hotkey: combat overlays shown\"});}\n\nfn select_adapter(state:&UiState,device:Option<String>){",
        "combat hotkey toggle helper",
    );
    fs::write(&win_path, win).expect("write v1.13 generated win source");

    println!("cargo:rerun-if-changed=build_v1130.rs");
    println!("cargo:rerun-if-changed=src/sharing.rs");
}
