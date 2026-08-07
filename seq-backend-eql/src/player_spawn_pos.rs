//! Parser for eql's 24-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — position broadcast for spawns other than the local player).
//!
//! **This is eql's OWN copy**; when eql and Live differ, only this copy changes
//! (Live's `parse_player_spawn_pos` lives in `seq-decode`, untouched).
//!
//! **Re-derived 2026-08-04** (the 08/04 rotation, which shrank this broadcast
//! 28B -> 24B and rearranged the body again — no 28B offset survives). Two of
//! the three coords still sit in the **low 19 bits** of a word (signed, ×8
//! fixed-point), but z now sits in the **high** 19 bits of its word:
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  spawnId2         (0 in every sample)
//!   /*0004*/ u32  { x:19 (low, signed) | hi:13 }
//!   /*0008*/ u32  unknown          (role TBD)
//!   /*0012*/ u32  { lo:13 | z:19 (high, signed) }
//!   /*0016*/ u32  { y:19 (low, signed) | heading:11 @bit19 | hi:2 }
//!   /*0020*/ u32  unknown          (role TBD)
//! ```
//!
//! How the coords were pinned (993 broadcasts, 41 tracked spawns): an
//! exhaustive scan of **all 173 candidate 19-bit windows** in the body scored
//! each against the `OP_MobUpdate` / `OP_NpcMoveUpdate` position streams, which
//! this patch left untouched and which therefore stand as map-frame ground
//! truth. The scan independently selected these three as the global best for
//! their axis, matching upstream's independent derivation.
//!
//! Absolute error against ground truth is not the right statistic here — this
//! opcode carries *other PCs* while `OP_MobUpdate` carries NPCs, so only 57 of
//! 993 records overlap a ground-truth track at all. The non-confounded checks:
//! per-spawn trajectory smoothness over 931 consecutive steps gives a 4.00-unit
//! median (p90 21.6) with 4 steps above 500 units, i.e. a real walk; and an
//! x/y-transposed control scores 6× worse against ground truth (667-unit median
//! vs 111, 1/57 within 25 units vs 18/57), which settles the axis orientation.
//!
//! Heading is an **11-bit** field at bit 19 of the @16 word — directly above
//! `y`, in the bits upstream's struct labels `deltaY`. It is a **compass** value
//! (2048 per circle, 0 = N, increasing clockwise) and is NOT inverted, unlike
//! the `heading_deg` convention the mob/npc streams use. Measured against travel
//! bearing over 448 legs at a 5.77-degree median; the next-best window scored
//! 25.8 degrees and a random field would score ~90.
//!
//! This parser surfaces the *raw* sign-extended coords and the daemon applies
//! `>> 3` (1/8-unit -> integer game world), matching the `EqlDispatch::mobUpdate`
//! path. Deltas/pitch/animation have no located field and read 0.

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 24;

/// Full circle in wire units for [`PlayerSpawnPos::heading`] (11-bit field).
///
/// The 08/04 rotation narrowed this from 13 bits to 11, bringing it in line
/// with the C>S self-report's [`crate::player_self_pos::HEADING_UNITS`]. Both
/// are compass values needing no inversion.
pub const HEADING_UNITS: u16 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawnPos {
    pub spawn_id: u16,
    pub spawn_id2: u16,
    /// Raw 19-bit signed; daemon applies `>> 3` for fixed-point conv.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// No located field on eql's 24B wire — surfaced as 0.
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
    /// 11-bit compass value (0..2047, see [`HEADING_UNITS`]); 0 = N, increasing
    /// clockwise, NOT inverted. `SpawnShell::moveSpawn` takes no heading, so the
    /// daemon currently ignores this; it is decoded so callers that want a
    /// facing don't have to re-derive it.
    pub heading: u16,
    /// Not carried on eql's 24B wire — surfaced as 0.
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

pub fn parse_player_spawn_pos(bytes: &[u8]) -> Result<PlayerSpawnPos, PlayerSpawnPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSpawnPosError::BadLength(bytes.len()));
    }

    let spawn_id = read_u16_le(bytes, 0);
    let spawn_id2 = read_u16_le(bytes, 2);

    // x and y in the LOW 19 bits of the @4 and @16 words; z in the HIGH 19 of
    // the @12 word (see module doc for how each was pinned).
    let x = sign_extend(read_u32_le(bytes, 4) & 0x7_FFFF, 19);
    let z = sign_extend(read_u32_le(bytes, 12) >> 13, 19);
    let y = sign_extend(read_u32_le(bytes, 16) & 0x7_FFFF, 19);

    let heading = ((read_u32_le(bytes, 16) >> 19) & 0x7FF) as u16;

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
        assert!(parse_player_spawn_pos(&[0; 28]).is_err()); // the pre-08/04 size is rejected
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
    fn x_is_the_low19_at_4_and_z_is_the_high19_at_12() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433
                                                             // x = 42 with every high bit set (the neighbouring field must be ignored).
        buf[4..8].copy_from_slice(&(42u32 | (0x1FFFu32 << 19)).to_le_bytes());
        // z = the 19-bit minimum, parked in the HIGH bits, low 13 bits all set.
        buf[12..16].copy_from_slice(&((0x0004_0000u32 << 13) | 0x1FFF).to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.x, 42);
        assert_eq!(p.z, -262_144);
    }

    #[test]
    fn y_and_heading_share_the_word_at_16() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // y = -1 (all 19 bits set) and a quarter-circle heading directly above
        // it, with the 2 spare top bits set so a sloppy mask would be caught.
        let quarter = u32::from(HEADING_UNITS) / 4;
        let w = 0x7_FFFFu32 | (quarter << 19) | (0x3u32 << 30);
        buf[16..20].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.y, -1);
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }
}
