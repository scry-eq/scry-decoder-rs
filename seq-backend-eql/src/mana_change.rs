//! Parser for `OP_ManaChange` — payload `manaDecrementStruct`,
//! 20 bytes. The daemon reads `newMana` for the live mana value; the
//! `curEndurance` field (formerly mislabelled `maxMana` in everquest.h) and
//! the last spell id ride along. The daemon computes max mana client-side,
//! so this `max_mana` output carries the raw curEndurance bytes and is unused.

use crate::eqstructs::manaDecrementStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<manaDecrementStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaChange {
    pub new_mana: i32,
    pub max_mana: i32,
    pub spell_id: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManaChangeError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_mana_change(bytes: &[u8]) -> Result<ManaChange, ManaChangeError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ManaChangeError::BadLength(bytes.len()));
    }
    let raw: manaDecrementStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const manaDecrementStruct) };
    Ok(ManaChange {
        new_mana: unsafe { std::ptr::addr_of!(raw.newMana).read_unaligned() },
        max_mana: unsafe { std::ptr::addr_of!(raw.curEndurance).read_unaligned() },
        spell_id: unsafe { std::ptr::addr_of!(raw.spellId).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_mana_change(&[0; 19]).is_err());
        assert!(parse_mana_change(&[0; 21]).is_err());
    }

    #[test]
    fn parses_negative_mana_delta() {
        let mut buf = [0u8; 20];
        buf[0..4].copy_from_slice(&(-100i32).to_le_bytes());
        buf[4..8].copy_from_slice(&0i32.to_le_bytes());
        buf[8..12].copy_from_slice(&42i32.to_le_bytes());
        let m = parse_mana_change(&buf).unwrap();
        assert_eq!(m.new_mana, -100);
        assert_eq!(m.spell_id, 42);
    }
}
