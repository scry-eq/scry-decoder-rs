//! Parser for `OP_CorpseLocResponse` — payload `corpseLocStruct`,
//! 16 bytes. spawn_id + (x, y, z) floats.

use seq_eqstructs_live::corpseLocStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<corpseLocStruct>();

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorpseLoc {
    pub spawn_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CorpseLocError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_corpse_loc(bytes: &[u8]) -> Result<CorpseLoc, CorpseLocError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(CorpseLocError::BadLength(bytes.len()));
    }
    let raw: corpseLocStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const corpseLocStruct) };
    Ok(CorpseLoc {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        x:        unsafe { std::ptr::addr_of!(raw.x).read_unaligned() },
        y:        unsafe { std::ptr::addr_of!(raw.y).read_unaligned() },
        z:        unsafe { std::ptr::addr_of!(raw.z).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_corpse_loc(&[0; 15]).is_err());
        assert!(parse_corpse_loc(&[0; 17]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&77u32.to_le_bytes());
        buf[4..8].copy_from_slice(&1.5f32.to_le_bytes());
        buf[8..12].copy_from_slice(&(-2.5f32).to_le_bytes());
        buf[12..16].copy_from_slice(&33.0f32.to_le_bytes());
        let c = parse_corpse_loc(&buf).unwrap();
        assert_eq!(c.spawn_id, 77);
        assert_eq!(c.x, 1.5);
        assert_eq!(c.y, -2.5);
        assert_eq!(c.z, 33.0);
    }
}
