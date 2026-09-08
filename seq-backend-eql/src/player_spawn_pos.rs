//! Parser for eql's OWN 28-byte `playerSpawnPosStruct` (`OP_ClientUpdate`, S>C
//! — position broadcast for spawns other than the local player); Live's copy
//! lives in `seq-decode` and is untouched by edits here.
//!
//! 09/01 layout from upstream 47a4992, unverified on our wire:
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  unknown          (0 on the wire)
//!   /*0004*/ u32  seq              per-entity counter, wraps at 32768
//!   /*0008*/ i64  animation:10@64 | y:19@74 | deltaZ:13@93 | deltaX:16@106 | pad:6
//!   /*0016*/ i64  deltaHeading:10@128 | x:19@138 | pad:3 | heading:11@160
//!                 | pad:1 | deltaY:13@172 | pad:7
//!   /*0024*/ i32  z:19@192 | pitch:11@211 | pad:2
//! ```
//!
//! Fields are already map-frame, so no transpose; coords stay raw and the
//! daemon applies `>> 3`, and the deltas are raw wire values (upstream's
//! `animation / 2` scaling is a display heuristic, not a decode).

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 28;

/// Full circle in wire units for [`PlayerSpawnPos::heading`] — 11 bits on a
/// 2048-step circle, matching both upstream and our own measurement.
pub const HEADING_UNITS: u16 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawnPos {
    pub spawn_id: u16,
    /// The u16 at offset 2; 0 on every packet upstream saw.
    pub spawn_id2: u16,
    /// Per-entity update counter, wrapping at 32768. Not in the FFI struct.
    pub seq: u32,
    /// Raw 19-bit signed; daemon applies `>> 3` for fixed-point conv.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Raw wire deltas — 16-bit for x, 13-bit for y and z. Scale unmeasured.
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
    /// Compass value (0..2047, see [`HEADING_UNITS`]); 0 = N, increasing
    /// clockwise, NOT inverted.
    pub heading: u16,
    pub delta_heading: i16,
    pub animation: i16,
    /// 11-bit up/down look angle on the same 2048-step circle.
    pub pitch: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlayerSpawnPosError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

fn read_u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u64_le(bytes: &[u8], at: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(b)
}

pub fn parse_player_spawn_pos(bytes: &[u8]) -> Result<PlayerSpawnPos, PlayerSpawnPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSpawnPosError::BadLength(bytes.len()));
    }

    // Bit positions are the module doc's, counted from bit 0 of the payload.
    let lo = read_u64_le(bytes, 8);
    let hi = read_u64_le(bytes, 16);
    let tail = read_u32_le(bytes, 24);
    let field = |w: u64, shift: u32, bits: u32| ((w >> shift) & ((1u64 << bits) - 1)) as u32;

    Ok(PlayerSpawnPos {
        spawn_id: read_u16_le(bytes, 0),
        spawn_id2: read_u16_le(bytes, 2),
        seq: read_u32_le(bytes, 4),
        x: sign_extend(field(hi, 10, 19), 19),
        y: sign_extend(field(lo, 10, 19), 19),
        z: sign_extend(tail & 0x7_FFFF, 19),
        delta_x: sign_extend(field(lo, 42, 16), 16),
        delta_y: sign_extend(field(hi, 44, 13), 13),
        delta_z: sign_extend(field(lo, 29, 13), 13),
        heading: field(hi, 32, 11) as u16,
        delta_heading: sign_extend(field(hi, 0, 10), 10) as i16,
        animation: sign_extend(field(lo, 0, 10), 10) as i16,
        pitch: ((tail >> 19) & 0x7FF) as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_player_spawn_pos(&[0; 24]).is_err());
        assert!(parse_player_spawn_pos(&[0; 27]).is_err());
        assert!(parse_player_spawn_pos(&[0; 29]).is_err());
    }

    #[test]
    fn zero_payload_is_zero() {
        let p = parse_player_spawn_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!(p.spawn_id, 0);
        assert_eq!((p.x, p.y, p.z), (0, 0, 0));
        assert_eq!(p.heading, 0);
    }

    /// Pack one field into a 64-bit word at its documented bit position.
    fn at(shift: u32, bits: u32, value: i64) -> u64 {
        ((value as u64) & ((1u64 << bits) - 1)) << shift
    }

    // Layout pin: distinct values per field, so a parser that slips a bit fails
    // on a value rather than on a plausible-looking number.
    #[test]
    fn every_field_reads_from_its_own_bit_range() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433
        buf[4..8].copy_from_slice(&30_000u32.to_le_bytes()); // seq
        let lo = at(0, 10, 37) | at(10, 19, -1065 * 8) | at(29, 13, -12) | at(42, 16, 4000);
        let hi = at(0, 10, -9) | at(10, 19, 709 * 8) | at(32, 11, 1512) | at(44, 13, -60);
        let tail = at(0, 19, -22 * 8) | at(19, 11, 300);
        buf[8..16].copy_from_slice(&lo.to_le_bytes());
        buf[16..24].copy_from_slice(&hi.to_le_bytes());
        buf[24..28].copy_from_slice(&(tail as u32).to_le_bytes());

        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.spawn_id2, 0);
        assert_eq!(p.seq, 30_000);
        // the parser surfaces raw 19-bit values; the daemon applies >> 3
        assert_eq!((p.x >> 3, p.y >> 3, p.z >> 3), (709, -1065, -22));
        assert_eq!((p.delta_x, p.delta_y, p.delta_z), (4000, -60, -12));
        assert_eq!(p.heading, 1512);
        assert_eq!(p.delta_heading, -9);
        assert_eq!(p.animation, 37);
        assert_eq!(p.pitch, 300);
    }

    // Heading is 11 bits at 160, so a 12-bit read borrows a pad or an x bit.
    #[test]
    fn heading_is_eleven_bits_at_bit_160() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let quarter = u64::from(HEADING_UNITS) / 4;
        let hi = !(0x7FFu64 << 32) | (quarter << 32);
        buf[16..24].copy_from_slice(&hi.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }

    // The 19-bit coordinates are signed; the minimum must not wrap to positive.
    #[test]
    fn coordinates_sign_extend_at_the_nineteen_bit_minimum() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[8..16].copy_from_slice(&at(10, 19, -262_144).to_le_bytes());
        assert_eq!(parse_player_spawn_pos(&buf).unwrap().y, -262_144);
    }
}
