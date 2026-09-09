pub const CHAT_RECOVERY: &str = "Ctrl+Shift+F9";
pub const COMBAT: &str = "Ctrl+Shift+F10";

pub fn parse(value: &str) -> Option<(u32,u32)> {
    let mut mods=0; let mut key=None;
    for token in value.split('+').map(str::trim) {
        match token.to_ascii_lowercase().as_str() {
            "ctrl"|"control" => mods |= 2,
            "shift" => mods |= 4,
            "alt" => mods |= 1,
            "win"|"windows" => mods |= 8,
            other => {
                if key.is_some() { return None; }
                let upper=other.to_ascii_uppercase();
                let vk=if let Some(rest)=upper.strip_prefix('F') {
                    rest.parse::<u32>().ok().filter(|n| (1..=24).contains(n)).map(|n| 0x70+n-1)
                } else if upper.len()==1 {
                    upper.bytes().next().filter(|b| b.is_ascii_alphanumeric()).map(u32::from)
                } else { None };
                key=Some(vk?);
            }
        }
    }
    key.map(|vk| (mods|0x4000,vk))
}

pub fn is_combat(value: &str) -> bool { parse(value)==parse(COMBAT) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reordered_legacy_chord_is_reserved() {
        assert!(is_combat("shift + Control + f10"));
        assert_ne!(parse(CHAT_RECOVERY),parse(COMBAT));
        for invalid in ["Ctrl+Shift+F9+F10", "Ctrl+wat+F9", "Ctrl+", ""] { assert!(parse(invalid).is_none()); }
        assert!(parse("Alt+F8").is_some());
    }
}
