//! Parser for `OP_LevelUpdate` — payload `levelUpUpdateStruct`,
//! 16 bytes. Fires once on level transition; `exp` is the post-level
//! exp value that cross-references the next OP_ExpUpdate.

use seq_eqstructs::levelUpUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<levelUpUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUpdate {
    pub level: u32,
    pub level_old: u32,
    pub exp: u32,
    pub unknown0: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LevelUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_level_update(bytes: &[u8]) -> Result<LevelUpdate, LevelUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(LevelUpdateError::BadLength(bytes.len()));
    }
    let raw: levelUpUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const levelUpUpdateStruct) };
    Ok(LevelUpdate {
        level:     unsafe { std::ptr::addr_of!(raw.level).read_unaligned() },
        level_old: unsafe { std::ptr::addr_of!(raw.levelOld).read_unaligned() },
        exp:       unsafe { std::ptr::addr_of!(raw.exp).read_unaligned() },
        unknown0:  unsafe { std::ptr::addr_of!(raw.unknown0012).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_level_update(&[0; 15]).is_err());
        assert!(parse_level_update(&[0; 17]).is_err());
    }

    #[test]
    fn parses_fields() {
        // Sample bytes from 2026-05-01 confirmation log:
        // {level=2, levelOld=1, exp=814}
        let buf = [
            0x02, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x2e, 0x03, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let l = parse_level_update(&buf).unwrap();
        assert_eq!(l.level, 2);
        assert_eq!(l.level_old, 1);
        assert_eq!(l.exp, 814);
    }
}
