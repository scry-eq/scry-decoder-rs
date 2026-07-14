//! Parser for `OP_FormattedMessage` (EQL id 0x3c0a).
//!
//! EQL diverges from Live here: the format id is NOT at the Live
//! `formattedMessageStruct` offset (5). Wire layout verified against the
//! eqlegends-corpsepin capture (750 packets, 2026-07-14):
//!
//! ```text
//!   u32 spellId  @0   0xffffffff on non-spell strings; a real spell id
//!                     on spell classes (233 = Expulse Undead, ...)
//!   u8  msgType  @4   message-class discriminator (see below)
//!   u32 spawnId  @5   actor spawn id (the player's self-id observed here
//!                     on self-directed messages)
//!   u32 formatId @9   eqstr format-string id (439 interrupt/heal, ...)
//!   args         @13  NUL-terminated substitution fields; link fields are
//!                     \x12-bracketed, caret-delimited EQ links
//! ```
//!
//! `msgType` multiplexes several kinds onto this one opcode. Observed:
//!   * 7 / 5 / 8 — overhead damage/heal floaters (one numeric arg, fmt 15566)
//!   * 0 / 1     — spell cast / heal / interrupt text (fmt 173 / 439, link arg)
//!   * 1 + name  — NPC-cast-at-you (a name arg followed by a spell-link arg)
//!
//! The parser surfaces the header fields plus the raw substitution list;
//! eqstr format-string lookup, `%N` interpolation and link cleanup are the
//! daemon's job (EQStr owns the string DB), exactly as the Live path slices
//! its arg blob C++-side.

use thiserror::Error;

/// Fixed header length; the substitution blob starts here.
pub const HEADER_LEN: usize = 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedMessage {
    /// Spell id at @0; `0xffffffff` marks a non-spell string.
    pub spell_id: u32,
    /// Message-class discriminator at @4.
    pub msg_type: u8,
    /// Actor spawn id at @5.
    pub spawn_id: u32,
    /// eqstr format-string id at @9.
    pub format_id: u32,
    /// Ordered substitution fields sliced from the trailing blob (@13),
    /// split on NUL. Link fields keep their raw `\x12…^…^…\x12` wrapper so
    /// the daemon can strip them with its existing link cleanup.
    pub args: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormattedMessageError {
    #[error("expected at least {HEADER_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_formatted_message(
    bytes: &[u8],
) -> Result<FormattedMessage, FormattedMessageError> {
    if bytes.len() < HEADER_LEN {
        return Err(FormattedMessageError::BadLength(bytes.len()));
    }
    // Fixed-offset reads; `try_into` on 4-byte windows can't fail here.
    let spell_id = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let msg_type = bytes[4];
    let spawn_id = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
    let format_id = u32::from_le_bytes(bytes[9..13].try_into().unwrap());
    let args = split_args(&bytes[HEADER_LEN..]);
    Ok(FormattedMessage {
        spell_id,
        msg_type,
        spawn_id,
        format_id,
        args,
    })
}

/// Split the trailing arg blob into NUL-terminated substitution fields.
/// A single trailing empty field (from the terminating NUL) is dropped;
/// embedded empty fields are preserved so positional `%N` indices stay
/// aligned. Bytes are decoded lossily — the caret links are ASCII inside a
/// `\x12` control wrapper, so this never loses a real character.
fn split_args(blob: &[u8]) -> Vec<String> {
    if blob.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = blob
        .split(|&b| b == 0)
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect();
    if out.last().is_some_and(|s| s.is_empty()) {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 13-byte header with the given fields, then append `tail`.
    fn pkt(spell: u32, ty: u8, spawn: u32, fmt: u32, tail: &[u8]) -> Vec<u8> {
        let mut b = Vec::with_capacity(HEADER_LEN + tail.len());
        b.extend_from_slice(&spell.to_le_bytes());
        b.push(ty);
        b.extend_from_slice(&spawn.to_le_bytes());
        b.extend_from_slice(&fmt.to_le_bytes());
        b.extend_from_slice(tail);
        b
    }

    #[test]
    fn rejects_short_payload() {
        assert_eq!(
            parse_formatted_message(&[0u8; 12]),
            Err(FormattedMessageError::BadLength(12))
        );
    }

    #[test]
    fn header_only_has_no_args() {
        // 13-byte packet, empty blob.
        let m = parse_formatted_message(&pkt(0xffff_ffff, 3, 12636, 15562, b"")).unwrap();
        assert_eq!(m.spell_id, 0xffff_ffff);
        assert_eq!(m.msg_type, 3);
        assert_eq!(m.spawn_id, 12636);
        assert_eq!(m.format_id, 15562);
        assert!(m.args.is_empty());
    }

    #[test]
    fn overhead_floater_single_numeric_arg() {
        // corpsepin: type=7 fmt=15566 tail "189\0".
        let m = parse_formatted_message(&pkt(665, 7, 40000, 15566, b"189\x00")).unwrap();
        assert_eq!(m.spell_id, 665);
        assert_eq!(m.msg_type, 7);
        assert_eq!(m.format_id, 15566);
        assert_eq!(m.args, vec!["189".to_string()]);
    }

    #[test]
    fn self_spell_message_keeps_raw_link() {
        // corpsepin: type=0 fmt=173 spell=233, self-id @5, one link arg.
        let tail = b"\x1263^233^0^0^'Expulse Undead\x12\x00";
        let m = parse_formatted_message(&pkt(233, 0, 12636, 173, tail)).unwrap();
        assert_eq!(m.spell_id, 233);
        assert_eq!(m.spawn_id, 12636);
        assert_eq!(m.format_id, 173);
        assert_eq!(m.args, vec!["\x1263^233^0^0^'Expulse Undead\x12".to_string()]);
    }

    #[test]
    fn npc_cast_splits_name_then_link() {
        // corpsepin: type=1 fmt=12478, "a tal ghoul wizard\0<link>\0".
        let tail = b"a tal ghoul wizard\x00\x1263^503^0^0^'Tishan's Clash\x12\x00";
        let m = parse_formatted_message(&pkt(38, 1, 20001, 12478, tail)).unwrap();
        assert_eq!(m.format_id, 12478);
        assert_eq!(
            m.args,
            vec![
                "a tal ghoul wizard".to_string(),
                "\x1263^503^0^0^'Tishan's Clash\x12".to_string(),
            ]
        );
    }

    #[test]
    fn preserves_embedded_empty_fields() {
        // Two NULs in a row → an embedded empty arg is kept (positional
        // %N alignment), only the final terminator is dropped.
        let m = parse_formatted_message(&pkt(1, 1, 2, 3, b"a\x00\x00b\x00")).unwrap();
        assert_eq!(m.args, vec!["a".to_string(), String::new(), "b".to_string()]);
    }
}
