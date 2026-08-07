//! Parser for `OP_Stamina` — payload `staminaStruct`, 8 bytes.
//! Hunger / thirst (NOT the run/jump endurance bar — that's
//! `OP_EndUpdate`). Both fields are u32 ticks-till-next-eat / drink,
//! capped at ~127 in practice.

use crate::eqstructs::staminaStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<staminaStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamina {
    pub food: u32,
    pub water: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StaminaError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_stamina(bytes: &[u8]) -> Result<Stamina, StaminaError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(StaminaError::BadLength(bytes.len()));
    }
    let raw: staminaStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const staminaStruct) };
    Ok(Stamina {
        food: unsafe { std::ptr::addr_of!(raw.food).read_unaligned() },
        water: unsafe { std::ptr::addr_of!(raw.water).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_stamina(&[0; 7]).is_err());
        assert!(parse_stamina(&[0; 9]).is_err());
    }

    #[test]
    fn parses_typical() {
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(&127u32.to_le_bytes());
        buf[4..8].copy_from_slice(&0u32.to_le_bytes());
        let s = parse_stamina(&buf).unwrap();
        assert_eq!(s.food, 127);
        assert_eq!(s.water, 0);
    }
}
