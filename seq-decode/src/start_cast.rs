//! Parser for `OP_CastSpell` — payload `startCastStruct`, 39 bytes.
//! Daemon reads slot, spell_id, target_id.

use seq_eqstructs::startCastStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<startCastStruct>();

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
    let raw: startCastStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const startCastStruct) };
    Ok(StartCast {
        slot:      unsafe { std::ptr::addr_of!(raw.slot).read_unaligned() },
        spell_id:  unsafe { std::ptr::addr_of!(raw.spellId).read_unaligned() },
        target_id: unsafe { std::ptr::addr_of!(raw.targetId).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_start_cast(&[0; 38]).is_err());
        assert!(parse_start_cast(&[0; 40]).is_err());
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
}
