//! Parser for eql `OP_SpawnDoor` (0x71ca) — payload is `count * 132` bytes.
//!
//! EQ Legends door rows are 132 bytes, not Live's 136 (`sizeof(doorStruct)`):
//! the first 88 bytes are byte-identical to Live's layout — `name[32]`,
//! `y/x/z/heading` floats + `incline` u32 at 32..52, a 20-byte copy of those
//! five fields, `size` u32 at 72, `doorId/opentype/spawnstate/invertstate`
//! bytes at 80..84, `zonePoint` u32 at 84 — and the trailing unknown region is
//! 44 bytes instead of Live's 48. Derived 2026-07-13 from dump-payload rows of
//! two 660B (5×132) door arrays: lever/block object names at 0, sane coord
//! floats at 32..48, size 100, zonePoint 0xffffffff.
//!
//! The layout lives on `eqstructs::doorStruct`, which is eql-owned and
//! hand-maintained at 132 bytes, so `PAYLOAD_LEN` derives from `size_of`. The
//! C++ daemon iterates the array with the backend's `door_stride()` (this
//! crate's `PAYLOAD_LEN`) and calls `decode_door` per row.

use crate::eqstructs::doorStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<doorStruct>();

#[derive(Debug, Clone)]
pub struct Door {
    pub name: String,
    pub y: f32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub incline: u32,
    pub size: u32,
    pub door_id: u8,
    pub opentype: u8,
    pub spawnstate: u8,
    pub invertstate: u8,
    pub zone_point: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DoorError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_door(bytes: &[u8]) -> Result<Door, DoorError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(DoorError::BadLength(bytes.len()));
    }
    // `unknown0048` (a 20-byte copy of the five fields above — closed-state
    // pose?) and `unknown0056` are skipped, same as Live's parser.
    let raw: doorStruct = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const doorStruct) };
    Ok(Door {
        name: crate::cstr_field(&bytes[0..32]),
        y: unsafe { std::ptr::addr_of!(raw.y).read_unaligned() },
        x: unsafe { std::ptr::addr_of!(raw.x).read_unaligned() },
        z: unsafe { std::ptr::addr_of!(raw.z).read_unaligned() },
        heading: unsafe { std::ptr::addr_of!(raw.heading).read_unaligned() },
        incline: unsafe { std::ptr::addr_of!(raw.incline).read_unaligned() },
        size: unsafe { std::ptr::addr_of!(raw.size).read_unaligned() },
        door_id: raw.doorId,
        opentype: raw.opentype,
        spawnstate: raw.spawnstate,
        invertstate: raw.invertstate,
        zone_point: unsafe { std::ptr::addr_of!(raw.zonePoint).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_door(&[0; 131]).is_err());
        assert!(parse_door(&[0; 136]).is_err()); // a Live-sized row is NOT an eql row
    }

    // First row of a captured 660B (5×132) OP_SpawnDoor payload: a
    // "GIANTLEV" lever object.
    #[test]
    fn parses_capture_row() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[..8].copy_from_slice(b"GIANTLEV");
        buf[32..36].copy_from_slice(&[0x34, 0xad, 0x9a, 0xc4]); // y
        buf[36..40].copy_from_slice(&[0xe6, 0x56, 0x06, 0xc4]); // x
        buf[40..44].copy_from_slice(&[0x29, 0xed, 0x5f, 0x41]); // z
        buf[44..48].copy_from_slice(&[0x00, 0x00, 0x80, 0x43]); // heading 256.0
        buf[48..52].copy_from_slice(&64u32.to_le_bytes()); // incline
        buf[72..76].copy_from_slice(&100u32.to_le_bytes()); // size
        buf[80] = 0x05; // doorId
        buf[81] = 0x28; // opentype 40
        buf[84..88].copy_from_slice(&0xffff_ffffu32.to_le_bytes()); // zonePoint none
        let d = parse_door(&buf).unwrap();
        assert_eq!(d.name, "GIANTLEV");
        assert_eq!(d.y.to_le_bytes(), [0x34, 0xad, 0x9a, 0xc4]);
        assert_eq!(d.x.to_le_bytes(), [0xe6, 0x56, 0x06, 0xc4]);
        assert_eq!(d.z.to_le_bytes(), [0x29, 0xed, 0x5f, 0x41]);
        assert_eq!(d.heading, 256.0);
        assert_eq!(d.incline, 64);
        assert_eq!(d.size, 100);
        assert_eq!(d.door_id, 5);
        assert_eq!(d.opentype, 40);
        assert_eq!(d.spawnstate, 0);
        assert_eq!(d.invertstate, 0);
        assert_eq!(d.zone_point, 0xffff_ffff);
    }
}
