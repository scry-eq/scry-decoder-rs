//! OP_LootMessage (0x7d46): personal auto-loot/sell text.
//! `{u32 color@0, NUL-term text@4}`; the text embeds `\x12`-wrapped links.

use thiserror::Error;

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
        text: crate::links::clean_links(&String::from_utf8_lossy(&raw[..end])),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str) -> String {
        format!(
            "\u{12}{}{}\u{12}",
            "0".repeat(crate::links::ITEM_LINK_HEX),
            name
        )
    }

    #[test]
    fn rejects_short() {
        assert_eq!(
            parse_loot_message(&[0; 4]),
            Err(LootMessageError::BadLength(4))
        );
    }

    #[test]
    fn cleans_item_link() {
        let mut b = 286u32.to_le_bytes().to_vec();
        b.extend_from_slice(
            format!("You looted a {} from a corpse", link("Fine Steel")).as_bytes(),
        );
        let m = parse_loot_message(&b).unwrap();
        assert_eq!(m.color, 286);
        assert_eq!(m.text, "You looted a Fine Steel from a corpse");
    }
}
