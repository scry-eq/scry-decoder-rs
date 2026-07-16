//! OP_LootMessage (0x7d46): personal auto-loot/sell text.
//! `{u32 color@0, NUL-term text@4}`; the text embeds `\x12`-wrapped links.

use thiserror::Error;

/// EQL item link = `\x12` + this many hex chars + item name + `\x12`.
const ITEM_LINK_HEX: usize = 197;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootMessage {
    pub color: u32,
    /// Message text with links reduced to the readable item name.
    pub text: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LootMessageError {
    #[error("expected at least 5 bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_loot_message(bytes: &[u8]) -> Result<LootMessage, LootMessageError> {
    if bytes.len() < 5 {
        return Err(LootMessageError::BadLength(bytes.len()));
    }
    let color = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let raw = &bytes[4..];
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    Ok(LootMessage {
        color,
        text: clean_links(&String::from_utf8_lossy(&raw[..end])),
    })
}

/// Replace each `\x12 <197 hex> name \x12` link with just `name`.
fn clean_links(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('\u{12}') {
        out.push_str(&rest[..open]);
        let body = &rest[open + 1..];
        match body.find('\u{12}') {
            Some(close) => {
                let link = &body[..close];
                out.push_str(link.get(ITEM_LINK_HEX..).unwrap_or(""));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str) -> String {
        format!("\u{12}{}{}\u{12}", "0".repeat(ITEM_LINK_HEX), name)
    }

    #[test]
    fn rejects_short() {
        assert_eq!(parse_loot_message(&[0; 4]), Err(LootMessageError::BadLength(4)));
    }

    #[test]
    fn cleans_item_link() {
        let mut b = 286u32.to_le_bytes().to_vec();
        b.extend_from_slice(format!("You looted a {} from a corpse", link("Fine Steel")).as_bytes());
        let m = parse_loot_message(&b).unwrap();
        assert_eq!(m.color, 286);
        assert_eq!(m.text, "You looted a Fine Steel from a corpse");
    }
}
