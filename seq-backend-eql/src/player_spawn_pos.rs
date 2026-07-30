//! Parser for eql's 28-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — position broadcast for spawns other than the local player).
//!
//! **This is eql's OWN copy and diverges from Live's 24B struct** (clean-break:
//! when eql and Live differ, only this copy changes — Live's `parse_player_spawn_pos`
//! lives in `seq-decode`, untouched).
//!
//! **Re-derived 2026-07-29** (the 07/29 rotation, which rotated the whole opcode
//! table AND grew this broadcast 24B -> 28B with a fully rearranged body — none
//! of the 24B offsets survive). Each coord still sits in the **low 19 bits** of a
//! word (signed, ×8 fixed-point), but all three moved and each now starts on a
//! 4-byte boundary:
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  spawnId2         (0 in every sample)
//!   /*0004*/ u32  { z:19 (low, signed) | hi:13 }
//!   /*0008*/ u32  { x:19 (low, signed) | hi:13 — reads 0 in every sample }
//!   /*0012*/ u32  { y:19 (low, signed) | hi:13 }
//!   /*0016*/ u32  unknown          (role TBD)
//!   /*0020*/ u32  { lowfrac:8 | heading:13 @bit8 | hi:11 }
//!   /*0024*/ u32  unknown          (upper 13 bits read 0 in every sample)
//! ```
//!
//! How the coords were pinned (146 broadcasts): each 19-bit window was scored
//! against the `OP_MobUpdate` / `OP_NpcMoveUpdate` position streams, which the
//! 07/29 patch left untouched and which therefore stand as map-frame ground
//! truth. The three winners beat their runners-up by 50-1000× on median error
//! (z 0.00 vs 37.25, x 0.38 vs 54.38, y 0.50 vs 697.00). Two independent checks
//! agree: decoded z spans a 144-unit terrain band while a wrong window spans the
//! whole 19-bit field, and per-spawn tracks imply 0.9-1.6 units/sec median with
//! **0 of 51** steps above 100 u/s (a wrong window puts the median at 2031 u/s).
//!
//! Heading is the 13-bit field at bit 8 of the @20 word — the same
//! `{ lowfrac:8 | heading | hi }` sub-structure the C>S self-report uses. It is a
//! **compass** value (8192 per circle, 0 = N, increasing clockwise) and is NOT
//! inverted, unlike the `heading_deg` convention the mob/npc streams use.
//! Measured against travel bearing over 26 player legs at a 3.8 degree median;
//! every other candidate window scored 27 degrees or worse. The sample is small
//! because this capture holds little movement, so treat the SENSE as measured but
//! lightly evidenced — the field location is not in doubt.
//!
//! This parser surfaces the *raw* sign-extended coords and the daemon applies
//! `>> 3` (1/8-unit -> integer game world), matching the `EqlDispatch::mobUpdate`
//! path. Deltas/pitch/animation have no located field and read 0.

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 28;

/// Full circle in wire units for [`PlayerSpawnPos::heading`] (13-bit field).
///
/// Note this differs from the C>S self-report's 11-bit
/// [`crate::player_self_pos::HEADING_UNITS`] — same packet family, different
/// field widths. Both are compass values needing no inversion.
pub const HEADING_UNITS: u16 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawnPos {
    pub spawn_id: u16,
    pub spawn_id2: u16,
    /// Raw 19-bit signed; daemon applies `>> 3` for fixed-point conv.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// No located field on eql's 28B wire — surfaced as 0.
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
    /// 13-bit compass value (0..8191, see [`HEADING_UNITS`]); 0 = N, increasing
    /// clockwise, NOT inverted. `SpawnShell::moveSpawn` takes no heading, so the
    /// daemon currently ignores this; it is decoded so callers that want a
    /// facing don't have to re-derive it.
    pub heading: u16,
    /// Not carried on eql's 28B wire — surfaced as 0.
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

    // z/x/y in the LOW 19 bits of the @4/@8/@12 words (high 13 bits carry
    // something velocity-shaped on z and y, and read 0 on x — see module doc).
    let z = sign_extend(read_u32_le(bytes, 4) & 0x7_FFFF, 19);
    let x = sign_extend(read_u32_le(bytes, 8) & 0x7_FFFF, 19);
    let y = sign_extend(read_u32_le(bytes, 12) & 0x7_FFFF, 19);

    let heading = ((read_u32_le(bytes, 20) >> 8) & 0x1FFF) as u16;

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
        assert!(parse_player_spawn_pos(&[0; 24]).is_err()); // the pre-07/29 size is rejected
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
    fn coords_are_the_low19_of_the_4_8_12_words() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433
        // z = 42 with every high bit set (velocity-ish field, must be ignored);
        // x = the 19-bit minimum; y = -1.
        buf[4..8].copy_from_slice(&(42u32 | (0x1FFFu32 << 19)).to_le_bytes());
        buf[8..12].copy_from_slice(&0x0004_0000u32.to_le_bytes());
        buf[12..16].copy_from_slice(&0x0007_FFFFu32.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.z, 42);
        assert_eq!(p.x, -262_144);
        assert_eq!(p.y, -1);
    }

    #[test]
    fn heading_is_13_bits_at_bit_8_of_the_word_at_20() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // A quarter circle, with both neighbouring sub-fields fully set so a
        // sloppy mask would be caught.
        let w = (u32::from(HEADING_UNITS) / 4) << 8 | 0xFF | (0x7FFu32 << 21);
        buf[20..24].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }

    // A real broadcast off the 07/29 wire: one sample of a walking player, taken
    // from the middle of a 32-sample track that traces a smooth uphill walk. The
    // expected position is cross-checked against the OP_MobUpdate /
    // OP_NpcMoveUpdate streams for the same window (see module doc).
    #[test]
    fn decodes_a_captured_broadcast() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0x7B, 0x3C, 0x00, 0x00, 0xBF, 0xFE, 0x07, 0x04, 0x9A, 0x09, 0x00, 0x00, 0xE9, 0x31,
            0x80, 0x02, 0x00, 0xC0, 0x0B, 0x00, 0x25, 0x88, 0x03, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];
        let p = parse_player_spawn_pos(&bytes).unwrap();
        assert_eq!(p.spawn_id, 15483);
        assert_eq!(p.spawn_id2, 0);
        // raw 1/8-unit values, and the >> 3 the daemon applies
        assert_eq!((p.x, p.y, p.z), (2458, 12777, -321));
        assert_eq!((p.x >> 3, p.y >> 3, p.z >> 3), (307, 1597, -41));
        assert_eq!(p.heading, 904); // ~39.7 degrees, i.e. NNE
    }
}
