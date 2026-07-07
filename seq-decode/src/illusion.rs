//! Parser for `OP_Illusion` — payload `spawnIllusionStruct`,
//! 332 bytes. Daemon uses spawn_id, race, gender, texture, helm,
//! face. The 248-byte trailing placeholder is consumed for the
//! length match but not surfaced.

use crate::eqstructs::spawnIllusionStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnIllusionStruct>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Illusion {
    pub spawn_id: u32,
    pub name: String,
    pub race: u32,
    pub gender: u8,
    pub texture: u8,
    pub helm: u8,
    pub face: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IllusionError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_illusion(bytes: &[u8]) -> Result<Illusion, IllusionError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(IllusionError::BadLength(bytes.len()));
    }
    let raw: spawnIllusionStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const spawnIllusionStruct) };
    let raw_name = unsafe { std::ptr::addr_of!(raw.name).read_unaligned() };
    let mut name_bytes = [0u8; 64];
    for i in 0..64 {
        name_bytes[i] = raw_name[i] as u8;
    }
    Ok(Illusion {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        name:     crate::cstr_field(&name_bytes),
        race:    unsafe { std::ptr::addr_of!(raw.race).read_unaligned() },
        gender:  unsafe { std::ptr::addr_of!(raw.gender).read_unaligned() },
        texture: unsafe { std::ptr::addr_of!(raw.texture).read_unaligned() },
        helm:    unsafe { std::ptr::addr_of!(raw.helm).read_unaligned() },
        face:    unsafe { std::ptr::addr_of!(raw.face).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_illusion(&[0; 331]).is_err());
        assert!(parse_illusion(&[0; 333]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = vec![0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&100u32.to_le_bytes());
        buf[4..10].copy_from_slice(b"Goblin");
        buf[68..72].copy_from_slice(&75u32.to_le_bytes()); // race
        buf[72] = 1; // gender female
        buf[73] = 5; // texture
        buf[74] = 2; // helm
        buf[80..84].copy_from_slice(&42u32.to_le_bytes()); // face
        let i = parse_illusion(&buf).unwrap();
        assert_eq!(i.spawn_id, 100);
        assert_eq!(i.name, "Goblin");
        assert_eq!(i.race, 75);
        assert_eq!(i.gender, 1);
        assert_eq!(i.texture, 5);
        assert_eq!(i.helm, 2);
        assert_eq!(i.face, 42);
    }
}
