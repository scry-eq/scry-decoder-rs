//! Parser for `OP_Consider` — payload `considerStruct`, 32 bytes.
//! Only the first four fields (player/target ids, faction, level) drive
//! observable behavior; the four trailing u32 unknowns are ignored.

use seq_eqstructs_live::considerStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<considerStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Consider {
    pub player_id: u32,
    pub target_id: u32,
    pub faction: i32,
    pub level: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConsiderError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_consider(bytes: &[u8]) -> Result<Consider, ConsiderError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ConsiderError::BadLength(bytes.len()));
    }
    let raw: considerStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const considerStruct) };
    Ok(Consider {
        player_id: unsafe { std::ptr::addr_of!(raw.playerid).read_unaligned() },
        target_id: unsafe { std::ptr::addr_of!(raw.targetid).read_unaligned() },
        faction:   unsafe { std::ptr::addr_of!(raw.faction).read_unaligned() },
        level:     unsafe { std::ptr::addr_of!(raw.level).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_consider(&[0; 31]).is_err());
        assert!(parse_consider(&[0; 33]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&100u32.to_le_bytes());
        buf[4..8].copy_from_slice(&200u32.to_le_bytes());
        buf[8..12].copy_from_slice(&(-1i32).to_le_bytes()); // faction
        buf[12..16].copy_from_slice(&50i32.to_le_bytes());
        let c = parse_consider(&buf).unwrap();
        assert_eq!(c.player_id, 100);
        assert_eq!(c.target_id, 200);
        assert_eq!(c.faction, -1);
        assert_eq!(c.level, 50);
    }
}
