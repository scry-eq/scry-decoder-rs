//! OP_LootMessage: personal auto-loot/sell text.
//! `{u32 color@0, NUL-term text@4}`; the text embeds `\x12`-wrapped links.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootMessage {
    pub color: u32,
    /// Message text with links reduced to the readable item name.
    pub text: String,
    /// Item id off the FIRST item link's header, 0 when the line carries none.
    /// A loot line names exactly one item, so first-link is unambiguous.
    pub item_id: u32,
    /// That link's readable name, empty when the line carries no item link.
    /// Authoritative — the regexes only need to classify the disposition.
    pub item_name: String,
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
    // Read the link header before clean_links() strips it — the id is already
    // on the wire, so consumers never have to recover it by pairing on text.
    let text = String::from_utf8_lossy(&raw[..end]);
    let (item_id, item_name) = crate::links::first_item_link(&text).unwrap_or((0, String::new()));
    Ok(LootMessage {
        color,
        text: crate::links::clean_links(&text),
        item_id,
        item_name,
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

    #[test]
    fn carries_the_item_id_and_name() {
        // 002D56 = 11606, wire-verified. The id sits in the header clean_links
        // walks past, so it costs nothing to keep.
        let hex = format!("002D56{}", "0".repeat(crate::links::ITEM_LINK_HEX - 6));
        let mut b = 286u32.to_le_bytes().to_vec();
        b.extend_from_slice(
            format!("You looted a \u{12}{hex}Fine Steel\u{12} from a corpse").as_bytes(),
        );
        let m = parse_loot_message(&b).unwrap();
        assert_eq!(m.item_id, 11606);
        assert_eq!(m.item_name, "Fine Steel");
        assert_eq!(m.text, "You looted a Fine Steel from a corpse");
    }

    #[test]
    fn a_line_with_no_item_link_yields_no_id() {
        let mut b = 286u32.to_le_bytes().to_vec();
        b.extend_from_slice(b"You receive 2 platinum from the corpse.");
        let m = parse_loot_message(&b).unwrap();
        assert_eq!(m.item_id, 0);
        assert!(m.item_name.is_empty());
    }
}
