//! Parser for `OP_ExpUpdate` — payload `expUpdateStruct`, 16 bytes.
//! `exp` is per-level progress 0..100000 (0.001% granularity) and
//! `kind` is the update flavor (0=set, 2=update). Two u32 unknowns
//! sandwich `kind`; preserved as raw fields so cross-decoder bytes
//! stay byte-identical.

use crate::eqstructs::expUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<expUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpUpdate {
    pub exp: u32,
    pub unknown0: u32,
    pub kind: u32,
    pub unknown1: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExpUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_exp_update(bytes: &[u8]) -> Result<ExpUpdate, ExpUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ExpUpdateError::BadLength(bytes.len()));
    }
    let raw: expUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const expUpdateStruct) };
    Ok(ExpUpdate {
        exp:       unsafe { std::ptr::addr_of!(raw.exp).read_unaligned() },
        unknown0:  unsafe { std::ptr::addr_of!(raw.unknown0004).read_unaligned() },
        kind:      unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
        unknown1:  unsafe { std::ptr::addr_of!(raw.unknown0012).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_exp_update(&[0; 15]).is_err());
        assert!(parse_exp_update(&[0; 17]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&97900u32.to_le_bytes());
        buf[4..8].copy_from_slice(&0u32.to_le_bytes());
        buf[8..12].copy_from_slice(&2u32.to_le_bytes()); // type=update
        buf[12..16].copy_from_slice(&0u32.to_le_bytes());
        let e = parse_exp_update(&buf).unwrap();
        assert_eq!(e.exp, 97900);
        assert_eq!(e.kind, 2);
    }
}
