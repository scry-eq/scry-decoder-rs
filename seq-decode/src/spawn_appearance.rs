//! Parser for `OP_SpawnAppearance` — payload `spawnAppearanceStruct`,
//! 8 bytes: `{u32 spawnId, u32 type}`. `type` is the appearance subcommand.
//!
//! Layout re-derived 2026-07-28 (see everquest.h): the pre-patch layout put
//! `type` at offset 2 and a `parameter` value at 4, which reads type == 0 on
//! every current-patch packet. There is no value field now — a typed value
//! rides OP_SpawnAppearance2. Type semantics are not yet confirmed, so the
//! parser surfaces the raw subcommand and leaves meaning to the consumer.

use crate::eqstructs::spawnAppearanceStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnAppearanceStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnAppearance {
    pub spawn_id: u32,
    pub kind: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnAppearanceError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_spawn_appearance(bytes: &[u8]) -> Result<SpawnAppearance, SpawnAppearanceError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SpawnAppearanceError::BadLength(bytes.len()));
    }
    let raw: spawnAppearanceStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const spawnAppearanceStruct) };
    Ok(SpawnAppearance {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        kind: unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
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
        buf[0..4].copy_from_slice(&50u32.to_le_bytes());
        buf[4..8].copy_from_slice(&3u32.to_le_bytes());
        let a = parse_spawn_appearance(&buf).unwrap();
        assert_eq!(a.spawn_id, 50);
        assert_eq!(a.kind, 3);
    }

    // A real current-patch packet: spawn 7011, type 4 (live capture, 2026-07-28).
    #[test]
    fn parses_a_captured_packet() {
        let bytes = [0x63, 0x1b, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00];
        let a = parse_spawn_appearance(&bytes).unwrap();
        assert_eq!(a.spawn_id, 7011);
        assert_eq!(a.kind, 4);
    }
}
