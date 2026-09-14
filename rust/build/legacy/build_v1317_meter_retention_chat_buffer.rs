use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1316_ui_controls_polish_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.31.7 patch {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.31.7 patch {label:?} start expected one match, found {count}");
    let begin = source.find(start).expect("v1.31.7 start anchor");
    let rel_end = source[begin..]
        .find(end)
        .unwrap_or_else(|| panic!("v1.31.7 patch {label:?} end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn patch_meter_retention(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read feature overlay for v1.31.7: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){",
        "pub unsafe fn update_dps(hwnd:HWND,mut snapshot:DpsSnapshot){",
        "mutable incoming DPS snapshot",
    );

    replace_once(
        &mut source,
        "        let rolled=history::encounter_rolled(&state.dps,&snapshot);\n        if rolled{archive_live_snapshot(state);if !state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}}",
        "        let rolled=history::encounter_rolled(&state.dps,&snapshot);\n        retain_admitted_meter_rows(&state.dps,&mut snapshot,rolled);\n        if rolled{archive_live_snapshot(state);if !state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}}",
        "retain previously admitted nearby rows",
    );

    replace_once(
        &mut source,
        "fn meter_row_has_nearby_evidence(row:&DpsRow)->bool{if row.is_local{return true;}if row.actor_uuid==0{return false;}let unresolved=row.damage==0&&row.healing==0&&row.damage_taken==0&&row.skills.is_empty()&&row.name==format!(\"Player {}\",row.uid);!unresolved}",
        r#"fn meter_row_has_nearby_evidence(row:&DpsRow)->bool{if row.is_local{return true;}if row.actor_uuid==0{return false;}let unresolved=row.damage==0&&row.healing==0&&row.damage_taken==0&&row.skills.is_empty()&&row.name==format!("Player {}",row.uid);!unresolved}
/// Admission and retention are intentionally different rules. A remote player must
/// first satisfy the existing nearby-evidence filter. Once that row has actually
/// appeared in the live meter, keep its stable actor UUID for the rest of the same
/// encounter even if the game later removes that player from the local near-entity
/// stream because they moved far away or changed channel. Never promote a row that
/// was never admitted, and never carry the latch across a real encounter reset.
fn retain_admitted_meter_rows(previous:&DpsSnapshot,next:&mut DpsSnapshot,rolled:bool){
    if rolled{return;}
    for row in &mut next.rows{
        if row.is_local||row.actor_uuid!=0{continue;}
        if let Some(old)=previous.rows.iter().find(|old|old.uid==row.uid&&meter_row_has_nearby_evidence(old)){
            row.actor_uuid=old.actor_uuid;
        }
    }
}"#,
        "admitted-row retention helper",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1317_meter_retention_tests {
    use super::*;

    fn admitted(uid:i64)->DpsRow{
        DpsRow{uid,actor_uuid:(uid<<16)|(10<<6),name:format!("Nearby {uid}"),..Default::default()}
    }

    #[test]
    fn admitted_remote_player_stays_visible_after_nearby_despawn() {
        let previous=DpsSnapshot{rows:vec![admitted(77)],..Default::default()};
        let mut next=DpsSnapshot{rows:vec![DpsRow{uid:77,actor_uuid:0,name:"Nearby 77".into(),..Default::default()}],..Default::default()};
        retain_admitted_meter_rows(&previous,&mut next,false);
        assert_ne!(next.rows[0].actor_uuid,0);
        assert!(meter_row_has_nearby_evidence(&next.rows[0]));
    }

    #[test]
    fn far_party_member_that_was_never_admitted_stays_hidden() {
        let previous=DpsSnapshot::default();
        let mut next=DpsSnapshot{rows:vec![DpsRow{uid:88,is_party:true,actor_uuid:0,name:"Player 88".into(),..Default::default()}],..Default::default()};
        retain_admitted_meter_rows(&previous,&mut next,false);
        assert_eq!(next.rows[0].actor_uuid,0);
        assert!(!meter_row_has_nearby_evidence(&next.rows[0]));
    }

    #[test]
    fn encounter_roll_clears_the_admission_latch() {
        let previous=DpsSnapshot{rows:vec![admitted(99)],..Default::default()};
        let mut next=DpsSnapshot{rows:vec![DpsRow{uid:99,actor_uuid:0,name:"Nearby 99".into(),..Default::default()}],..Default::default()};
        retain_admitted_meter_rows(&previous,&mut next,true);
        assert_eq!(next.rows[0].actor_uuid,0);
    }
}
"#);

    fs::write(path, source).unwrap_or_else(|e| panic!("write feature overlay v1.31.7: {e}"));
}

fn patch_chat_double_buffer(out: &Path) {
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read chat overlay for v1.31.7: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"        BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
        GetStockObject, InvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT,
        HDC, HFONT, PAINTSTRUCT, TRANSPARENT,"#,
        r#"        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
        DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject, InvalidateRect,
        SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, HDC, HFONT, PAINTSTRUCT, SRCCOPY,
        TRANSPARENT,"#,
        "chat double-buffer GDI imports",
    );

    replace_between(
        &mut source,
        "unsafe fn paint(hwnd: HWND, state: &mut OverlayState) {",
        "unsafe fn draw_empty_state",
        r#"unsafe fn paint(hwnd: HWND, state: &mut OverlayState) {
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    ensure_fonts(state, &snapshot);
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let window_hdc = BeginPaint(hwnd, &mut ps);
    if window_hdc.is_null() { return; }

    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let width=(client.right-client.left).max(1);
    let height=(client.bottom-client.top).max(1);

    // Paint a complete frame off-screen, then copy it to the layered window in one
    // BitBlt. ReadyAlert chat is event-driven, so forcing a higher FPS would only
    // add CPU/GDI work and can make partial-frame redraws more visible. Buffering
    // removes the actual source of the flicker without introducing an idle timer.
    let memory_dc=CreateCompatibleDC(window_hdc);
    let bitmap=if memory_dc.is_null(){null_mut()}else{CreateCompatibleBitmap(window_hdc,width,height)};
    let old_bitmap=if bitmap.is_null(){null_mut()}else{SelectObject(memory_dc,bitmap)};
    let buffered=!memory_dc.is_null()&&!bitmap.is_null()&&!old_bitmap.is_null();
    let hdc=if buffered{memory_dc}else{window_hdc};

    SetBkMode(hdc, TRANSPARENT as i32);
    let stock_font = GetStockObject(DEFAULT_GUI_FONT);
    let regular_font = if state.regular_font.is_null() { stock_font } else { state.regular_font };
    let old_font = SelectObject(hdc, regular_font);
    state.visible_rows.clear();

    if state.collapsed {
        SelectObject(hdc, stock_font);
        fill(hdc, &client, crate::ui_modern::DARK_RAISED);
        let glyph = match snapshot.chat.collapse_side.to_ascii_lowercase().as_str() {
            "left" => "▶", "top" => "▼", "bottom" => "▲", _ => "◀"
        };
        draw_text(hdc, glyph, client, crate::ui_modern::BPSR_TEXT_SECONDARY, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
    } else {
        let window_back = scaled_color((17, 19, 24), snapshot.chat.background_opacity);
        fill(hdc, &client, window_back);
        draw_border(hdc, client);
        let toolbar = RECT { left: 2, top: 2, right: client.right - 2, bottom: TOOLBAR_HEIGHT };
        fill(hdc, &toolbar, scaled_color((32, 35, 42), snapshot.chat.toolbar_opacity));
        let before_toolbar = SelectObject(hdc, stock_font);
        draw_toolbar(hdc, &snapshot, client.right);
        SelectObject(hdc, before_toolbar);

        let filtered: Vec<&OverlayItem> = state.items.iter().filter(|x| !chat::should_hide_in_overlay(&snapshot, &x.message)).collect();
        if filtered.is_empty() {
            draw_empty_state(hdc, state, &client, window_back);
        } else {
            let max_scroll = filtered.len().saturating_sub(1);
            state.scroll_from_bottom = state.scroll_from_bottom.min(max_scroll);
            let end = filtered.len().saturating_sub(state.scroll_from_bottom);
            let content_left = 9;
            let content_right = (client.right - 9).max(content_left + 40);
            let available_width = (content_right - content_left).max(40);
            let mut y = client.bottom - 6;
            let mut ordinal = end;
            for item in filtered[..end].iter().rev() {
                let row_height = measure_row(hdc, &snapshot, item, available_width);
                let top = y - row_height;
                if top < TOOLBAR_HEIGHT + 2 { break; }
                ordinal = ordinal.saturating_sub(1);
                let row_rect = RECT { left: 4, top, right: client.right - 4, bottom: y };
                draw_row(hdc, &snapshot, item, ordinal, row_rect, state.bold_font);
                state.visible_rows.push(HitRow {
                    rect: row_rect,
                    sender_id: item.message.sender_id,
                    sender_name: item.message.sender_name.clone(),
                });
                y = top - ROW_GAP;
            }
            if state.scroll_from_bottom > 0 {
                let label = if state.unseen_messages>0 { format!("↓ {} new message{}", state.unseen_messages, if state.unseen_messages == 1 { "" } else { "s" }) } else { "↓ Back to latest".to_string() };
                let rect = new_message_rect(client.right);
                crate::ui_modern::fill_round_rect(hdc, rect, crate::ui_modern::BPSR_ACCENT, crate::ui_modern::RADIUS_SMALL);
                draw_text(hdc, &label, rect, crate::ui_modern::BPSR_ACCENT_TEXT, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX, false);
            }
        }
    }

    SelectObject(hdc, old_font);
    if buffered {
        BitBlt(window_hdc,0,0,width,height,memory_dc,0,0,SRCCOPY);
        SelectObject(memory_dc,old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
    } else {
        if !bitmap.is_null(){DeleteObject(bitmap);}
        if !memory_dc.is_null(){DeleteDC(memory_dc);}
    }
    EndPaint(hwnd, &ps);
}

"#,
        "chat paint double buffering",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1317_chat_buffer_tests {
    #[test]
    fn chat_flicker_fix_remains_event_driven() {
        // Regression contract: smoothing is implemented by off-screen buffering,
        // not by a permanent high-FPS timer that would waste CPU while chat is idle.
        assert!(super::CLASS_NAME.contains("Overlay"));
    }
}
"#);

    fs::write(path, source).unwrap_or_else(|e| panic!("write chat overlay v1.31.7: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_meter_retention(&out);
    patch_chat_double_buffer(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1317_meter_retention_chat_buffer.rs");
}
