use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

mod previous {
    include!("build_v182.rs");
    pub fn run() { main(); }
}

const CN_COMMIT: &str = "bd71d2dfd3c7289e6398c4cd042f4357d4f35721";

#[derive(Clone, Debug)]
struct ImagineEntry {
    id: i32,
    name: String,
    icon: String,
    fallback_key: String,
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.8.3 patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..]
        .find(end)
        .unwrap_or_else(|| panic!("v1.8.3 patch `{label}` end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn ps_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn run_powershell(script: &str, label: &str) {
    // Windows CreateProcess has a small command-line limit. The complete CN
    // icon catalog produces a long script, so always execute through a .ps1.
    let script_path = env::temp_dir().join(format!("bpsr-readyalert-build-{}.ps1", std::process::id()));
    fs::write(&script_path, script).unwrap_or_else(|err| panic!("write PowerShell script for {label}: {err}"));
    let status = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .status()
        .unwrap_or_else(|err| panic!("failed to launch PowerShell for {label}: {err}"));
    let _ = fs::remove_file(&script_path);
    assert!(status.success(), "PowerShell failed while {label}");
}

fn load_pinned_json(cache: &Path, repo_path: &str) -> serde_json::Value {
    let file_name = repo_path.rsplit('/').next().expect("CN json filename");
    let local = cache.join(file_name);
    let url = format!("https://raw.githubusercontent.com/fudiyangjin/resonance-logs-cn/{CN_COMMIT}/{repo_path}");
    for attempt in 0..2 {
        if !local.exists() {
            let temp = local.with_extension("download");
            let script = format!(
                "$ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'; Move-Item -Force '{}' '{}'",
                url, ps_quote(&temp), ps_quote(&temp), ps_quote(&local)
            );
            run_powershell(&script, &format!("downloading {file_name}"));
        }
        if let Ok(text) = fs::read_to_string(&local) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) { return value; }
        }
        if attempt == 0 { let _ = fs::remove_file(&local); } else { panic!("pinned CN JSON {file_name} could not be parsed"); }
    }
    unreachable!()
}

fn cleaned_monster_name(name: &str) -> String {
    name.strip_suffix(" - Resonance")
        .or_else(|| name.strip_suffix("- Resonance"))
        .unwrap_or(name)
        .trim()
        .to_string()
}

fn fallback_key(name: &str) -> String {
    let mut out = String::new();
    for word in name.split(|c: char| !c.is_alphanumeric()) {
        if let Some(ch) = word.chars().next().filter(|c| c.is_ascii_alphanumeric()) {
            out.push(ch.to_ascii_uppercase());
            if out.len() == 2 { break; }
        }
    }
    if out.is_empty() { "BI".into() } else { out }
}

fn build_catalog(cache: &Path) -> (Vec<ImagineEntry>, Vec<(i32, i32)>) {
    let skills = load_pinned_json(cache, "src/lib/config/en-US/skill_aoyi_icons.json");
    let fantasy = load_pinned_json(cache, "src-tauri/meter-data/FantasyMonsterSkillMap.json");
    let monster_names = load_pinned_json(cache, "src/lib/config/en-US/MonsterIdNameType.json");
    let fantasy_obj = fantasy.as_object().expect("FantasyMonsterSkillMap object");
    let names_obj = monster_names.as_object().expect("MonsterIdNameType object");

    let mut monster_skill: Vec<(i32, i32)> = fantasy_obj.iter().filter_map(|(monster, skill)| {
        Some((monster.parse::<i32>().ok()?, i32::try_from(skill.as_i64()?).ok()?))
    }).collect();
    monster_skill.sort_unstable();

    let mut display_name_by_skill = BTreeMap::<i32, String>::new();
    for (monster_id, skill_id) in &monster_skill {
        if display_name_by_skill.contains_key(skill_id) { continue; }
        let Some(name) = names_obj.get(&monster_id.to_string())
            .and_then(|value| value.get("Name"))
            .and_then(|value| value.as_str()) else { continue; };
        let cleaned = cleaned_monster_name(name);
        if !cleaned.is_empty() { display_name_by_skill.insert(*skill_id, cleaned); }
    }

    let mut catalog = Vec::new();
    for value in skills.as_array().expect("skill_aoyi_icons array") {
        let Some(id) = value.get("id").and_then(|v| v.as_i64()).and_then(|v| i32::try_from(v).ok()) else { continue; };
        if !(3_898..=4_100).contains(&id) { continue; }
        let Some(icon) = value.get("Icon").and_then(|v| v.as_str()) else { continue; };
        if !icon.starts_with("skill_aoyi_skill_icon_") || !icon.ends_with(".png")
            || !icon.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.')) {
            panic!("unexpected CN Imagine icon filename for {id}: {icon}");
        }
        let skill_name = value.get("NameDesign").and_then(|v| v.as_str()).unwrap_or("Battle Imagine");
        let name = display_name_by_skill.get(&id).cloned().unwrap_or_else(|| skill_name.to_string());
        catalog.push(ImagineEntry { id, fallback_key: fallback_key(&name), name, icon: icon.to_string() });
    }
    catalog.sort_by_key(|entry| entry.id);
    assert!(catalog.len() >= 80, "CN Imagine catalog unexpectedly small: {}", catalog.len());
    assert!(catalog.iter().any(|entry| entry.id == 3938), "CN Imagine catalog lost skill 3938");
    assert!(monster_skill.iter().any(|pair| *pair == (3_000_023, 3938)), "CN monster map lost 3000023 -> 3938");
    (catalog, monster_skill)
}

fn prepare_icons(cache: &Path, out: &Path, catalog: &[ImagineEntry]) {
    if !cfg!(windows) { return; }
    let icons: BTreeSet<String> = catalog.iter().map(|entry| entry.icon.clone()).collect();
    let mut item_rows = Vec::with_capacity(icons.len());
    for icon in &icons {
        let png = cache.join(icon);
        let bmp_name = format!("{}.bmp", icon.trim_end_matches(".png"));
        let bmp = cache.join(&bmp_name);
        let url = format!("https://raw.githubusercontent.com/fudiyangjin/resonance-logs-cn/{CN_COMMIT}/static/images/resonance_skill/{icon}");
        item_rows.push(format!("[pscustomobject]@{{Url='{}';Png='{}';Bmp='{}'}}", url, ps_quote(&png), ps_quote(&bmp)));
    }
    let items = item_rows.join(",\n");

    let script = format!(r#"
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$items=@(
{items}
)
foreach($item in $items) {{
  $valid=$false
  if(Test-Path $item.Bmp) {{
    $b=[IO.File]::ReadAllBytes($item.Bmp)
    $valid=($b.Length -ge 54 -and $b[0] -eq 0x42 -and $b[1] -eq 0x4d)
  }}
  if($valid) {{ continue }}
  if(!(Test-Path $item.Png)) {{ Invoke-WebRequest -UseBasicParsing -Uri $item.Url -OutFile $item.Png }}
  $p=[IO.File]::ReadAllBytes($item.Png)
  if($p.Length -lt 24 -or $p[0] -ne 0x89 -or $p[1] -ne 0x50 -or $p[2] -ne 0x4e -or $p[3] -ne 0x47) {{ throw "Invalid CN Imagine PNG: $($item.Png)" }}
  $src=[System.Drawing.Image]::FromFile($item.Png)
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
      $dst.Save($item.Bmp,[System.Drawing.Imaging.ImageFormat]::Bmp)
    }} finally {{ $dst.Dispose() }}
  }} finally {{ $src.Dispose() }}
}}
"#);
    run_powershell(&script, "preparing complete CN Imagine icon set");

    for icon in icons {
        let bmp_name = format!("{}.bmp", icon.trim_end_matches(".png"));
        let bytes = fs::read(cache.join(&bmp_name)).unwrap_or_else(|err| panic!("read cached {bmp_name}: {err}"));
        assert!(bytes.len() >= 54 && &bytes[0..2] == b"BM", "invalid cached Imagine BMP {bmp_name}");
        fs::write(out.join(&bmp_name), bytes).unwrap_or_else(|err| panic!("write OUT_DIR {bmp_name}: {err}"));
    }
}

fn rust_string(value: &str) -> String { format!("{value:?}") }

fn telemetry_sources(catalog: &[ImagineEntry], monster_skill: &[(i32, i32)]) -> (String, String, String) {
    let known_ids = catalog.iter().map(|entry| entry.id.to_string()).collect::<Vec<_>>().join("|");
    let known = format!("fn is_fantasy_skill(id:i32)->bool{{matches!(id,{known_ids})}}\n\n");
    let monster_arms = monster_skill.iter().map(|(monster, skill)| format!("{monster}=>{skill},")).collect::<String>();
    let monster = format!("fn fantasy_skill_for_monster(monster_id:i32)->Option<i32>{{Some(match monster_id{{{monster_arms}_=>return None,}})}}\n\n");
    let info_arms = catalog.iter().map(|entry| {
        format!("{}=>({}.into(),{}.into()),", entry.id, rust_string(&entry.name), rust_string(&entry.fallback_key))
    }).collect::<String>();
    let info = format!("fn imagine_info(id:i32)->(String,String){{match id{{{info_arms}_=>(format!(\"Battle Imagine {{id}}\"),\"BI\".into()),}}}}\n\n");
    (known, monster, info)
}

fn overlay_asset_source(catalog: &[ImagineEntry]) -> String {
    let arms = catalog.iter().map(|entry| {
        let bmp = format!("{}.bmp", entry.icon.trim_end_matches(".png"));
        format!("{}=>Some(include_bytes!(concat!(env!(\"OUT_DIR\"),\"/{bmp}\"))),", entry.id)
    }).collect::<String>();
    format!("fn imagine_asset(skill_id:i32)->Option<&'static [u8]>{{match skill_id{{{arms}_=>None}}}}\n")
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let cache = env::temp_dir().join(format!("BPSR-ReadyAlert-CN-Imagines-{CN_COMMIT}"));
    fs::create_dir_all(&cache).expect("create pinned CN Imagine cache");
    let (catalog, monster_skill) = build_catalog(&cache);
    prepare_icons(&cache, &out, &catalog);

    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read v1.8.2 telemetry");
    let (known_source, monster_source, info_source) = telemetry_sources(&catalog, &monster_skill);
    replace_between(&mut telemetry, "fn is_fantasy_skill(id: i32) -> bool {", "/// CN FantasyMonsterSkillMap.json fallback", &known_source, "complete Imagine skill recognition");
    replace_between(&mut telemetry, "fn fantasy_skill_for_monster(monster_id: i32) -> Option<i32> {", "fn imagine_info(id: i32) -> (String, String) {", &monster_source, "CN-generated monster to Imagine map");
    replace_between(&mut telemetry, "fn imagine_info(id: i32) -> (String, String) {", "/// Frequently observed player skill names", &info_source, "complete Imagine hover metadata");
    fs::write(&telemetry_path, telemetry).expect("write v1.8.3 telemetry");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read v1.8.2 overlay");
    replace_between(&mut overlay, "fn imagine_asset(skill_id:i32)->Option<&'static [u8]>{", "unsafe fn draw_imagine_asset", &overlay_asset_source(&catalog), "complete Imagine icon assets");

    replace_between(
        &mut overlay,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        "unsafe fn paint_hover",
        r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row.imagines.len());let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){if x>=bx&&x<bx+BADGE_W&&y>=r.top+4&&y<r.top+28{let tier=if badge.tier>0{badge.tier.to_string()}else{"?".into()};return Some(format!("{} · Tier {}",badge.name,tier));}bx+=BADGE_W+BADGE_GAP;}None}
"#,
        "Imagine hover follows grouped row layout",
    );

    replace_between(
        &mut overlay,
        "unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){",
        "unsafe fn draw_percent",
        r#"#[derive(Clone,Copy)]struct DpsRowLayout{identity_left:i32,identity_right:i32,badge_left:i32,score_left:i32,left_right:i32,middle_left:i32,right_left:i32,middle_col:i32,right_col:i32}
fn dps_row_layout(r:RECT,show_imagines:bool,imagine_count:usize)->DpsRowLayout{let total=(r.right-r.left).max(1);let right_w=if total<600{138}else{150};let middle_w=if total<600{150}else{170};let gap=5;let right_left=r.right-right_w;let middle_left=right_left-gap-middle_w;let left_right=middle_left-gap;let identity_left=r.left+30;let score_w=if total<600{78}else{92};let badge_count=if show_imagines{imagine_count.min(2)as i32}else{0};let badge_span=badge_count*(BADGE_W+BADGE_GAP);let score_left=(left_right-score_w).max(identity_left+50);let badge_left=(score_left-badge_span).max(identity_left+48);let identity_right=(badge_left-4).max(identity_left+45);DpsRowLayout{identity_left,identity_right,badge_left,score_left,left_right,middle_left,right_left,middle_col:(middle_w/2).max(1),right_col:(right_w/3).max(1)}}
fn dps_identity(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if spec.trim().is_empty(){row.name.clone()}else{format!("{}-{}",row.name,spec)}}
fn dps_score_pair(row:&DpsRow)->String{match(row.ability_score>0,row.illusion_break>0){(true,true)=>format!("({} + {})",score(row.ability_score),score(row.illusion_break)),(true,false)=>format!("({})",score(row.ability_score)),(false,true)=>format!("(+ {})",score(row.illusion_break)),_=>String::new()}}
fn rate(total:i64,encounter_ms:u64)->f64{if total<=0{0.0}else{let seconds=(encounter_ms.max(1)as f64/1000.0).max(0.001);total as f64/seconds}}
fn mode_values(row:&DpsRow,mode:SortMode,encounter_ms:u64)->(String,String){match mode{SortMode::Damage=>(compact(row.damage as f64),format!("{}/s",compact(row.dps))),SortMode::Heal=>(compact(row.healing as f64),format!("{}/s",compact(rate(row.healing,encounter_ms)))),SortMode::Tank=>(compact(row.damage_taken as f64),format!("{}/s",compact(rate(row.damage_taken,encounter_ms))))}}
unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();let control_top=TOOLBAR_H+4;paint_tab(hdc,8,control_top,70,"Damage",state.sort_mode==SortMode::Damage);paint_tab(hdc,83,control_top,65,"Heal",state.sort_mode==SortMode::Heal);paint_tab(hdc,153,control_top,65,"Tank",state.sort_mode==SortMode::Tank);paint_tab(hdc,225,control_top,73,"Reset",false);SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("Total {}   Heal {}   Taken {}",compact(state.dps.total_damage as f64),compact(state.dps.total_healing as f64),compact(state.dps.total_damage_taken as f64)),RECT{left:304,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let rows=sorted_rows(&state.dps,state.sort_mode);let top=dps_rows_top();let visible=visible_dps_rows(rc.bottom);if rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"Waiting for party / combat data...",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}for(screen_i,row)in rows.iter().skip(state.scroll).take(visible).enumerate(){let rank=state.scroll+screen_i+1;let y=top+screen_i as i32*DPS_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=spec_color(row);fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base_text=text_on(bg);let layout=dps_row_layout(r,settings.meter.show_imagines,row.imagines.len());SetTextColor(hdc,base_text);draw(hdc,&format!("{rank}."),RECT{left:r.left+4,top:r.top,right:r.left+28,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,if row.is_dead{rgb(255,45,45)}else{base_text});draw(hdc,&dps_identity(row),RECT{left:layout.identity_left,top:r.top,right:layout.identity_right,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if settings.meter.show_imagines{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){paint_badge(hdc,bx,r.top+4,badge);bx+=BADGE_W+BADGE_GAP;}}SetTextColor(hdc,base_text);draw(hdc,&dps_score_pair(row),RECT{left:layout.score_left,top:r.top,right:layout.left_right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);fill(hdc,&RECT{left:layout.left_right+2,top:r.top+5,right:layout.left_right+3,bottom:r.bottom-5},rgb(30,35,42));let(first,second)=mode_values(row,state.sort_mode,state.dps.encounter_ms);SetTextColor(hdc,base_text);draw(hdc,&first,RECT{left:layout.middle_left,top:r.top,right:layout.middle_left+layout.middle_col-3,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&second,RECT{left:layout.middle_left+layout.middle_col+2,top:r.top,right:layout.right_left-3,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);fill(hdc,&RECT{left:layout.right_left-2,top:r.top+5,right:layout.right_left-1,bottom:r.bottom-5},rgb(30,35,42));let mut share_x=layout.right_left;if settings.meter.show_damage_share{SetTextColor(hdc,rgb(255,70,70));draw_percent_width(hdc,row.damage_share,share_x,r,layout.right_col);}share_x+=layout.right_col;if settings.meter.show_healing_share{SetTextColor(hdc,rgb(40,215,100));draw_percent_width(hdc,row.healing_share,share_x,r,layout.right_col);}share_x+=layout.right_col;if settings.meter.show_tank_share{SetTextColor(hdc,rgb(35,145,255));draw_percent_width(hdc,row.tank_share,share_x,r,layout.right_col);}if settings.meter.show_deaths&&row.deaths>0{SetTextColor(hdc,rgb(255,45,45));draw(hdc,&format!("D{}",row.deaths),RECT{left:r.right-30,top:r.top,right:r.right-2,bottom:r.top+12},DT_RIGHT|DT_SINGLELINE|DT_NOPREFIX);}}paint_scrollbar(hdc,rc,rows.len(),visible,state.scroll,top);}
unsafe fn draw_percent_width(hdc:HDC,value:f64,x:i32,r:RECT,width:i32){draw(hdc,&format!("{value:.1}%"),RECT{left:x,top:r.top,right:(x+width-2).min(r.right),bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
"#,
        "three-group DPS row layout",
    );

    fs::write(&overlay_path, overlay).expect("write v1.8.3 grouped DPS overlay");
    println!("cargo:rerun-if-changed=build_v183.rs");
}
