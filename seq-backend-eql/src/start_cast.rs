//! Parser for `OP_CastSpell` — eql's C>S cast request, 44 bytes.
//! Daemon reads slot, spell_id, target_id.
//!
//! Size validated locally 2026-08-03 (first local sighting; the id came from
//! upstream unvalidated and the gate was pinned at Live's 40, so every packet
//! was size-dropped). The three consumed fields kept their Live offsets — the
//! record grew at the tail:
//!
//! ```text
//!   /*@0*/  i32  slot          gem slot
//!   /*@4*/  u32  spellId
//!   /*@8*/  u8[10]             0xff filler
//!   /*@18*/ u32  targetId      0 when cast with no target
//!   /*@22*/ u32                per-spell constant (same value every cast of a
//!                              given spell; role unmapped)
//!   /*@26*/ ...                zero through @43, except a 1 at @31
//! ```
//!
//! Offsets confirmed against 10 captured casts of three spells: slot/spellId
//! read 15 (Greater Healing), 235 (Invisibility Versus Undead) and 191
//! (Feedback), matching what was cast; targetId read 0 for untargeted casts and
//! two distinct spawn ids for the targeted ones.
//!
//! Read at explicit offsets rather than through the pinned `startCastStruct`
//! binding: that struct is 39 bytes, so its `size_of` no longer describes this
//! wire and must not drive the gate.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartCast {
    pub slot: i32,
    pub spell_id: u32,
    pub target_id: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StartCastError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_start_cast(bytes: &[u8]) -> Result<StartCast, StartCastError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(StartCastError::BadLength(bytes.len()));
    }
    let u32_at = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    Ok(StartCast {
        slot:      u32_at(0) as i32,
        spell_id:  u32_at(4),
        target_id: u32_at(18),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        // 39/40 are the pre-2026-08-03 Live struct size and the old eql gate.
        assert!(parse_start_cast(&[0; 39]).is_err());
        assert!(parse_start_cast(&[0; 40]).is_err());
        assert!(parse_start_cast(&[0; 45]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&2i32.to_le_bytes());
        buf[4..8].copy_from_slice(&12345u32.to_le_bytes());
        buf[18..22].copy_from_slice(&999u32.to_le_bytes());
        let s = parse_start_cast(&buf).unwrap();
        assert_eq!(s.slot, 2);
        assert_eq!(s.spell_id, 12345);
        assert_eq!(s.target_id, 999);
    }

    // Real 44B captures. Targeted and untargeted casts of two spells, so the
    // gate size and all three offsets are pinned to wire bytes.
    #[test]
    fn decodes_captured_casts() {
        let untargeted: [u8; PAYLOAD_LEN] = [
            0x03, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x03, 0x91, 0xC3, 0x3D, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let s = parse_start_cast(&untargeted).unwrap();
        assert_eq!((s.slot, s.spell_id, s.target_id), (3, 15, 0));

        let targeted: [u8; PAYLOAD_LEN] = [
            0x0C, 0x00, 0x00, 0x00, 0xEB, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xC2, 0x1C, 0x00, 0x00, 0x3C, 0xE0, 0x46, 0x34, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let s = parse_start_cast(&targeted).unwrap();
        assert_eq!((s.slot, s.spell_id, s.target_id), (12, 235, 7362));
    }
}
