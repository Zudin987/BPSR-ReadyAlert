use std::{fs, path::Path};

const MARKER: &str = "readyalert-readable-type";

fn inject(path: &Path, css: &str, label: &str) -> Result<(), String> {
    let html = fs::read_to_string(path).map_err(|err| format!("read {label} typography: {err}"))?;
    if html.contains(MARKER) {
        return Err(format!("{label} typography already applied"));
    }
    if html.matches("</body>").count() != 1 {
        return Err(format!("{label} typography expected one closing body"));
    }
    let style = format!("<style id=\"{MARKER}\">\n{css}\n</style>\n");
    let updated = html.replacen("</body>", &(style + "</body>"), 1);
    fs::write(path, updated).map_err(|err| format!("write {label} typography: {err}"))
}

pub fn upgrade_encounter(path: &Path) -> Result<(), String> {
    inject(path, r#"
html{font-size:100%}
body{font-size:1rem;line-height:1.5}
.brand h1{font-size:1.125rem}.brand p,.local{font-size:.8125rem}
.archive-tabs a{font-size:.75rem}
.tool{font-size:.8125rem}.compare-count{font-size:.75rem}.search{font-size:.875rem}
.enc-title{font-size:.9375rem}.enc-meta{font-size:.75rem}.enc-stat{font-size:.8125rem}
.top h2{font-size:1.375rem}.subtitle{font-size:.8125rem}.pill{font-size:.75rem}
.metric .k{font-size:.6875rem}.metric .v{font-size:1.25rem}
.section-head{font-size:.8125rem}.player-name{font-size:.875rem}.player-sub{font-size:.75rem}
.tab{font-size:.8125rem}.mini .label{font-size:.75rem}.mini .value{font-size:1rem}
.table{font-size:.8125rem}.table th{font-size:.6875rem}
.event{font-size:.8125rem}.empty{font-size:.875rem}.empty strong{font-size:1.0625rem}.footer{font-size:.75rem}
.compare-head h3{font-size:1rem}.compare-player{font-size:.8125rem}
.enc-filter-field>span{font-size:.6875rem}.enc-filter-control{font-size:.8125rem}
.enc-filter-chip,.enc-active-filter,.enc-filter-summary{font-size:.6875rem}
.compare-insight-head h3{font-size:.875rem}.compare-insight-select,.same-player-toggle{font-size:.75rem}
.delta-card .label,.compare-build .label{font-size:.6875rem}
.delta-card .route,.delta-card .delta,.compare-build .value,.skill-delta-title{font-size:.75rem}
.skill-delta-table{font-size:.75rem}.skill-delta-table th{font-size:.6875rem}.compare-note{font-size:.75rem}
.attribute-note{font-size:.75rem}.attribute-compare h3{font-size:.875rem}.attribute-compare-sub,.attribute-summary{font-size:.75rem}
.attribute-compare .table{font-size:.8125rem}.attribute-change-table .stat-delta{font-size:.6875rem}
.attribute-no-changes{font-size:.8125rem}.attribute-no-changes strong{font-size:.9375rem}
"#, "Encounter History")
}

pub fn upgrade_chat(path: &Path) -> Result<(), String> {
    inject(path, r#"
html{font-size:100%}
body{font-size:1rem;line-height:1.5}
.brand h1{font-size:1.125rem}.brand p,.local{font-size:.8125rem}
.archive-tabs a{font-size:.75rem}.side-title{font-size:.75rem}
.date-btn{font-size:.875rem}.date-count{font-size:.8125rem}
.toolbar h2{font-size:1.25rem}.toolbar-sub,.summary{font-size:.8125rem}
.control{font-size:.875rem}.archive-filter-btn,.archive-clear-btn{font-size:.8125rem}
.archive-filter-count{font-size:.6875rem}.filter-field>span{font-size:.6875rem}
.filter-check{font-size:.8125rem}.quick-filter,.active-filter-chip,.search-hint{font-size:.75rem}
.day-head h3{font-size:.875rem}.day-head span{font-size:.75rem}
.message{font-size:1rem}.time{font-size:.8125rem}.channel{font-size:.6875rem}.sender{font-size:.9375rem}
.message-text{font-size:1rem;line-height:1.55}.raw .message-text{font-size:.8125rem}
.empty{font-size:.875rem}.empty strong{font-size:1.0625rem}.footer{font-size:.75rem}.kbd{font-size:.6875rem}
"#, "Chat Archive")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_and_chat_use_browser_relative_readable_type() {
        let root=std::env::temp_dir().join(format!("readyalert-type-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir_all(&root).unwrap();
        let encounter=root.join("enc.html");\n        let chat=root.join("chat.html");
        fs::write(&encounter,"<html><body>x</body></html>").unwrap();
        fs::write(&chat,"<html><body>x</body></html>").unwrap();
        upgrade_encounter(&encounter).unwrap();upgrade_chat(&chat).unwrap();
        for path in [&encounter,&chat] {
            let text=fs::read_to_string(path).unwrap();
            assert!(text.contains(MARKER));
            assert!(text.contains("html{font-size:100%}"));
            assert!(text.contains("body{font-size:1rem"));
        }
        let _=fs::remove_dir_all(root);
    }
}
