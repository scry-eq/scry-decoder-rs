//! Parser for `OP_WearChange` — payload `SpawnUpdateStruct`, 32
//! bytes. Daemon uses spawn_id + subcommand + arg1; arg2/arg3 are
//! decoded for completeness.

use seq_eqstructs::SpawnUpdateStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<SpawnUpdateStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WearChange {
    pub spawn_id: u16,
    pub subcommand: u16,
    pub arg1: i16,
    pub arg2: i16,
    pub arg3: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WearChangeError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_wear_change(bytes: &[u8]) -> Result<WearChange, WearChangeError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(WearChangeError::BadLength(bytes.len()));
    }
    let raw: SpawnUpdateStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const SpawnUpdateStruct) };
    Ok(WearChange {
        spawn_id:   unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        subcommand: unsafe { std::ptr::addr_of!(raw.subcommand).read_unaligned() },
        arg1:       unsafe { std::ptr::addr_of!(raw.arg1).read_unaligned() },
        arg2:       unsafe { std::ptr::addr_of!(raw.arg2).read_unaligned() },
        arg3:       unsafe { std::ptr::addr_of!(raw.arg3).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_wear_change(&[0; 31]).is_err());
        assert!(parse_wear_change(&[0; 33]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&123u16.to_le_bytes());
        buf[2..4].copy_from_slice(&17u16.to_le_bytes());
        buf[4..6].copy_from_slice(&7777i16.to_le_bytes());
        buf[6..8].copy_from_slice(&(-3i16).to_le_bytes());
        buf[8] = 0xab;
        let w = parse_wear_change(&buf).unwrap();
        assert_eq!(w.spawn_id, 123);
        assert_eq!(w.subcommand, 17);
        assert_eq!(w.arg1, 7777);
        assert_eq!(w.arg2, -3);
        assert_eq!(w.arg3, 0xab);
    }
}
