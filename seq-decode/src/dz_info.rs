//! Parser for `OP_DzInfo` — payload `dzInfo`, 212 bytes. The
//! daemon reads `new_dz` (clears DZ-state when zero); the rest is
//! surfaced for completeness.

use seq_eqstructs::dzInfo;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<dzInfo>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DzInfo {
    pub new_dz: u8,
    pub max_players: u32,
    pub dz_name: [u8; 128],
    pub name: [u8; 64],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DzInfoError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_dz_info(bytes: &[u8]) -> Result<DzInfo, DzInfoError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(DzInfoError::BadLength(bytes.len()));
    }
    let raw: dzInfo =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const dzInfo) };
    let dz_name = unsafe { std::ptr::addr_of!(raw.dzName).read_unaligned() };
    let name    = unsafe { std::ptr::addr_of!(raw.name).read_unaligned() };
    Ok(DzInfo {
        new_dz:      unsafe { std::ptr::addr_of!(raw.newDZ).read_unaligned() },
        max_players: unsafe { std::ptr::addr_of!(raw.maxPlayers).read_unaligned() },
        dz_name,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_dz_info(&[0; 211]).is_err());
        assert!(parse_dz_info(&[0; 213]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[8] = 1; // newDZ
        buf[12..16].copy_from_slice(&6u32.to_le_bytes()); // maxPlayers
        buf[16..21].copy_from_slice(b"DZ-9!");
        buf[144..148].copy_from_slice(b"Bob\0");
        let d = parse_dz_info(&buf).unwrap();
        assert_eq!(d.new_dz, 1);
        assert_eq!(d.max_players, 6);
        assert_eq!(&d.dz_name[..5], b"DZ-9!");
        assert_eq!(&d.name[..3], b"Bob");
    }
}
