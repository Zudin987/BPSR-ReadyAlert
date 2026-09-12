use std::{fs, path::Path};
fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"compact meter patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
pub fn run(out:&Path){let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read generated feature overlay").replace("\r\n","\n");
replace_once(&mut source,r#"CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
GetWindowRect, IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW,
SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, SetTimer, KillTimer,"#,r#"AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, GetClientRect, GetWindowLongPtrW,
GetWindowRect, IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW,
SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, SetTimer, KillTimer, TrackPopupMenu,"#,"menu imports");
replace_once(&mut source,r#"HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WM_ERASEBKGND, WM_EXITSIZEMOVE,"#,r#"HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
MF_SEPARATOR, MF_STRING, TPM_RETURNCMD, TPM_RIGHTBUTTON,
SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WM_ERASEBKGND, WM_EXITSIZEMOVE,"#,"menu constants");
replace_once(&mut source,r#"fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},clamp_kind_scale(kind,scale))}
"#,r#"fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},clamp_kind_scale(kind,scale))}
const DPS_COMPACT_MIN_WIDTH:i32=300;
const DPS_COMPACT_MIN_HEIGHT:i32=140;
fn overlay_min_width_mode(kind:Kind,scale:i32,compact:bool)->i32{if kind==Kind::Dps&&compact{scale_px(DPS_COMPACT_MIN_WIDTH,clamp_kind_scale(kind,scale))}else{overlay_min_width(kind,scale)}}
fn overlay_min_height_mode(kind:Kind,scale:i32,compact:bool)->i32{if kind==Kind::Dps&&compact{scale_px(DPS_COMPACT_MIN_HEIGHT,clamp_kind_scale(kind,scale))}else{overlay_min_height(kind,scale)}}
"#,"mode minimums");
replace_once(&mut source,r#"fn slot_rect(slot:&OverlayScaleSlot)->Option<RECT>{(slot.percent!=100&&slot.has_bounds).then_some(RECT{left:slot.x,top:slot.y,right:slot.x.saturating_add(slot.width),bottom:slot.y.saturating_add(slot.height)})}
"#,r#"fn slot_rect(slot:&OverlayScaleSlot)->Option<RECT>{(slot.percent!=100&&slot.has_bounds).then_some(RECT{left:slot.x,top:slot.y,right:slot.x.saturating_add(slot.width),bottom:slot.y.saturating_add(slot.height)})}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",default)]
struct MeterViewSize{width:i32,height:i32,scale:i32,valid:bool}
impl Default for MeterViewSize{fn default()->Self{Self{width:0,height:0,scale:100,valid:false}}}
#[derive(Clone,Debug,serde::Serialize,serde::Deserialize,Default)]
#[serde(rename_all="camelCase",default)]
struct MeterViewFile{compact_mode:bool,normal:MeterViewSize,compact:MeterViewSize,raid_normal:MeterViewSize,raid_compact:MeterViewSize}
fn meter_view_path(paths:&AppPaths)->std::path::PathBuf{paths.root.join("dps-meter-view.json")}
fn read_meter_view(paths:&AppPaths)->MeterViewFile{std::fs::read_to_string(meter_view_path(paths)).ok().and_then(|text|serde_json::from_str::<MeterViewFile>(&text).ok()).unwrap_or_default()}
fn write_meter_view(paths:&AppPaths,file:&MeterViewFile){let path=meter_view_path(paths);let tmp=path.with_extension("json.new");match serde_json::to_vec_pretty(file){Ok(bytes)=>{if std::fs::write(&tmp,bytes).is_ok(){let _=std::fs::remove_file(&path);if let Err(err)=std::fs::rename(&tmp,&path){crate::logging::write(format!("meter view: save failed: {err}"));}}},Err(err)=>crate::logging::write(format!("meter view: encode failed: {err}")),}}
fn meter_view_size(file:&MeterViewFile,compact:bool,raid:bool)->&MeterViewSize{match(compact,raid){(false,false)=>&file.normal,(true,false)=>&file.compact,(false,true)=>&file.raid_normal,(true,true)=>&file.raid_compact}}
fn meter_view_size_mut(file:&mut MeterViewFile,compact:bool,raid:bool)->&mut MeterViewSize{match(compact,raid){(false,false)=>&mut file.normal,(true,false)=>&mut file.compact,(false,true)=>&mut file.raid_normal,(true,true)=>&mut file.raid_compact}}
fn restored_meter_size(slot:&MeterViewSize,current_scale:i32)->Option<(i32,i32)>{if !slot.valid||slot.width<=0||slot.height<=0{return None;}let old=slot.scale.clamp(DPS_SCALE_MIN,OVERLAY_SCALE_MAX).max(1);let new=current_scale.clamp(DPS_SCALE_MIN,OVERLAY_SCALE_MAX);Some((rescale_px(slot.width,old,new),rescale_px(slot.height,old,new)))}
fn default_compact_size(scale:i32,raid:bool)->(i32,i32){let logical_w=if raid{760}else{420};let logical_h=if raid{TOOLBAR_H+2+10*24+6}else{TOOLBAR_H+2+7*24+6};(scale_px(logical_w,scale).max(overlay_min_width_mode(Kind::Dps,scale,true)),scale_px(logical_h,scale).max(overlay_min_height_mode(Kind::Dps,scale,true)))}
"#,"meter view persistence");
replace_once(&mut source,r#"scale_percent:i32,
dps: DpsSnapshot,"#,r#"scale_percent:i32,
compact_mode:bool,
dps: DpsSnapshot,"#,"state compact field");
replace_once(&mut source,r#"let snapshot=features.read().map(|x|x.clone()).unwrap_or_default();let layout=if kind==Kind::Dps{snapshot.dps.clone()}else{snapshot.mechanics.clone()};let slot=scale_slot(&paths,kind);let scale_percent=slot.percent;
let stored=slot_rect(&slot);let x=stored.map(|r|r.left).unwrap_or(if layout.x==i32::MIN{CW_USEDEFAULT}else{layout.x});let y=stored.map(|r|r.top).unwrap_or(if layout.y==i32::MIN{CW_USEDEFAULT}else{layout.y});
let mut initial_w=stored.map(|r|r.right-r.left).unwrap_or(layout.width);let mut initial_h=stored.map(|r|r.bottom-r.top).unwrap_or(layout.height);
if stored.is_none()&&scale_percent!=100{initial_w=scale_px(initial_w,scale_percent);initial_h=scale_px(initial_h,scale_percent);}
"#,r#"let snapshot=features.read().map(|x|x.clone()).unwrap_or_default();let layout=if kind==Kind::Dps{snapshot.dps.clone()}else{snapshot.mechanics.clone()};let slot=scale_slot(&paths,kind);let scale_percent=slot.percent;let meter_view=if kind==Kind::Dps{read_meter_view(&paths)}else{MeterViewFile::default()};let compact_mode=kind==Kind::Dps&&meter_view.compact_mode;
let stored=slot_rect(&slot);let x=stored.map(|r|r.left).unwrap_or(if layout.x==i32::MIN{CW_USEDEFAULT}else{layout.x});let y=stored.map(|r|r.top).unwrap_or(if layout.y==i32::MIN{CW_USEDEFAULT}else{layout.y});
let mut initial_w=stored.map(|r|r.right-r.left).unwrap_or(layout.width);let mut initial_h=stored.map(|r|r.bottom-r.top).unwrap_or(layout.height);
if stored.is_none()&&scale_percent!=100{initial_w=scale_px(initial_w,scale_percent);initial_h=scale_px(initial_h,scale_percent);}
if compact_mode{let size=restored_meter_size(&meter_view.compact,scale_percent).unwrap_or_else(||default_compact_size(scale_percent,false));initial_w=size.0;initial_h=size.1;}
"#,"create compact preference");
replace_once(&mut source,r#"let state=Box::new(State{kind,main_hwnd,paths,features,scale_percent,dps:DpsSnapshot::default(),"#,r#"let state=Box::new(State{kind,main_hwnd,paths,features,scale_percent,compact_mode,dps:DpsSnapshot::default(),"#,"create state init");
replace_once(&mut source,r#"let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let work_w=(work.right-work.left).max(1);let work_h=(work.bottom-work.top).max(1);let w=restored_overlay_extent(bounds.right-bounds.left,overlay_min_width(kind,scale_percent),work_w);let h=restored_overlay_extent(bounds.bottom-bounds.top,overlay_min_height(kind,scale_percent),work_h);"#,r#"let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let work_w=(work.right-work.left).max(1);let work_h=(work.bottom-work.top).max(1);let w=restored_overlay_extent(bounds.right-bounds.left,overlay_min_width_mode(kind,scale_percent,compact_mode),work_w);let h=restored_overlay_extent(bounds.bottom-bounds.top,overlay_min_height_mode(kind,scale_percent,compact_mode),work_h);"#,"create mode min");
fs::write(path,source).expect("write compact meter generated overlay");}
