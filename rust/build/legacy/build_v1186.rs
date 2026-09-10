use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1185.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.6 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.18.6 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.18.6 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.18.6 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}
fn replace_line_start(source:&mut String,prefix:&str,replacement:&str,label:&str){
    let matches:Vec<usize>=source.match_indices(prefix).filter_map(|(i,_)|((i==0||source.as_bytes()[i-1]==b'\n')).then_some(i)).collect();
    assert_eq!(matches.len(),1,"v1.18.6 patch {label} expected one line, found {}",matches.len());
    let begin=matches[0];let end=source[begin..].find('\n').map(|n|begin+n).unwrap_or(source.len());
    source.replace_range(begin..end,replacement);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.5 generated overlays").replace("\r\n","\n");
    replace_once(&mut source,r######"#[link(name="user32")]extern "system"{fn TrackMouseEvent(event:*mut TrackMouseEventData)->i32;}
"######,r######"#[link(name="user32")]extern "system"{fn TrackMouseEvent(event:*mut TrackMouseEventData)->i32;}
#[link(name="gdi32")]extern "system"{fn SetMapMode(hdc:HDC,mode:i32)->i32;fn SetWindowExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;fn SetViewportExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;}
const MM_TEXT_:i32=1;
const MM_ANISOTROPIC_:i32=8;
"######,"GDI scale mapping");
    replace_once(&mut source,r######"#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DetailMode { Damage, Heal, Tank, Buffs, Deaths }
"######,r######"#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DetailMode { Damage, Heal, Tank, Buffs, Deaths }

const OVERLAY_SCALE_MIN:i32=30;
const OVERLAY_SCALE_MAX:i32=300;
const OVERLAY_SCALE_DEFAULT:i32=100;
fn clamp_overlay_scale(value:i32)->i32{value.clamp(OVERLAY_SCALE_MIN,OVERLAY_SCALE_MAX)}
fn scale_px(value:i32,scale:i32)->i32{(((value as i64)*(clamp_overlay_scale(scale)as i64)+50)/100).clamp(1,i32::MAX as i64)as i32}
fn rescale_px(value:i32,old_scale:i32,new_scale:i32)->i32{let old=clamp_overlay_scale(old_scale).max(1)as i64;(((value.max(1)as i64)*(clamp_overlay_scale(new_scale)as i64)+old/2)/old).clamp(1,i32::MAX as i64)as i32}
fn physical_to_logical(value:i32,scale:i32)->i32{((value as i64)*100/(clamp_overlay_scale(scale).max(1)as i64)).clamp(i32::MIN as i64,i32::MAX as i64)as i32}
fn physical_extent_to_logical(value:i32,scale:i32)->i32{let s=clamp_overlay_scale(scale).max(1)as i64;(((value.max(0)as i64)*100+s-1)/s).clamp(0,i32::MAX as i64)as i32}
fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{620}else{340},scale)}
fn overlay_min_height(scale:i32)->i32{scale_px(260,scale)}
unsafe fn logical_client_rect(hwnd:HWND,scale:i32)->RECT{let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);RECT{left:0,top:0,right:physical_extent_to_logical(rc.right.max(0),scale),bottom:physical_extent_to_logical(rc.bottom.max(0),scale)}}
unsafe fn configure_overlay_dc(hdc:HDC,scale:i32){let scale=clamp_overlay_scale(scale);if scale!=100{SetMapMode(hdc,MM_ANISOTROPIC_);SetWindowExtEx(hdc,100,100,null_mut());SetViewportExtEx(hdc,scale,scale,null_mut());}}
unsafe fn reset_overlay_dc(hdc:HDC){SetMapMode(hdc,MM_TEXT_);}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",default)]
struct OverlayScaleSlot{percent:i32,x:i32,y:i32,width:i32,height:i32,has_bounds:bool}
impl Default for OverlayScaleSlot{fn default()->Self{Self{percent:OVERLAY_SCALE_DEFAULT,x:i32::MIN,y:i32::MIN,width:0,height:0,has_bounds:false}}}
#[derive(Clone,Debug,serde::Serialize,serde::Deserialize,Default)]
#[serde(rename_all="camelCase",default)]
struct OverlayScaleFile{dps:OverlayScaleSlot,mechanics:OverlayScaleSlot}
fn scale_file_path(paths:&AppPaths)->std::path::PathBuf{paths.root.join("overlay-scale.json")}
fn normalize_scale_slot(slot:&mut OverlayScaleSlot){slot.percent=clamp_overlay_scale(slot.percent);if slot.width<=0||slot.height<=0||slot.x==i32::MIN||slot.y==i32::MIN{slot.has_bounds=false;}}
fn load_scale_file(paths:&AppPaths)->OverlayScaleFile{let path=scale_file_path(paths);let mut value=std::fs::read_to_string(path).ok().and_then(|text|serde_json::from_str::<OverlayScaleFile>(&text).ok()).unwrap_or_default();normalize_scale_slot(&mut value.dps);normalize_scale_slot(&mut value.mechanics);value}
fn scale_slot(paths:&AppPaths,kind:Kind)->OverlayScaleSlot{let file=load_scale_file(paths);if kind==Kind::Dps{file.dps}else{file.mechanics}}
fn save_scale_slot(paths:&AppPaths,kind:Kind,mut slot:OverlayScaleSlot)->std::io::Result<()>{
    normalize_scale_slot(&mut slot);let mut file=load_scale_file(paths);if kind==Kind::Dps{file.dps=slot}else{file.mechanics=slot}
    let path=scale_file_path(paths);let temp=path.with_extension("json.new");let json=serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;std::fs::write(&temp,json)?;
    if path.exists(){std::fs::remove_file(&path)?;}std::fs::rename(temp,path)
}
fn slot_rect(slot:&OverlayScaleSlot)->Option<RECT>{(slot.percent!=100&&slot.has_bounds).then_some(RECT{left:slot.x,top:slot.y,right:slot.x.saturating_add(slot.width),bottom:slot.y.saturating_add(slot.height)})}
"######,"scale core and persistence");
    replace_once(&mut source,r######"features: Arc<RwLock<FeatureSettings>>,
dps: DpsSnapshot,"######,r######"features: Arc<RwLock<FeatureSettings>>,
scale_percent:i32,
dps: DpsSnapshot,"######,"State scale field");
    replace_between(&mut source,r######"unsafe fn create(instance:HINSTANCE"######,r######"struct ConsumablePopupState"######,r######"unsafe fn create(instance:HINSTANCE,main_hwnd:HWND,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths,kind:Kind)->Result<HWND,String>{
let class_name=if kind==Kind::Dps{DPS_CLASS}else{MECH_CLASS};let class=wide(class_name);let wc=WNDCLASSW{lpfnWndProc:Some(wnd_proc),hInstance:instance,hCursor:LoadCursorW(null_mut(),IDC_ARROW),lpszClassName:class.as_ptr(),..std::mem::zeroed()};if RegisterClassW(&wc)==0&&GetLastError()!=1410{return Err(format!("RegisterClassW({class_name}) failed: {}",GetLastError()));}
let snapshot=features.read().map(|x|x.clone()).unwrap_or_default();let layout=if kind==Kind::Dps{snapshot.dps.clone()}else{snapshot.mechanics.clone()};let slot=scale_slot(&paths,kind);let scale_percent=slot.percent;
let stored=slot_rect(&slot);let x=stored.map(|r|r.left).unwrap_or(if layout.x==i32::MIN{CW_USEDEFAULT}else{layout.x});let y=stored.map(|r|r.top).unwrap_or(if layout.y==i32::MIN{CW_USEDEFAULT}else{layout.y});
let mut initial_w=stored.map(|r|r.right-r.left).unwrap_or(layout.width);let mut initial_h=stored.map(|r|r.bottom-r.top).unwrap_or(layout.height);
if stored.is_none()&&scale_percent!=100{initial_w=scale_px(initial_w,scale_percent);initial_h=scale_px(initial_h,scale_percent);}
let history_records=if kind==Kind::Dps{history::load_recent(&paths.root,snapshot.meter.history_limit)}else{Vec::new()};
let state=Box::new(State{kind,main_hwnd,paths,features,scale_percent,dps:DpsSnapshot::default(),dps_updated_unix_ms:0,image_render_all:false,mechanics:MechanicSnapshot::default(),collapsed:false,expanded:RECT{left:x,top:y,right:x.saturating_add(initial_w),bottom:y.saturating_add(initial_h)},scroll:0,sort_mode:SortMode::Damage,detail_hwnd:null_mut(),detail_uid:0,settings_hwnd:null_mut(),consumable_hwnd:null_mut(),font:create_overlay_font(kind==Kind::Dps),capture_status:"Starting capture...".into(),history:history_records,history_index:None,share_notice:None,hover_text:None,hover_x:0,hover_y:0});
let ptr=Box::into_raw(state);let title=wide(if kind==Kind::Dps{"ReadyAlert DPS Meter"}else{"ReadyAlert Dungeon Mechanics"});let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_LAYERED|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|WS_THICKFRAME,x,y,initial_w,initial_h,null_mut(),null_mut(),instance,ptr.cast::<c_void>());if hwnd.is_null(){drop(Box::from_raw(ptr));return Err(format!("CreateWindowExW({class_name}) failed: {}",GetLastError()));}
let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let work_w=(work.right-work.left).max(1);let work_h=(work.bottom-work.top).max(1);let w=restored_overlay_extent(bounds.right-bounds.left,overlay_min_width(kind,scale_percent),work_w);let h=restored_overlay_extent(bounds.bottom-bounds.top,overlay_min_height(scale_percent),work_h);SetWindowPos(hwnd,null_mut(),bounds.left,bounds.top,w,h,SWP_NOACTIVATE|windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER);crate::ui::fit_window(hwnd,hwnd,false);with_state(hwnd,|s|save_bounds(hwnd,s));apply_opacity(hwnd,layout.opacity);SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);if kind==Kind::Dps{let popup=create_consumable_popup(instance,hwnd);with_state(hwnd,|s|{s.consumable_hwnd=popup;sync_consumable_popup(hwnd,s);});}Ok(hwnd)}
"######,"scaled overlay creation");
    replace_between(&mut source,r######"unsafe fn sync_consumable_popup("######,r######"unsafe fn popup_status_at"######,r######"unsafe fn sync_consumable_popup(parent:HWND,state:&mut State){
    if state.kind!=Kind::Dps||state.consumable_hwnd.is_null()||IsWindow(state.consumable_hwnd)==0{return;}
    if state.collapsed{ShowWindow(state.consumable_hwnd,0);return;}
    let mut r:RECT=std::mem::zeroed();if GetWindowRect(parent,&mut r)==0{return;}let top=scale_px(dps_rows_top(),state.scale_percent);let popup_w=scale_px(DPS_CONSUMABLE_POPUP_W,state.scale_percent);let popup_gap=scale_px(DPS_CONSUMABLE_POPUP_GAP,state.scale_percent);let h=(r.bottom-r.top-top).max(1);
    SetWindowPos(state.consumable_hwnd,HWND_TOPMOST,r.left-popup_w-popup_gap,r.top+top,popup_w,h,SWP_NOACTIVATE);ShowWindow(state.consumable_hwnd,SW_SHOW);
}
"######,"scaled consumable popup geometry");
    replace_between(&mut source,r######"unsafe fn popup_status_at<'a>"######,r######"unsafe fn paint_consumable_popup"######,r######"unsafe fn popup_status_at<'a>(state:&'a State,x:i32,y:i32,now:i64)->Option<(&'a ConsumableStatus,&'static str)>{
    if y<0{return None;}let screen_i=(y/DPS_ROW_H)as usize;let logical_height=physical_extent_to_logical((state.expanded.bottom-state.expanded.top).max(0),state.scale_percent);let physical=((logical_height-dps_rows_top())/DPS_ROW_H).max(0)as usize;let shown=meter_page_rows(state,physical);let row=shown.get(screen_i).map(|(_,row)|*row)?;
    let iy=screen_i as i32*DPS_ROW_H+((DPS_ROW_H-2-DPS_CONSUMABLE_ICON)/2).max(0);if y<iy||y>=iy+DPS_CONSUMABLE_ICON{return None;}
    let fx=2;let sx=fx+DPS_CONSUMABLE_ICON+DPS_CONSUMABLE_GAP;
    if x>=fx&&x<fx+DPS_CONSUMABLE_ICON{return active_consumable(row.food.as_ref(),now).map(|s|(s,"Food"));}
    if x>=sx&&x<sx+DPS_CONSUMABLE_ICON{return active_consumable(row.serum.as_ref(),now).map(|s|(s,"Serum"));}None
}
"######,"scaled consumable hit rows");
    replace_between(&mut source,r######"unsafe fn paint_consumable_popup("######,r######"unsafe fn on_consumable_mouse_move"######,r######"unsafe fn paint_consumable_popup(hwnd:HWND,state:&ConsumablePopupState){
    let parent_ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}
    let scale=if parent_ptr.is_null(){100}else{(*parent_ptr).scale_percent};let mut physical:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut physical);configure_overlay_dc(hdc,scale);let rc=RECT{left:0,top:0,right:physical_extent_to_logical(physical.right.max(0),scale),bottom:physical_extent_to_logical(physical.bottom.max(0),scale)};fill(hdc,&rc,DPS_CONSUMABLE_KEY);
    if !parent_ptr.is_null(){let parent=&*parent_ptr;if !parent.collapsed{let now=now_ms();let visible=(rc.bottom/DPS_ROW_H).max(0)as usize;let shown=meter_page_rows(parent,visible);
        for(screen_i,(_,row))in shown.iter().enumerate(){let row=*row;let y=screen_i as i32*DPS_ROW_H+((DPS_ROW_H-2-DPS_CONSUMABLE_ICON)/2).max(0);let fx=2;let sx=fx+DPS_CONSUMABLE_ICON+DPS_CONSUMABLE_GAP;
            if let Some(food)=active_consumable(row.food.as_ref(),now){paint_consumable_ring(hdc,fx,y,"F",food,now);}if let Some(serum)=active_consumable(row.serum.as_ref(),now){paint_consumable_ring(hdc,sx,y,"S",serum,now);}
        }
    }}reset_overlay_dc(hdc);EndPaint(hwnd,&ps);
}
"######,"scaled consumable painting");
    replace_between(&mut source,r######"unsafe fn on_consumable_mouse_move("######,r######"unsafe extern "system" fn consumable_wnd_proc"######,r######"unsafe fn on_consumable_mouse_move(hwnd:HWND,state:&ConsumablePopupState,lparam:LPARAM){
    let parent_ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if parent_ptr.is_null(){return;}let parent=&mut*parent_ptr;let x=physical_to_logical(lo_signed(lparam),parent.scale_percent);let y=physical_to_logical(hi_signed(lparam),parent.scale_percent);let now=now_ms();
    let next=popup_status_at(parent,x,y,now).map(|(status,kind)|format!("{}: {} · {}",kind,status.name,consumable_remaining_text(status,now)));
    if next!=parent.hover_text{parent.hover_text=next;parent.hover_x=4;parent.hover_y=dps_rows_top()+y;InvalidateRect(state.parent,null(),0);}
    let mut track=TrackMouseEventData{cb_size:std::mem::size_of::<TrackMouseEventData>()as u32,flags:TME_LEAVE_,hwnd,hover_time:0};TrackMouseEvent(&mut track);
}
"######,"scaled consumable mouse");
    replace_once(&mut source,r######"let x=sx-r.left;let y=sy-r.top;let parent_ptr=GetWindowLongPtrW((*ptr).parent,GWLP_USERDATA)as *mut State;if parent_ptr.is_null(){-1}else if popup_status_at(&*parent_ptr,x,y,now_ms()).is_some(){HTCLIENT as isize}else{-1}"######,r######"let parent_ptr=GetWindowLongPtrW((*ptr).parent,GWLP_USERDATA)as *mut State;if parent_ptr.is_null(){-1}else{let x=physical_to_logical(sx-r.left,(*parent_ptr).scale_percent);let y=physical_to_logical(sy-r.top,(*parent_ptr).scale_percent);if popup_status_at(&*parent_ptr,x,y,now_ms()).is_some(){HTCLIENT as isize}else{-1}}"######,"scaled popup hit testing");
    replace_once(&mut source,r######"let(width,height)=dps_image_dimensions(client.right-client.left,row_count);"######,r######"let logical_width=physical_extent_to_logical(client.right-client.left,state.scale_percent);let(width,height)=dps_image_dimensions(logical_width,row_count);"######,"copy image remains 100 percent resolution");
    replace_between(&mut source,r######"unsafe extern "system" fn wnd_proc("######,r######"unsafe fn on_click"######,r######"unsafe extern "system" fn wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
if msg==WM_NCCREATE{let create=lparam as *const CREATESTRUCTW;if !create.is_null(){SetWindowLongPtrW(hwnd,GWLP_USERDATA,(*create).lpCreateParams as isize);}}
let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut State;
match msg{
0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{crate::ui::min_window(hwnd,lparam,overlay_min_width((*ptr).kind,(*ptr).scale_percent),overlay_min_height((*ptr).scale_percent));}0},
0x0100=>{if !ptr.is_null(){overlay_key(hwnd,&mut *ptr,wparam);}0},
WM_NCCALCSIZE=>0,
WM_NCHITTEST=>{if ptr.is_null(){DefWindowProcW(hwnd,msg,wparam,lparam)}else{resize_hit_test(hwnd,lparam)}},
WM_ERASEBKGND=>1,
WM_MOUSEMOVE=>{if !ptr.is_null(){on_mouse_move(hwnd,&mut*ptr,lparam);}0},
WM_MOUSELEAVE_=>{if !ptr.is_null(){(*ptr).hover_text=None;InvalidateRect(hwnd,null(),0);}0},
WM_PAINT=>{if !ptr.is_null(){paint(hwnd,&mut*ptr);}0},
0x0003=>{if !ptr.is_null(){sync_consumable_popup(hwnd,&mut*ptr);}0},
WM_SIZE=>{if !ptr.is_null(){clamp_scroll(hwnd,&mut*ptr);sync_consumable_popup(hwnd,&mut*ptr);}InvalidateRect(hwnd,null(),0);0},
WM_LBUTTONDOWN=>{if !ptr.is_null(){on_click(hwnd,&mut*ptr,lparam);}0},
WM_MOUSEWHEEL=>{if !ptr.is_null(){on_wheel(hwnd,&mut*ptr,wparam);}0},
WM_EXITSIZEMOVE=>{if !ptr.is_null(){save_bounds(hwnd,&mut*ptr);clamp_scroll(hwnd,&mut*ptr);}0},
WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){let state=&mut*ptr;for child in[state.detail_hwnd,state.settings_hwnd,state.consumable_hwnd]{if !child.is_null()&&IsWindow(child)!=0{DestroyWindow(child);}}if !state.font.is_null(){DeleteObject(state.font);}drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},
_=>DefWindowProcW(hwnd,msg,wparam,lparam)
}}
"######,"scaled min track size");
    replace_between(&mut source,r######"unsafe fn on_click("######,r######"fn wheel_row_steps"######,r######"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){
crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}
let x=physical_to_logical(lo_signed(lparam),state.scale_percent);let y=physical_to_logical(hi_signed(lparam),state.scale_percent);let rc=logical_client_rect(hwnd,state.scale_percent);
if y<TOOLBAR_H{
    if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}
    else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}
    else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}
    else if state.kind==Kind::Dps{
        for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}
        drag_window(hwnd);
    }else{drag_window(hwnd);}
    return;
}
if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}
if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}
}
"######,"logical click coordinates");
    replace_between(&mut source,r######"unsafe fn paint(hwnd:HWND,state:&mut State)"######,r######"unsafe fn paint_toolbar"######,r######"unsafe fn paint(hwnd:HWND,state:&mut State){clamp_scroll(hwnd,state);
let mut ps:PAINTSTRUCT=std::mem::zeroed();let screen=BeginPaint(hwnd,&mut ps);if screen.is_null(){return;}let mut physical:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut physical);let width=physical.right.max(1);let height=physical.bottom.max(1);let mem=CreateCompatibleDC(screen);let bitmap=if !mem.is_null(){CreateCompatibleBitmap(screen,width,height)}else{null_mut()};let old_bitmap=if !mem.is_null()&&!bitmap.is_null(){SelectObject(mem,bitmap)}else{null_mut()};let hdc=if !mem.is_null()&&!bitmap.is_null(){mem}else{screen};configure_overlay_dc(hdc,state.scale_percent);let rc=RECT{left:0,top:0,right:physical_extent_to_logical(width,state.scale_percent),bottom:physical_extent_to_logical(height,state.scale_percent)};fill(hdc,&rc,crate::ui_theme::BG);let font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};SelectObject(hdc,font);SetBkMode(hdc,TRANSPARENT as i32);if state.collapsed{SetTextColor(hdc,crate::ui_theme::ACCENT);draw(hdc,expand_glyph(state),rc,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}else{paint_toolbar(hdc,rc,state);match state.kind{Kind::Dps=>paint_dps(hdc,rc,state),Kind::Mechanics=>paint_mechanics(hdc,rc,state)}paint_hover(hdc,rc,state);}reset_overlay_dc(hdc);if hdc==mem{BitBlt(screen,0,0,width,height,mem,0,0,SRCCOPY);SelectObject(mem,old_bitmap);DeleteObject(bitmap);DeleteDC(mem);}EndPaint(hwnd,&ps);if state.kind==Kind::Dps&&!state.consumable_hwnd.is_null()&&IsWindow(state.consumable_hwnd)!=0{InvalidateRect(state.consumable_hwnd,null(),0);}}
"######,"whole-overlay GDI scale");
    replace_between(&mut source,r######"unsafe fn on_mouse_move("######,r######"fn active_consumable"######,r######"unsafe fn on_mouse_move(hwnd:HWND,state:&mut State,lparam:LPARAM){let mut track=TrackMouseEventData{cb_size:std::mem::size_of::<TrackMouseEventData>() as u32,flags:TME_LEAVE_,hwnd,hover_time:0};TrackMouseEvent(&mut track);let x=physical_to_logical(lo_signed(lparam),state.scale_percent);let y=physical_to_logical(hi_signed(lparam),state.scale_percent);let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));if state.hover_text!=next||state.hover_x!=x||state.hover_y!=y{state.hover_text=next;state.hover_x=x;state.hover_y=y;InvalidateRect(hwnd,null(),0);}}

"######,"logical hover coordinates");
    replace_line_start(&mut source,r######"unsafe fn hover_badge_at("######,r######"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let rc=logical_client_rect(hwnd,state.scale_percent);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;let physical=visible_dps_rows(rc.bottom);if screen_i>=physical{return None;}let shown=meter_page_rows(state,physical);let row=shown.get(screen_i).map(|(_,row)|*row)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);if layout.badge_count==0{return None;}let mut bx=layout.badge_left;for badge in row.imagines.iter().take(layout.badge_count){if x>=bx&&x<bx+BADGE_W&&y>=r.top+2&&y<r.top+25{return Some(format!("{} · {}",badge.name,crate::model::imagine_tier_label(badge.tier)));}bx+=BADGE_W+BADGE_GAP;}None}"######,"scaled badge hover");
    replace_line_start(&mut source,r######"unsafe fn dps_row_at("######,r######"unsafe fn dps_row_at(hwnd:HWND,state:&State,y:i32)->Option<DpsRow>{let rc=logical_client_rect(hwnd,state.scale_percent);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;let physical=visible_dps_rows(rc.bottom);if screen_i>=physical{return None;}meter_page_rows(state,physical).get(screen_i).map(|(_,row)|(*row).clone())}"######,"scaled DPS row hit test");
    replace_line_start(&mut source,r######"unsafe fn clamp_scroll("######,r######"unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let rc=logical_client_rect(hwnd,state.scale_percent);let(total,visible)=if state.kind==Kind::Dps{let physical=visible_dps_rows(rc.bottom);(meter_rows(state).len(),meter_regular_page_size(state,physical))}else{let(total,visible,_)=mechanics_scroll_metrics(state,rc.bottom);(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}"######,"scaled viewport scrolling");
    replace_once(&mut source,r######"feature_label(hwnd,7000,"",18,50,112,22);feature_button(hwnd,7001,"−",134,44,36);feature_button(hwnd,7002,"+",176,44,36);
    feature_label(hwnd,0,"Collapse edge",242,50,100,22);feature_combo(hwnd,7003,346,44,&["Right","Bottom","Left","Top"]);"######,r######"feature_label(hwnd,7000,"",18,50,86,22);feature_button(hwnd,7001,"−",106,44,30);feature_button(hwnd,7002,"+",140,44,30);
    feature_label(hwnd,7004,"",190,50,92,22);feature_button(hwnd,7005,"−",286,44,30);feature_button(hwnd,7006,"+",320,44,30);
    feature_label(hwnd,0,"Collapse",370,50,70,22);feature_combo(hwnd,7003,444,44,&["Right","Bottom","Left","Top"]);"######,"scale controls");
    replace_once(&mut source,r######"windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7000),wide(&format!("Opacity: {}%",layout.opacity)).as_ptr());
    let side="######,r######"windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7000),wide(&format!("Opacity: {}%",layout.opacity)).as_ptr());
    let scale=settings_overlay_scale(state);windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7004),wide(&format!("Scale: {}%",scale)).as_ptr());crate::ui::EnableWindow(fc(hwnd,7005),(scale>OVERLAY_SCALE_MIN)as i32);crate::ui::EnableWindow(fc(hwnd,7006),(scale<OVERLAY_SCALE_MAX)as i32);
    let side="######,"refresh scale controls");
    replace_once(&mut source,r######"unsafe extern "system" fn settings_wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT {"######,r######"unsafe fn settings_overlay_scale(state:&SettingsState)->i32{let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){scale_slot(&state.paths,state.kind).percent}else{(*ptr).scale_percent}}
unsafe fn persist_overlay_rect(state:&mut State,rect:RECT){
    state.expanded=rect;
    if let Ok(mut features)=state.features.write(){let layout=if state.kind==Kind::Dps{&mut features.dps}else{&mut features.mechanics};layout.x=rect.left;layout.y=rect.top;layout.width=(rect.right-rect.left).max(1);layout.height=(rect.bottom-rect.top).max(1);let snapshot=features.clone();drop(features);let _=feature_settings::save(&state.paths,&snapshot);}
    let mut slot=scale_slot(&state.paths,state.kind);slot.percent=state.scale_percent;if state.scale_percent==100{slot.has_bounds=false;}else{slot.x=rect.left;slot.y=rect.top;slot.width=(rect.right-rect.left).max(1);slot.height=(rect.bottom-rect.top).max(1);slot.has_bounds=true;}let _=save_scale_slot(&state.paths,state.kind,slot);
}
unsafe fn resize_collapsed_for_scale(hwnd:HWND,state:&State){
    let work=crate::ui::work_area(hwnd);let side=layout_side(state);let thick=scale_px(COLLAPSED,state.scale_percent);let expanded_w=(state.expanded.right-state.expanded.left).max(thick);let expanded_h=(state.expanded.bottom-state.expanded.top).max(thick);
    let(x,y,w,h)=match side.as_str(){"left"=>(work.left,state.expanded.top.clamp(work.top,work.bottom-thick),thick,expanded_h.min(work.bottom-work.top)),"top"=>(state.expanded.left.clamp(work.left,work.right-thick),work.top,expanded_w.min(work.right-work.left),thick),"bottom"=>(state.expanded.left.clamp(work.left,work.right-thick),work.bottom-thick,expanded_w.min(work.right-work.left),thick),_=>(work.right-thick,state.expanded.top.clamp(work.top,work.bottom-thick),thick,expanded_h.min(work.bottom-work.top))};SetWindowPos(hwnd,HWND_TOPMOST,x,y,w.max(1),h.max(1),SWP_NOACTIVATE);
}
unsafe fn change_overlay_scale(state:&SettingsState,delta:i32){
    let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return;}let overlay=&mut*ptr;let old=overlay.scale_percent;let new=clamp_overlay_scale(old.saturating_add(delta));if new==old{return;}
    let source=if overlay.collapsed{overlay.expanded}else{let mut r:RECT=std::mem::zeroed();if GetWindowRect(state.parent,&mut r)==0{return;}r};let work=crate::ui::work_area(state.parent);
    let target_w=rescale_px((source.right-source.left).max(1),old,new).max(overlay_min_width(overlay.kind,new));let target_h=rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height(new));let target=crate::ui::fit_rect(RECT{left:source.left,top:source.top,right:source.left.saturating_add(target_w),bottom:source.top.saturating_add(target_h)},work);
    overlay.scale_percent=new;overlay.hover_text=None;
    if overlay.collapsed{persist_overlay_rect(overlay,target);resize_collapsed_for_scale(state.parent,overlay);}else{SetWindowPos(state.parent,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);persist_overlay_rect(overlay,target);clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);}
    InvalidateRect(state.parent,null(),0);if !overlay.consumable_hwnd.is_null(){InvalidateRect(overlay.consumable_hwnd,null(),0);}
}
unsafe extern "system" fn settings_wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT {"######,"live scale resize helpers");
    replace_once(&mut source,r######"if id==7021{crate::event_tracker_ui::show(hwnd);return 0;}
            if matches!(id,7003|7010)&&code!=1{return 0;}"######,r######"if id==7021{crate::event_tracker_ui::show(hwnd);return 0;}
            if id==7005||id==7006{change_overlay_scale(state,if id==7005{-10}else{10});refresh_feature_form(hwnd,state);return 0;}
            if matches!(id,7003|7010)&&code!=1{return 0;}"######,"scale commands");
    replace_line_start(&mut source,r######"unsafe fn collapse("######,r######"unsafe fn collapse(hwnd:HWND,state:&mut State){let mut rect:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut rect);state.expanded=rect;let work=monitor_work(hwnd);let side=layout_side(state);let collapsed=scale_px(COLLAPSED,state.scale_percent);let width=(rect.right-rect.left).max(scale_px(100,state.scale_percent));let height=(rect.bottom-rect.top).max(scale_px(80,state.scale_percent));let(x,y,new_width,new_height)=match side.as_str(){"left"=>(work.left,rect.top.clamp(work.top,work.bottom-collapsed),collapsed,height.min(work.bottom-work.top)),"top"=>(rect.left.clamp(work.left,work.right-collapsed),work.top,width.min(work.right-work.left),collapsed),"bottom"=>(rect.left.clamp(work.left,work.right-collapsed),work.bottom-collapsed,width.min(work.right-work.left),collapsed),_=>(work.right-collapsed,rect.top.clamp(work.top,work.bottom-collapsed),collapsed,height.min(work.bottom-work.top))};state.collapsed=true;SetWindowPos(hwnd,HWND_TOPMOST,x,y,new_width.max(1),new_height.max(1),SWP_NOACTIVATE);sync_consumable_popup(hwnd,state);InvalidateRect(hwnd,null(),0);}"######,"scaled collapse thickness");
    replace_line_start(&mut source,r######"unsafe fn expand_state("######,r######"unsafe fn expand_state(hwnd:HWND,state:&mut State){let work=crate::ui::work_area(hwnd);let min_w=overlay_min_width(state.kind,state.scale_percent);let min_h=overlay_min_height(state.scale_percent);let raw=RECT{left:state.expanded.left,top:state.expanded.top,right:state.expanded.left.saturating_add((state.expanded.right-state.expanded.left).max(min_w)),bottom:state.expanded.top.saturating_add((state.expanded.bottom-state.expanded.top).max(min_h))};let rect=crate::ui::fit_rect(raw,work);state.collapsed=false;SetWindowPos(hwnd,HWND_TOPMOST,rect.left,rect.top,(rect.right-rect.left).max(1),(rect.bottom-rect.top).max(1),SWP_NOACTIVATE);state.expanded=rect;persist_overlay_rect(state,rect);sync_consumable_popup(hwnd,state);InvalidateRect(hwnd,null(),0);}"######,"scaled expand minimum");
    replace_line_start(&mut source,r######"unsafe fn save_bounds("######,r######"unsafe fn save_bounds(hwnd:HWND,state:&mut State){if state.collapsed{return;}let mut rect:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut rect)==0{return;}persist_overlay_rect(state,rect);}"######,"persist scaled bounds");
    replace_once(&mut source,r######"let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    let(total,page)=if state.kind==Kind::Dps"######,r######"let rc=logical_client_rect(hwnd,state.scale_percent);
    let(total,page)=if state.kind==Kind::Dps"######,"keyboard viewport scale");
    replace_once(&mut source,r######"let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    if y<TOOLBAR_H{"######,r######"let rc=logical_client_rect(hwnd,state.scale_percent);
    if y<TOOLBAR_H{"######,"help viewport scale");
    replace_once(&mut source,r######"fn lo_signed(value:LPARAM)->i32{(value as u16 as i16)as i32}
"######,r######"#[cfg(test)]
mod v1186_overlay_scale_tests{
    use super::*;
    #[test]fn scale_range_is_30_to_300_percent(){assert_eq!(clamp_overlay_scale(0),30);assert_eq!(clamp_overlay_scale(30),30);assert_eq!(clamp_overlay_scale(100),100);assert_eq!(clamp_overlay_scale(300),300);assert_eq!(clamp_overlay_scale(999),300);}
    #[test]fn minimum_window_size_tracks_scale(){assert_eq!(overlay_min_width(Kind::Dps,30),186);assert_eq!(overlay_min_height(30),78);assert_eq!(overlay_min_width(Kind::Dps,100),620);assert_eq!(overlay_min_height(100),260);assert_eq!(overlay_min_width(Kind::Dps,300),1860);assert_eq!(overlay_min_height(300),780);assert_eq!(overlay_min_width(Kind::Mechanics,30),102);}
    #[test]fn resizing_scale_preserves_relative_window_size(){assert_eq!(rescale_px(650,100,30),195);assert_eq!(rescale_px(195,30,100),650);assert_eq!(rescale_px(650,100,300),1950);}
    #[test]fn physical_client_is_converted_back_to_base_layout_units(){assert_eq!(physical_extent_to_logical(195,30),650);assert_eq!(physical_extent_to_logical(650,100),650);assert_eq!(physical_extent_to_logical(1950,300),650);}
    #[test]fn missing_scale_config_defaults_to_current_ui_size(){let slot=OverlayScaleSlot::default();assert_eq!(slot.percent,100);assert!(!slot.has_bounds);}
}
fn lo_signed(value:LPARAM)->i32{(value as u16 as i16)as i32}
"######,"v1.18.6 regression tests");
    fs::write(path,source).expect("write v1.18.6 scaled overlays");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1186.rs");
}
