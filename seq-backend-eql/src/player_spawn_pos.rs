//! Parser for eql's 28-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — position broadcast for spawns other than the local player).
//!
//! **This is eql's OWN copy and diverges from Live's 24B struct** (clean-break:
//! when eql and Live differ, only this copy changes — Live's `parse_player_spawn_pos`
//! lives in `seq-decode`, untouched). Position cracked 2026-07-10 from
//! `eqlegends-levelup.vpk`; evidence + confidence tiers in
//! `showeq-daemon/OPCODES_LEGENDS.md`.
//!
//! Unlike Live (which packs the coordinate in the *high* 19 bits of each word),
//! eql keeps each coord in the **low 19 bits** and adds an extra u32 vs Live's
//! 24B. Layout (LSB-first, `#pragma pack(1)`):
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  spawnId2        (0 in every sample)
//!   /*0004*/ u32  unknown04       (eql-only; role TBD — not in Live's 24B)
//!   /*0008*/ u32  heading @bit16 = (w>>16)&0x7FF  (11-bit, 0..2047; velocity in low bits)
//!   /*0012*/ u32  z:19 (low, signed)  + high 13 bits (≈0)
//!   /*0016*/ u32  y:19 (low, signed)  + high 13 bits (velocity-ish)
//!   /*0020*/ u32  x:19 (low, signed)  + high 13 bits
//!   /*0024*/ u32  packed velocity/heading tail (TBD)
//! ```
//!
//! Each coord is a signed 19-bit ×8 fixed-point value; this parser surfaces the
//! *raw* sign-extended value and the daemon applies `>> 3` (1/8-unit → integer
//! game world), matching the `EqlDispatch::mobUpdate` path. Only position is
//! HIGH-confidence; the deltas/pitch/animation in the high bits aren't pinned
//! and are surfaced as 0 (the daemon uses position only).

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawnPos {
    pub spawn_id: u16,
    pub spawn_id2: u16,
    /// Raw 19-bit signed; daemon applies `>> 3` for fixed-point conv.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Not resolved on eql's 28B wire — surfaced as 0 (daemon uses position only).
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
    /// 11-bit (0..2047); moderate confidence (R=0.65 vs movement direction).
    pub heading: u16,
    pub delta_heading: i16,
    pub animation: i16,
    pub pitch: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlayerSpawnPosError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

fn read_u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

pub fn parse_player_spawn_pos(
    bytes: &[u8],
) -> Result<PlayerSpawnPos, PlayerSpawnPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSpawnPosError::BadLength(bytes.len()));
    }

    let spawn_id = read_u16_le(bytes, 0);
    let spawn_id2 = read_u16_le(bytes, 2);

    // offset 8: heading occupies bits 16..27 (11-bit, 0..2047); low bits = velocity.
    let heading = ((read_u32_le(bytes, 8) >> 16) & 0x7FF) as u16;

    // offsets 12/16/20: coordinate in the LOW 19 bits (signed), delta in high bits.
    let z = sign_extend(read_u32_le(bytes, 12) & 0x7_FFFF, 19);
    let y = sign_extend(read_u32_le(bytes, 16) & 0x7_FFFF, 19);
    let x = sign_extend(read_u32_le(bytes, 20) & 0x7_FFFF, 19);

    Ok(PlayerSpawnPos {
        spawn_id,
        spawn_id2,
        x,
        y,
        z,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        heading,
        delta_heading: 0,
        animation: 0,
        pitch: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_player_spawn_pos(&[0; 24]).is_err()); // Live's size is rejected
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

    #[test]
    fn coords_are_low_19_bits_signed() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433
        // z = -1 (0x7_FFFF), y = 42, x = -262144 (0x4_0000, the 19-bit minimum).
        buf[12..16].copy_from_slice(&0x0007_FFFFu32.to_le_bytes());
        buf[16..20].copy_from_slice(&42u32.to_le_bytes());
        buf[20..24].copy_from_slice(&0x0004_0000u32.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.z, -1);
        assert_eq!(p.y, 42);
        assert_eq!(p.x, -262_144);
    }

    #[test]
    fn high_bits_of_coord_word_dont_leak() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // x low-19 = 100; high 13 bits (a delta) all set — must be ignored.
        buf[20..24].copy_from_slice(&(100u32 | (0x1FFFu32 << 19)).to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.x, 100);
    }

    #[test]
    fn heading_is_11_bit_at_bit16() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // heading = 0x7FF at bits 16..27 of the @8 word; low bits set = velocity, ignored.
        buf[8..12].copy_from_slice(&(0xFFFFu32 | (0x7FFu32 << 16)).to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.heading, 0x7FF);
    }
}
