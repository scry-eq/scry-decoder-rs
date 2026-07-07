//! Parser for `OP_SpawnDoor` — payload is `count * 136` bytes (a
//! C array of `doorStruct`). The C++ daemon loops `nDoors = len /
//! sizeof(doorStruct)` and dispatches per-element to
//! `SpawnShell::newDoorSpawn`. This parser exposes a per-element
//! decode that mirrors that loop body — it yields one Door per call,
//! and the C++ side either iterates or invokes once per door.

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
    let raw: doorStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const doorStruct) };
    let name_raw: [u8; 32] = unsafe { std::ptr::addr_of!(raw.name).read_unaligned() };
    Ok(Door {
        name: crate::cstr_field(&name_raw),
        y:           unsafe { std::ptr::addr_of!(raw.y).read_unaligned() },
        x:           unsafe { std::ptr::addr_of!(raw.x).read_unaligned() },
        z:           unsafe { std::ptr::addr_of!(raw.z).read_unaligned() },
        heading:     unsafe { std::ptr::addr_of!(raw.heading).read_unaligned() },
        incline:     unsafe { std::ptr::addr_of!(raw.incline).read_unaligned() },
        size:        unsafe { std::ptr::addr_of!(raw.size).read_unaligned() },
        door_id:     unsafe { std::ptr::addr_of!(raw.doorId).read_unaligned() },
        opentype:    unsafe { std::ptr::addr_of!(raw.opentype).read_unaligned() },
        spawnstate:  unsafe { std::ptr::addr_of!(raw.spawnstate).read_unaligned() },
        invertstate: unsafe { std::ptr::addr_of!(raw.invertstate).read_unaligned() },
        zone_point:  unsafe { std::ptr::addr_of!(raw.zonePoint).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_door(&[0; 135]).is_err());
        assert!(parse_door(&[0; 137]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[..4].copy_from_slice(b"OAK1");
        buf[32..36].copy_from_slice(&1.5f32.to_le_bytes()); // y
        buf[36..40].copy_from_slice(&2.5f32.to_le_bytes()); // x
        buf[40..44].copy_from_slice(&3.5f32.to_le_bytes()); // z
        buf[44..48].copy_from_slice(&90.0f32.to_le_bytes()); // heading
        buf[48..52].copy_from_slice(&7u32.to_le_bytes()); // incline
        buf[72..76].copy_from_slice(&3u32.to_le_bytes()); // size
        buf[80] = 0x42; // doorId
        buf[81] = 1; // opentype
        buf[82] = 0; // spawnstate
        buf[83] = 1; // invertstate
        buf[84..88].copy_from_slice(&100u32.to_le_bytes()); // zonePoint
        let d = parse_door(&buf).unwrap();
        assert_eq!(d.name, "OAK1");
        assert_eq!(d.y, 1.5);
        assert_eq!(d.x, 2.5);
        assert_eq!(d.z, 3.5);
        assert_eq!(d.heading, 90.0);
        assert_eq!(d.incline, 7);
        assert_eq!(d.size, 3);
        assert_eq!(d.door_id, 0x42);
        assert_eq!(d.opentype, 1);
        assert_eq!(d.spawnstate, 0);
        assert_eq!(d.invertstate, 1);
        assert_eq!(d.zone_point, 100);
    }
}
