//! Parser for one `zonePointStruct` element of an `OP_SendZonePoints`
//! payload. The wire layout is a 4-byte count, then `count` 24-byte
//! `zonePointStruct` records, then a 24-byte trailing block. Like the
//! per-door pattern for `OP_SpawnDoor`, this parser handles a single
//! element; the daemon slices the payload at `data + 4 + i * 24` and
//! invokes once per record.

use crate::eqstructs::zonePointStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<zonePointStruct>();

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZonePoint {
    pub zone_trigger: u32,
    pub y: f32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub zone_id: u16,
    pub zone_instance: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZonePointError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_zone_point(bytes: &[u8]) -> Result<ZonePoint, ZonePointError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ZonePointError::BadLength(bytes.len()));
    }
    let raw: zonePointStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const zonePointStruct) };
    Ok(ZonePoint {
        zone_trigger: unsafe { std::ptr::addr_of!(raw.zoneTrigger).read_unaligned() },
        y: unsafe { std::ptr::addr_of!(raw.y).read_unaligned() },
        x: unsafe { std::ptr::addr_of!(raw.x).read_unaligned() },
        z: unsafe { std::ptr::addr_of!(raw.z).read_unaligned() },
        heading: unsafe { std::ptr::addr_of!(raw.heading).read_unaligned() },
        zone_id: unsafe { std::ptr::addr_of!(raw.zoneId).read_unaligned() },
        zone_instance: unsafe { std::ptr::addr_of!(raw.zoneInstance).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_zone_point(&[0; 23]).is_err());
        assert!(parse_zone_point(&[0; 25]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&7u32.to_le_bytes()); // zoneTrigger
        buf[4..8].copy_from_slice(&1.5f32.to_le_bytes()); // y
        buf[8..12].copy_from_slice(&2.5f32.to_le_bytes()); // x
        buf[12..16].copy_from_slice(&3.5f32.to_le_bytes()); // z
        buf[16..20].copy_from_slice(&90.0f32.to_le_bytes()); // heading
        buf[20..22].copy_from_slice(&57u16.to_le_bytes()); // zoneId
        buf[22..24].copy_from_slice(&3u16.to_le_bytes()); // zoneInstance
        let p = parse_zone_point(&buf).unwrap();
        assert_eq!(p.zone_trigger, 7);
        assert_eq!(p.y, 1.5);
        assert_eq!(p.x, 2.5);
        assert_eq!(p.z, 3.5);
        assert_eq!(p.heading, 90.0);
        assert_eq!(p.zone_id, 57);
        assert_eq!(p.zone_instance, 3);
    }
}
