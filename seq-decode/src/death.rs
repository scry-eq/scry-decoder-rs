//! Parser for `OP_Death` — payload `newCorpseStruct`, 40 bytes.
//! Fired when a spawn dies; the daemon uses spawn_id, killer_id,
//! type, and damage. The 12-byte trailing placeholder is consumed
//! for cursor accounting but not surfaced.

use crate::eqstructs::newCorpseStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<newCorpseStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    pub spawn_id: u32,
    pub killer_id: u32,
    pub corpse_id: u32,
    pub kind: i32,
    pub spell_id: u32,
    pub zone_id: u16,
    pub zone_instance: u16,
    pub damage: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeathError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_death(bytes: &[u8]) -> Result<Death, DeathError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(DeathError::BadLength(bytes.len()));
    }
    let raw: newCorpseStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const newCorpseStruct) };
    Ok(Death {
        spawn_id:      unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        killer_id:     unsafe { std::ptr::addr_of!(raw.killerId).read_unaligned() },
        corpse_id:     unsafe { std::ptr::addr_of!(raw.corpseid).read_unaligned() },
        kind:          unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
        spell_id:      unsafe { std::ptr::addr_of!(raw.spellId).read_unaligned() },
        zone_id:       unsafe { std::ptr::addr_of!(raw.zoneId).read_unaligned() },
        zone_instance: unsafe { std::ptr::addr_of!(raw.zoneInstance).read_unaligned() },
        damage:        unsafe { std::ptr::addr_of!(raw.damage).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_death(&[0; 39]).is_err());
        assert!(parse_death(&[0; 41]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 40];
        buf[0..4].copy_from_slice(&111u32.to_le_bytes());
        buf[4..8].copy_from_slice(&222u32.to_le_bytes());
        buf[8..12].copy_from_slice(&333u32.to_le_bytes());
        buf[12..16].copy_from_slice(&1i32.to_le_bytes());
        buf[16..20].copy_from_slice(&666u32.to_le_bytes());
        buf[20..22].copy_from_slice(&77u16.to_le_bytes());
        buf[22..24].copy_from_slice(&8u16.to_le_bytes());
        buf[24..28].copy_from_slice(&500u32.to_le_bytes());
        let d = parse_death(&buf).unwrap();
        assert_eq!(d.spawn_id, 111);
        assert_eq!(d.killer_id, 222);
        assert_eq!(d.corpse_id, 333);
        assert_eq!(d.kind, 1);
        assert_eq!(d.spell_id, 666);
        assert_eq!(d.zone_id, 77);
        assert_eq!(d.zone_instance, 8);
        assert_eq!(d.damage, 500);
    }
}
