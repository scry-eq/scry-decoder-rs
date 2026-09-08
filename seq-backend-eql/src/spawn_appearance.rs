//! Parser for eql's OWN 24-byte `OP_SpawnAppearance` payload, which diverges
//! from Live's 8B struct: eql widens every field to `u32` and has only the ONE
//! appearance opcode, so Live's two-opcode split does not map onto it.
//!
//! ```text
//!   /*0000*/ u32 spawnId
//!   /*0004*/ u32 type       eql's own numbering — 6 = pose
//!   /*0008*/ u32 value
//!   /*0012*/ u32 params[3]  zero on nearly every packet
//!   /*0024*/
//! ```
//!
//! Type 6 is pose (100 stand / 110 sit / 111 duck), wire-verified here; upstream
//! additionally labels 1 level, 3 invis, 5 light, 15 sneak, 22 guild id, 41
//! timestamp. This parser surfaces the raw triple, interpretation is the caller's.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnAppearance {
    pub spawn_id: u32,
    /// eql's own appearance-type numbering (NOT Live's).
    pub kind: u32,
    /// Type-specific value. Live's current wire has no such field; eql's does.
    pub parameter: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnAppearanceError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

fn read_u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

pub fn parse_spawn_appearance(bytes: &[u8]) -> Result<SpawnAppearance, SpawnAppearanceError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SpawnAppearanceError::BadLength(bytes.len()));
    }
    Ok(SpawnAppearance {
        spawn_id: read_u32_le(bytes, 0),
        kind: read_u32_le(bytes, 4),
        parameter: read_u32_le(bytes, 8),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        // 8 is Live's size; pinned so a regression to it fails loudly.
        assert!(parse_spawn_appearance(&[0; 8]).is_err());
        assert!(parse_spawn_appearance(&[0; 23]).is_err());
        assert!(parse_spawn_appearance(&[0; 25]).is_err());
    }

    #[test]
    fn parses_the_wide_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&15483u32.to_le_bytes());
        buf[4..8].copy_from_slice(&6u32.to_le_bytes()); // pose
        buf[8..12].copy_from_slice(&110u32.to_le_bytes()); // sitting
        let a = parse_spawn_appearance(&buf).unwrap();
        assert_eq!(a.spawn_id, 15483);
        assert_eq!(a.kind, 6);
        assert_eq!(a.parameter, 110);
    }

    /// Under the narrow Live read a spawn id above 65535 truncates and a type
    /// with a zero high half decodes as 0; both must come out right here.
    #[test]
    fn does_not_read_the_narrow_legacy_layout() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&70000u32.to_le_bytes());
        buf[4..8].copy_from_slice(&22u32.to_le_bytes());
        buf[8..12].copy_from_slice(&7u32.to_le_bytes());
        let a = parse_spawn_appearance(&buf).unwrap();
        assert_eq!(a.spawn_id, 70000, "id must not truncate to u16");
        assert_eq!(a.kind, 22, "type must not read the zero high half");
        assert_eq!(a.parameter, 7);
    }

    #[test]
    fn trailing_params_are_ignored() {
        let mut buf = [0xAAu8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&1u32.to_le_bytes());
        buf[4..8].copy_from_slice(&6u32.to_le_bytes());
        buf[8..12].copy_from_slice(&100u32.to_le_bytes());
        let a = parse_spawn_appearance(&buf).unwrap();
        assert_eq!((a.spawn_id, a.kind, a.parameter), (1, 6, 100));
    }
}
