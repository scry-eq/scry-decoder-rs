//! Reduce EQ `\x12`-wrapped links to their readable name.

/// EQL item link = `\x12` + this many hex chars + item name + `\x12`.
pub const ITEM_LINK_HEX: usize = 197;

/// The EQ item id is the leading 6 hex chars of an item-link body (big-endian),
/// e.g. "0057F5…" -> 22517. Wire-verified across the eql-fighting fixture
/// (consecutive Lady-Vox loot ids 002D56/002D57 = 11606/11607; Motes 02446E =
/// 148590 confirm the field is 6 wide). Spell links (which carry `^` args) and
/// short bodies yield 0.
pub fn item_id_from_link(body: &[u8]) -> u32 {
    std::str::from_utf8(body.get(0..6).unwrap_or(&[]))
        .ok()
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .unwrap_or(0)
}

/// Replace each `\x12 … \x12` span with its name. Spell links carry caret args
/// (`fmt^id^..^'Name`) — keep the trailing name; item links are a fixed hex
/// header + name — skip the header.
pub fn clean_links(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('\u{12}') {
        out.push_str(&rest[..open]);
        let body = &rest[open + 1..];
        match body.find('\u{12}') {
            Some(close) => {
                out.push_str(&link_name(&body[..close]));
                rest = &body[close + 1..];
            }
            None => {
                out.push_str(body); // unterminated — keep as-is
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

fn link_name(link: &str) -> String {
    match link.rfind('^') {
        Some(i) => link[i + 1..].trim_start_matches('\'').to_string(), // spell link
        None => link.get(ITEM_LINK_HEX..).unwrap_or("").to_string(),   // item link
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_link() {
        assert_eq!(clean_links("cast \u{12}63^3686^0^1^'Blood of Pain\u{12}!"),
                   "cast Blood of Pain!");
    }

    #[test]
    fn item_link() {
        let l = format!("got \u{12}{}Fine Steel\u{12}", "0".repeat(ITEM_LINK_HEX));
        assert_eq!(clean_links(&l), "got Fine Steel");
    }

    #[test]
    fn plain_passthrough() {
        assert_eq!(clean_links("no links here"), "no links here");
    }
}
