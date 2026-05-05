//! Parser for `OP_EndUpdate` — payload `endUpdateStruct`, 10 bytes.
//! Run / jump endurance bar (the yellow bar under mana). Resolved
//! 2026-04-28 from a jump-drain capture (n=139, dominant 10-byte
//! S>C unknown). Self-only — `spawn_id` always matches the local
//! player.

use seq_eqstructs::endUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<endUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndUpdate {
    pub spawn_id: u16,
    pub cur: u32,
    pub max: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EndUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_end_update(bytes: &[u8]) -> Result<EndUpdate, EndUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(EndUpdateError::BadLength(bytes.len()));
    }
    let raw: endUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const endUpdateStruct) };
    Ok(EndUpdate {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawn_id).read_unaligned() },
        cur:      unsafe { std::ptr::addr_of!(raw.cur).read_unaligned() },
        max:      unsafe { std::ptr::addr_of!(raw.max).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_end_update(&[0; 9]).is_err());
        assert!(parse_end_update(&[0; 11]).is_err());
    }

    #[test]
    fn parses_jump_drain_tick() {
        // 10-byte packed payload {u16 spawn_id, u32 cur, u32 max}.
        let mut buf = [0u8; 10];
        buf[0..2].copy_from_slice(&0x1234u16.to_le_bytes());
        buf[2..6].copy_from_slice(&94u32.to_le_bytes());
        buf[6..10].copy_from_slice(&100u32.to_le_bytes());
        let e = parse_end_update(&buf).unwrap();
        assert_eq!(e.spawn_id, 0x1234);
        assert_eq!(e.cur, 94);
        assert_eq!(e.max, 100);
    }
}
