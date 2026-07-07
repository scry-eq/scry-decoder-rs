//! Parser for the post-2026-05-22 variable-size `OP_Buff` broadcast. The legacy
//! 168-byte `buffStruct` is dead. Wire forms cracked from captures:
//!
//! ```text
//!   Header (all forms): u32 spawnID @0, u32 spellID @4
//!   13b "fade"
//!   30b "initial sync": buff slot @9
//!   34+b "live update": block-1 duration ticks (u32) @15
//! ```
//!
//! This only extracts the wire fields. The application logic — the spell-DB
//! level-scaled duration for the 30b form, the self-spawn / null-spell filter,
//! and SpellItem management — stays daemon-side (it needs the Player + Spells
//! DB, which don't cross the FFI).

use thiserror::Error;

/// Buff wire-form discriminators (see [`Buff::form`]).
pub const FORM_FADE: u8 = 0;
pub const FORM_INITIAL: u8 = 1;
pub const FORM_UPDATE: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buff {
    pub spawn_id: u32,
    pub spell_id: u32,
    /// [`FORM_FADE`] (13b) | [`FORM_INITIAL`] (30b) | [`FORM_UPDATE`] (34+b).
    pub form: u8,
    /// Buff slot — valid for [`FORM_INITIAL`] only (`0xff` otherwise).
    pub slot: u8,
    /// Block-1 duration in ticks — valid for [`FORM_UPDATE`] only (0 otherwise).
    pub dur_ticks: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuffError {
    #[error("payload too short: {0} bytes")]
    Short(usize),
    #[error("unrecognized OP_Buff form: {0} bytes")]
    BadForm(usize),
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse_buff(bytes: &[u8]) -> Result<Buff, BuffError> {
    if bytes.len() < 8 {
        return Err(BuffError::Short(bytes.len()));
    }
    let spawn_id = rd_u32(bytes, 0);
    let spell_id = rd_u32(bytes, 4);

    let (form, slot, dur_ticks) = match bytes.len() {
        13 => (FORM_FADE, 0xff, 0),
        30 => (FORM_INITIAL, bytes[9], 0),
        // block 1 starts after the 15-byte header; its tick count is the
        // player's remaining time.
        n if n >= 34 => (FORM_UPDATE, 0xff, rd_u32(bytes, 15)),
        n => return Err(BuffError::BadForm(n)),
    };

    Ok(Buff { spawn_id, spell_id, form, slot, dur_ticks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short() {
        assert!(parse_buff(&[0u8; 7]).is_err());
    }

    #[test]
    fn rejects_unrecognized_form() {
        assert!(matches!(parse_buff(&[0u8; 20]), Err(BuffError::BadForm(20))));
    }

    #[test]
    fn fade_form_13b() {
        let mut b = [0u8; 13];
        b[0..4].copy_from_slice(&123u32.to_le_bytes());
        b[4..8].copy_from_slice(&5024u32.to_le_bytes());
        let out = parse_buff(&b).unwrap();
        assert_eq!(out.spawn_id, 123);
        assert_eq!(out.spell_id, 5024);
        assert_eq!(out.form, FORM_FADE);
    }

    #[test]
    fn initial_form_30b_reads_slot() {
        let mut b = [0u8; 30];
        b[0..4].copy_from_slice(&7u32.to_le_bytes());
        b[4..8].copy_from_slice(&42u32.to_le_bytes());
        b[9] = 3; // slot
        let out = parse_buff(&b).unwrap();
        assert_eq!(out.form, FORM_INITIAL);
        assert_eq!(out.slot, 3);
    }

    #[test]
    fn update_form_reads_dur_ticks() {
        let mut b = [0u8; 34];
        b[0..4].copy_from_slice(&7u32.to_le_bytes());
        b[4..8].copy_from_slice(&42u32.to_le_bytes());
        b[15..19].copy_from_slice(&600u32.to_le_bytes()); // dur ticks
        let out = parse_buff(&b).unwrap();
        assert_eq!(out.form, FORM_UPDATE);
        assert_eq!(out.dur_ticks, 600);
    }
}
