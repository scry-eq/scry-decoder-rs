//! Parser for `OP_AAExpUpdate` — payload `altExpUpdateStruct`, 12 bytes.
//! `alt_exp` is progress toward the next AA point and `aa_points` the
//! unspent count; `percent` is the same progress pre-rounded by the
//! server (the daemon prefers the raw value).

use crate::eqstructs::altExpUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<altExpUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AltExpUpdate {
    pub alt_exp: u32,
    pub aa_points: u32,
    pub percent: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AltExpUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_alt_exp_update(bytes: &[u8]) -> Result<AltExpUpdate, AltExpUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(AltExpUpdateError::BadLength(bytes.len()));
    }
    let raw: altExpUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const altExpUpdateStruct) };
    Ok(AltExpUpdate {
        alt_exp: unsafe { std::ptr::addr_of!(raw.altexp).read_unaligned() },
        aa_points: unsafe { std::ptr::addr_of!(raw.aapoints).read_unaligned() },
        percent: unsafe { std::ptr::addr_of!(raw.percent).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_alt_exp_update(&[0; 11]).is_err());
        assert!(parse_alt_exp_update(&[0; 13]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&91_234u32.to_le_bytes());
        buf[4..8].copy_from_slice(&317u32.to_le_bytes());
        buf[8] = 91;
        let a = parse_alt_exp_update(&buf).unwrap();
        assert_eq!(a.alt_exp, 91_234);
        assert_eq!(a.aa_points, 317);
        assert_eq!(a.percent, 91);
    }
}
