//! Parser for `OP_ZoneChange` — payload `zoneChangeStruct`, 100
//! bytes. Daemon reads zone_id; name and zone_instance are surfaced
//! for completeness.

use crate::eqstructs::zoneChangeStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<zoneChangeStruct>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneChange {
    pub name: String,
    pub zone_id: u16,
    pub zone_instance: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZoneChangeError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_zone_change(bytes: &[u8]) -> Result<ZoneChange, ZoneChangeError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ZoneChangeError::BadLength(bytes.len()));
    }
    let raw: zoneChangeStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const zoneChangeStruct) };
    let raw_name: [u8; 64] = unsafe { std::ptr::addr_of!(raw.name).read_unaligned() };
    Ok(ZoneChange {
        name: crate::cstr_field(&raw_name),
        zone_id: unsafe { std::ptr::addr_of!(raw.zoneId).read_unaligned() },
        zone_instance: unsafe { std::ptr::addr_of!(raw.zoneInstance).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_zone_change(&[0; 99]).is_err());
        assert!(parse_zone_change(&[0; 101]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..6].copy_from_slice(b"Bob\0\0\0");
        buf[64..66].copy_from_slice(&57u16.to_le_bytes());
        buf[66..68].copy_from_slice(&3u16.to_le_bytes());
        let z = parse_zone_change(&buf).unwrap();
        assert_eq!(z.name, "Bob");
        assert_eq!(z.zone_id, 57);
        assert_eq!(z.zone_instance, 3);
    }
}
