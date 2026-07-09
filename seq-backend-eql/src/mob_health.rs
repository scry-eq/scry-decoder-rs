//! Parser for `OP_MobHealth` — payload `mobHealthStruct`, 6 bytes.
//! `hpPercent` is the spawn's HP as a percentage (0..100), not raw HP
//! (resolved 2026-05-01 — see OPCODES_LIVE_TODO.md).

use crate::eqstructs::mobHealthStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<mobHealthStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobHealth {
    pub spawn_id: u16,
    pub hp_percent: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MobHealthError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_mob_health(bytes: &[u8]) -> Result<MobHealth, MobHealthError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(MobHealthError::BadLength(bytes.len()));
    }
    let raw: mobHealthStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const mobHealthStruct) };
    Ok(MobHealth {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        hp_percent: unsafe { std::ptr::addr_of!(raw.hpPercent).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_mob_health(&[0; 5]).is_err());
        assert!(parse_mob_health(&[0; 7]).is_err());
    }

    #[test]
    fn parses_percentage() {
        let mut buf = [0u8; 6];
        buf[0..2].copy_from_slice(&199u16.to_le_bytes());
        buf[2..6].copy_from_slice(&100i32.to_le_bytes());
        let m = parse_mob_health(&buf).unwrap();
        assert_eq!(m.spawn_id, 199);
        assert_eq!(m.hp_percent, 100);
    }
}
