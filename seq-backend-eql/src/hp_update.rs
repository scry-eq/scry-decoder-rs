//! Parser for `OP_HPUpdate` — payload `hpNpcUpdateStruct`, 18 bytes.
//! Two u32 placeholder fields (added 2019-06-19) are skipped — the
//! daemon's C++ path discards them too. Output keeps the three
//! semantic fields plus the spawn id.

use crate::eqstructs::hpNpcUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<hpNpcUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpUpdate {
    pub spawn_id: u16,
    pub cur_hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HpUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_hp_update(bytes: &[u8]) -> Result<HpUpdate, HpUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(HpUpdateError::BadLength(bytes.len()));
    }
    let raw: hpNpcUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const hpNpcUpdateStruct) };
    Ok(HpUpdate {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        cur_hp:   unsafe { std::ptr::addr_of!(raw.curHP).read_unaligned() },
        max_hp:   unsafe { std::ptr::addr_of!(raw.maxHP).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_hp_update(&[0; 17]).is_err());
        assert!(parse_hp_update(&[0; 19]).is_err());
    }

    #[test]
    fn parses_fields_with_negative_hp() {
        let mut buf = [0u8; 18];
        buf[0..2].copy_from_slice(&0xABCDu16.to_le_bytes());
        buf[2..6].copy_from_slice(&(-1234i32).to_le_bytes()); // curHP
        // bytes 6..10 = unknown0006 — set to garbage; should be ignored.
        buf[6..10].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        buf[10..14].copy_from_slice(&5678i32.to_le_bytes()); // maxHP
        let r = parse_hp_update(&buf).unwrap();
        assert_eq!(r.spawn_id, 0xABCD);
        assert_eq!(r.cur_hp, -1234);
        assert_eq!(r.max_hp, 5678);
    }
}
