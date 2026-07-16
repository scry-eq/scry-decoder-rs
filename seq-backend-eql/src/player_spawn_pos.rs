//! Parser for eql's 24-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — position broadcast for spawns other than the local player).
//!
//! **This is eql's OWN copy and diverges from Live's 24B struct** (clean-break:
//! when eql and Live differ, only this copy changes — Live's `parse_player_spawn_pos`
//! lives in `seq-decode`, untouched). The 2026-07-14 patch rotated the opcode id
//! (0x7171 -> 0x5188) AND shrank this broadcast 28B -> 24B; position re-cracked
//! against the post-patch capture (cross-validated vs the OP_MobUpdate stream +
//! self-trajectory smoothness). Evidence + confidence tiers in
//! `showeq-daemon/OPCODES_LEGENDS.md`.
//!
//! Like the prior 28B form, each coord sits in the **low 19 bits** of a word
//! (signed, ×8 fixed-point) — unlike Live, which packs them in the *high* bits.
//! Byte offsets (LSB-first, `#pragma pack(1)`):
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  spawnId2         (0 in every sample)
//!   /*0004*/ u32  header/velocity  (role TBD)
//!   /*0008*/ u32  z:19 @bit13 = (w>>13)&0x7FFFF (signed); low 13 bits = deltaZ-ish
//!   /*0012*/ u32  unknown12        (role TBD)
//!   /*0016*/ u32  y:19 (low, signed) + high 13 bits (velocity-ish)
//!   /*0020*/ u32  x:19 (low, signed) + high 13 bits
//! ```
//!
//! Each coord is a signed 19-bit ×8 fixed-point value; this parser surfaces the
//! *raw* sign-extended value and the daemon applies `>> 3` (1/8-unit -> integer
//! game world), matching the `EqlDispatch::mobUpdate` path. Only position is
//! confirmed (Y/Z high-confidence vs MobUpdate ground truth; X strong via the
//! self walk). Heading is not yet located in the 24B form (surfaced as 0 —
//! `SpawnShell::moveSpawn` takes no heading anyway); deltas/pitch/animation are 0.

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawnPos {
    pub spawn_id: u16,
    pub spawn_id2: u16,
    /// Raw 19-bit signed; daemon applies `>> 3` for fixed-point conv.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Not resolved on eql's 24B wire — surfaced as 0 (daemon uses position only).
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
    /// Not yet located in the 24B form — surfaced as 0 (moveSpawn takes no heading).
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

    // z rides bits 13..31 of the @8 word (low 13 bits = deltaZ-ish); y/x are in the
    // LOW 19 bits of the @16/@20 words (high 13 bits = velocity/delta, ignored).
    let z = sign_extend((read_u32_le(bytes, 8) >> 13) & 0x7_FFFF, 19);
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
        heading: 0,
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
        assert!(parse_player_spawn_pos(&[0; 28]).is_err()); // the pre-07/14 size is rejected
        assert!(parse_player_spawn_pos(&[0; 23]).is_err());
        assert!(parse_player_spawn_pos(&[0; 25]).is_err());
    }

    #[test]
    fn zero_payload_is_zero() {
        let p = parse_player_spawn_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!(p.spawn_id, 0);
        assert_eq!((p.x, p.y, p.z), (0, 0, 0));
        assert_eq!(p.heading, 0);
    }

    #[test]
    fn coords_low19_signed_x_y_at_16_20() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433
        // y = 42, x = -262144 (the 19-bit minimum); high bits set = velocity, ignored.
        buf[16..20].copy_from_slice(&(42u32 | (0x1FFFu32 << 19)).to_le_bytes());
        buf[20..24].copy_from_slice(&0x0004_0000u32.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.y, 42);
        assert_eq!(p.x, -262_144);
    }

    #[test]
    fn z_rides_bits_13_to_31_of_word8() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // z low-19 = -1 (0x7FFFF) at bit 13; low 13 bits (deltaZ) all set — ignored.
        buf[8..12].copy_from_slice(&((0x7_FFFFu32 << 13) | 0x1FFF).to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.z, -1);
    }
}
