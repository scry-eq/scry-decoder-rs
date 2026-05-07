//! Parser for `OP_DzSwitchInfo` — payload `dzSwitchInfo`, 32 bytes.
//! The C++ handler also accepts a stub 8-byte form ("we quit the
//! expedition"); this parser only handles the 32-byte struct path.
//! The daemon falls back to the C++ branch when bytes != 32.

use seq_eqstructs::dzSwitchInfo;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<dzSwitchInfo>();

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DzSwitch {
    pub zone_id: u16,
    pub instance_id: u16,
    pub kind: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DzSwitchError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_dz_switch_info(bytes: &[u8]) -> Result<DzSwitch, DzSwitchError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(DzSwitchError::BadLength(bytes.len()));
    }
    let raw: dzSwitchInfo =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const dzSwitchInfo) };
    Ok(DzSwitch {
        zone_id:     unsafe { std::ptr::addr_of!(raw.zoneID).read_unaligned() },
        instance_id: unsafe { std::ptr::addr_of!(raw.instanceID).read_unaligned() },
        kind:        unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
        x:           unsafe { std::ptr::addr_of!(raw.x).read_unaligned() },
        y:           unsafe { std::ptr::addr_of!(raw.y).read_unaligned() },
        z:           unsafe { std::ptr::addr_of!(raw.z).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_dz_switch_info(&[0; 8]).is_err());
        assert!(parse_dz_switch_info(&[0; 33]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[8..10].copy_from_slice(&42u16.to_le_bytes()); // zoneID
        buf[10..12].copy_from_slice(&2u16.to_le_bytes()); // instanceID
        buf[12..16].copy_from_slice(&5u32.to_le_bytes()); // type
        buf[20..24].copy_from_slice(&7.5f32.to_le_bytes()); // y
        buf[24..28].copy_from_slice(&1.25f32.to_le_bytes()); // x
        buf[28..32].copy_from_slice(&(-3.0f32).to_le_bytes()); // z
        let s = parse_dz_switch_info(&buf).unwrap();
        assert_eq!(s.zone_id, 42);
        assert_eq!(s.instance_id, 2);
        assert_eq!(s.kind, 5);
        assert_eq!(s.x, 1.25);
        assert_eq!(s.y, 7.5);
        assert_eq!(s.z, -3.0);
    }
}
