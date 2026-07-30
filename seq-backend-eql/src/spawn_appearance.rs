//! Parser for eql's 24-byte `OP_SpawnAppearance` payload.
//!
//! **This is eql's OWN copy and diverges from Live's 8B struct.** eql widens
//! every field to `u32` and keeps the value on this opcode, where Live moved it
//! out to a second opcode (`OP_SpawnAppearance2`) and left an 8-byte
//! `{u32 spawnId, u32 type}` behind. eql has only the ONE appearance opcode and
//! it already carries the wide record, so Live's two-opcode split does not map
//! onto it at all:
//!
//! ```text
//!   /*0000*/ u32 spawnId
//!   /*0004*/ u32 type       eql's own numbering — 6 = pose, and see below
//!   /*0008*/ u32 value
//!   /*0012*/ u32 params[3]  zero in 134 of 157 captured packets
//!   /*0024*/
//! ```
//!
//! Until 2026-07-30 this module read the pinned Live binding instead — the old
//! `{u16 spawnId, u16 type, u32 parameter}` shape — which is a silent
//! mis-decode, not a loud one: `type` is a `u32` whose high half is zero, so the
//! `u16` at offset 2 reads that zero half and every later field shifts one to
//! the right. Over 157 captured packets the legacy read reports **type 0 in all
//! 157** with the real type values landing in `parameter`, while the spawn id
//! decodes identically either way — which is why it hid. It never actually ran
//! on eql because the size gate rejected all 24 bytes against a Live `sizeof` of
//! 8; both halves of that bug are fixed together, since correcting only the gate
//! would just hand 24 bytes to a parser that wants 8.
//!
//! Layout independently confirmed against upstream's legends branch, whose
//! `spawnEventEQLStruct` is field-for-field identical.
//!
//! Type numbering is eql's own and only type 6 is confirmed (pose: 110 sit /
//! 100 stand / 111 duck), wire-verified against scripted toggles and
//! corroborated by upstream. Upstream additionally labels 13 = anon, 22 =
//! periodic tick, 36 = LFG, 41 = timestamp. A 26-minute capture also saw types
//! 43, 11, 26, 3, 8, 5 and 1 with no confirmed meaning. This parser surfaces the
//! raw triple and leaves interpretation to the caller.

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

pub fn parse_spawn_appearance(
    bytes: &[u8],
) -> Result<SpawnAppearance, SpawnAppearanceError> {
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
        // 8 is Live's size, and was this parser's own size until 2026-07-30 —
        // pinned so a regression to the narrow layout fails loudly.
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

    /// The regression this module sat in for two patches: a spawn id above
    /// 65535 is impossible under the narrow read, and a type whose high half is
    /// zero decodes as type 0 there. Both must come out right now.
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
