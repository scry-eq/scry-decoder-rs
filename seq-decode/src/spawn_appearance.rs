//! Parser for `OP_SpawnAppearance` — payload `spawnAppearanceStruct`,
//! 8 bytes. `type` is the appearance subcommand (anim, light, AFK,
//! etc.); `parameter` is the value.

use seq_eqstructs::spawnAppearanceStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnAppearanceStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnAppearance {
    pub spawn_id: u16,
    pub kind: u16,
    pub parameter: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnAppearanceError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_spawn_appearance(
    bytes: &[u8],
) -> Result<SpawnAppearance, SpawnAppearanceError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SpawnAppearanceError::BadLength(bytes.len()));
    }
    let raw: spawnAppearanceStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const spawnAppearanceStruct) };
    Ok(SpawnAppearance {
        spawn_id:  unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        kind:      unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
        parameter: unsafe { std::ptr::addr_of!(raw.parameter).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_spawn_appearance(&[0; 7]).is_err());
        assert!(parse_spawn_appearance(&[0; 9]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&50u16.to_le_bytes());
        buf[2..4].copy_from_slice(&3u16.to_le_bytes());
        buf[4..8].copy_from_slice(&0xCAFEBABEu32.to_le_bytes());
        let a = parse_spawn_appearance(&buf).unwrap();
        assert_eq!(a.spawn_id, 50);
        assert_eq!(a.kind, 3);
        assert_eq!(a.parameter, 0xCAFEBABE);
    }
}
