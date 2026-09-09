use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v184.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.8.5 patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Capture-health status is already emitted by the capture worker and shown in
    // the tray. Surface the same status in the Dungeon Mechanics toolbar so a
    // stale/wrong adapter is visible while playing instead of silently producing
    // plausible-but-incomplete meter data.
    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read v1.8.4 generated overlay");
    replace_once(
        &mut overlay,
        "settings_hwnd: HWND,\nfont: HFONT,\nhover_text: Option<String>,",
        "settings_hwnd: HWND,\nfont: HFONT,\ncapture_status: String,\nhover_text: Option<String>,",
        "capture status overlay state",
    );
    replace_once(
        &mut overlay,
        "settings_hwnd:null_mut(),font:create_overlay_font(kind==Kind::Dps),hover_text:None,hover_x:0,hover_y:0}",
        "settings_hwnd:null_mut(),font:create_overlay_font(kind==Kind::Dps),capture_status:\"Starting capture...\".into(),hover_text:None,hover_x:0,hover_y:0}",
        "capture status initialization",
    );
    replace_once(
        &mut overlay,
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|state.mechanics=snapshot);}",
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|state.mechanics=snapshot);}\npub unsafe fn update_capture_status(hwnd:HWND,status:String){with_state(hwnd,|state|state.capture_status=status);}",
        "capture status update API",
    );
    replace_once(
        &mut overlay,
        "return \"Dungeon Mechanics\".into();",
        "return format!(\"Dungeon Mechanics  |  {}\",state.capture_status);",
        "mechanics capture health title",
    );
    fs::write(&overlay_path, overlay).expect("write v1.8.5 overlay");

    // v1.8.x previously re-applied several chat choices on every startup/save.
    // Preserve the user's settings instead; normalize() remains the sole policy
    // layer for validating persisted values.
    let win_path = out.join("win_v182_fixed.rs");
    let mut win = fs::read_to_string(&win_path).expect("read generated v1.8.4 win source");
    replace_once(
        &mut win,
        "fn enforce_hardcoded(s:&mut AppSettings){s.auto_launch_resonance_logs=false;s.resonance_logs_path.clear();s.chat.bold_message_text=false;s.chat.text_shadow=true;s.chat.show_separators=false;}",
        "fn enforce_hardcoded(_s:&mut AppSettings){}",
        "stop overriding user settings",
    );
    replace_once(
        &mut win,
        "AppEvent::CaptureStatus(status)=>{state.capture_status=status;update_tip(hwnd,&state.capture_status);}",
        "AppEvent::CaptureStatus(status)=>{state.capture_status=status.clone();update_tip(hwnd,&state.capture_status);feature_overlays::update_capture_status(state.mechanics_overlay,status);}",
        "surface capture health in mechanics overlay",
    );
    fs::write(&win_path, win).expect("write v1.8.5 win source");

    // Generate a tiny capture wrapper with periodic health telemetry. The actual
    // packet/reassembly/parser implementation remains the audited v1.6 source.
    let mut capture = fs::read_to_string("src/capture_v160.rs")
        .expect("read capture_v160.rs")
        .replace("\r\n", "\n");
    replace_once(
        &mut capture,
        "            let mut last_watchdog = Instant::now();\n            let mut reopen = false;",
        "            let mut last_watchdog = Instant::now();\n            let mut last_status_emit = Instant::now().checked_sub(Duration::from_secs(10)).unwrap_or_else(Instant::now);\n            let mut reopen = false;",
        "capture health timer",
    );
    replace_once(
        &mut capture,
        "                    let running = filter.game_running();\n                    let anchor = last_packet.unwrap_or(opened);",
        "                    let running = filter.game_running();\n                    let anchor = last_packet.unwrap_or(opened);\n                    if last_status_emit.elapsed() >= Duration::from_secs(5) {\n                        last_status_emit = Instant::now();\n                        let packet_recent = last_packet.map(|x| x.elapsed() <= Duration::from_secs(5)).unwrap_or(false);\n                        let frame_recent = processor.last_valid_frame.map(|x| x.elapsed() <= Duration::from_secs(5)).unwrap_or(false);\n                        let health = if !running {\n                            \"Capture ready\"\n                        } else if packet_recent && frame_recent {\n                            \"Capture OK\"\n                        } else if packet_recent {\n                            \"Capture warning: protocol frames stale\"\n                        } else {\n                            \"Capture warning: no recent game packets\"\n                        };\n                        let detail = if running {\n                            format!(\"{health} | {} | packet {} | frame {}\", device.description, age_text(last_packet), age_text(processor.last_valid_frame))\n                        } else {\n                            format!(\"{health} | {} | game not detected\", device.description)\n                        };\n                        let _ = tx.send(AppEvent::CaptureStatus(detail));\n                    }",
        "periodic capture health status",
    );
    capture.push_str("\nfn age_text(last: Option<Instant>) -> String {\n    match last {\n        Some(instant) => {\n            let elapsed = instant.elapsed();\n            if elapsed < Duration::from_secs(1) {\n                format!(\"{}ms\", elapsed.as_millis())\n            } else {\n                format!(\"{:.1}s\", elapsed.as_secs_f64())\n            }\n        }\n        None => \"none\".into(),\n    }\n}\n");
    fs::write(out.join("capture_v185.rs"), capture).expect("write v1.8.5 capture source");

    println!("cargo:rerun-if-changed=build_v185.rs");
    println!("cargo:rerun-if-changed=src/capture_v160.rs");
}
