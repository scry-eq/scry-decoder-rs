//! Parser for the post-2026-05-22 variable-size `OP_Buff` broadcast. The legacy
//! 168-byte `buffStruct` is dead. Wire forms cracked from captures:
//!
//! ```text
//!   Header (all forms): u32 spawnID @0, u32 spellID @4
//!   13b "fade"
//!   30b "initial sync": buff slot @9
//!   34+b "live update": block-1 duration ticks (u32) @15
//!   24b  "compact":     buff SLOT @0 (not a spawn id), spellID @4,
//!                       changeType @12 (1 = faded, 4 = applied)
//! ```
//!
//! The 24b compact record is the eql buff-slot channel. Note its @0 is a buff
//! SLOT, not a spawn id like every other form — it always describes the local
//! player's own buff window, so callers must not apply a spawn-id filter to it.
//! Slots 0-14 are real buff-window entries; higher values are scribe / bar
//! refreshes and are reported as slot 0xff so callers can drop them. Layout
//! credit: legacy showeq SpellShell::buffChange.
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
pub const FORM_COMPACT: u8 = 3;

/// `change_type` values carried by [`FORM_COMPACT`].
pub const CHANGE_FADED: u32 = 1;
pub const CHANGE_APPLIED: u32 = 4;

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
    /// Apply/fade code — valid for [`FORM_COMPACT`] only (0 otherwise).
    pub change_type: u32,
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
    let spell_id = rd_u32(bytes, 4);

    // The compact form reuses offset 0 for a buff slot rather than a spawn id,
    // so it is resolved before the spawn-id read below.
    if bytes.len() == 24 {
        let raw_slot = rd_u32(bytes, 0);
        return Ok(Buff {
            spawn_id: 0,
            spell_id,
            form: FORM_COMPACT,
            slot: if raw_slot < 15 { raw_slot as u8 } else { 0xff },
            dur_ticks: 0,
            change_type: rd_u32(bytes, 12),
        });
    }

    let spawn_id = rd_u32(bytes, 0);

    let (form, slot, dur_ticks) = match bytes.len() {
        13 => (FORM_FADE, 0xff, 0),
        30 => (FORM_INITIAL, bytes[9], 0),
        // block 1 starts after the 15-byte header; its tick count is the
        // player's remaining time.
        n if n >= 34 => (FORM_UPDATE, 0xff, rd_u32(bytes, 15)),
        n => return Err(BuffError::BadForm(n)),
    };

    Ok(Buff {
        spawn_id,
        spell_id,
        form,
        slot,
        dur_ticks,
        change_type: 0,
    })
}

#[cfg(test)]
mod compact_tests {
    use super::*;

    fn rec(slot: u32, spell: u32, change: u32) -> [u8; 24] {
        let mut b = [0u8; 24];
        b[0..4].copy_from_slice(&slot.to_le_bytes());
        b[4..8].copy_from_slice(&spell.to_le_bytes());
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        b[12..16].copy_from_slice(&change.to_le_bytes());
        b
    }

    #[test]
    fn parses_an_applied_record() {
        // Captured verbatim: slot 0, spell 296, applied.
        let m = parse_buff(&rec(0, 296, CHANGE_APPLIED)).unwrap();
        assert_eq!(m.form, FORM_COMPACT);
        assert_eq!(m.slot, 0);
        assert_eq!(m.spell_id, 296);
        assert_eq!(m.change_type, CHANGE_APPLIED);
    }

    #[test]
    fn parses_a_faded_record() {
        // Captured verbatim: slot 2, spell 231, faded.
        let m = parse_buff(&rec(2, 231, CHANGE_FADED)).unwrap();
        assert_eq!(m.slot, 2);
        assert_eq!(m.spell_id, 231);
        assert_eq!(m.change_type, CHANGE_FADED);
    }

    #[test]
    fn flags_scribe_slots_as_ignorable() {
        // Slots >= 15 are bar/scribe refreshes, not buff-window entries; 128 of
        // 162 records in one capture were these (331..339 and similar).
        assert_eq!(parse_buff(&rec(331, 4010, 0)).unwrap().slot, 0xff);
        assert_eq!(parse_buff(&rec(15, 1, 0)).unwrap().slot, 0xff);
        assert_eq!(parse_buff(&rec(14, 1, 0)).unwrap().slot, 14);
    }

    #[test]
    fn does_not_disturb_the_other_forms() {
        // 24 must not be swallowed by the >= 34 arm, and 13/30 keep spawn_id@0.
        let mut b13 = [0u8; 13];
        b13[0..4].copy_from_slice(&12345u32.to_le_bytes());
        let f = parse_buff(&b13).unwrap();
        assert_eq!(f.form, FORM_FADE);
        assert_eq!(f.spawn_id, 12345);
        assert_eq!(f.change_type, 0);
    }
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
        assert!(matches!(
            parse_buff(&[0u8; 20]),
            Err(BuffError::BadForm(20))
        ));
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
