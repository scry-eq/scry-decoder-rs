//! Parser for eql's 24-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — position broadcast for spawns other than the local player).
//!
//! **This is eql's OWN copy**; when eql and Live differ, only this copy changes
//! (Live's `parse_player_spawn_pos` lives in `seq-decode`, untouched).
//!
//! **Re-laid-out 2026-08-18** from upstream legends `1cd04be`
//! (`playerPosUpdateEQLStruct`). The 08/18 patch rearranged the body again at
//! the same 24-byte size — the third rearrangement in three patches, and again
//! one no size gate can catch. Every coordinate now sits in the **low** 19 bits
//! of its word (signed, ×8 fixed-point); the high-19 z of the 08/04 layout is
//! gone:
//!
//! ```text
//!   /*0000*/ u16  spawnId
//!   /*0002*/ u16  spawnId2         (0 in every pre-patch sample)
//!   /*0004*/ u32  unknown          (role TBD — carried x before this patch)
//!   /*0008*/ u32  { z:19 (low, signed) | deltaZ:13 }
//!   /*0012*/ u32  unknown          (role TBD)
//!   /*0016*/ u32  { x:19 (low, signed) | heading @bit19 | pad:1 }
//!   /*0020*/ u32  { y:19 (low, signed) | deltaY:13 }
//! ```
//!
//! Only the heading kept its home: it is still the field at bit 19 of the @16
//! word. What moved under it is the coordinate sharing that word — `y` before
//! this patch, `x` now.
//!
//! **UNVALIDATED LOCALLY — no post-patch capture exists yet.** The 08/04 layout
//! was pinned by scoring all 173 candidate 19-bit windows against the
//! `OP_MobUpdate` / `OP_NpcMoveUpdate` streams; that scan has not been re-run
//! for 08/18 because there is no recording from this wire. This layout is
//! upstream's derivation taken as data. Re-run the scan on the first post-patch
//! capture before treating any axis here as confirmed — upstream and we have
//! disagreed on a word index before (see the ZoneEntry `posData` note in
//! `lib.rs`), and a transposed x/y decodes into a plausible-looking map.
//!
//! Previous layouts, kept so a re-derivation can tell drift from a bad read:
//! 08/04–08/05 was x @4 low-19 / z @12 high-19 / y @16 low-19, heading @16 bit19;
//! before that a 28-byte body with `spawnId@0, z@4, x@8, y@12`.
//!
//! This parser surfaces the *raw* sign-extended coords and the daemon applies
//! `>> 3` (1/8-unit -> integer game world), matching the `EqlDispatch::mobUpdate`
//! path. Deltas/pitch/animation have no located field and read 0.

use crate::eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 24;

/// Full circle in wire units for [`PlayerSpawnPos::heading`].
///
/// Upstream declares this field 12 bits wide but consumes it as `h2048` — 2048
/// units per circle, which is what an 11-bit field carries. We keep the 11-bit
/// read: it is what our own 08/04 measurement pinned (5.77-degree median
/// against travel bearing over 448 legs, next-best window 25.8) and it agrees
/// with upstream's *scale* rather than its declared width. Reading 12 bits at a
/// 2048 scale would double every angle. If a post-patch capture shows headings
/// wrapping at half a circle, widen to 12 and set this to 4096.
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
    /// Compass value (0..2047, see [`HEADING_UNITS`]); 0 = N, increasing
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

    // All three coords are the LOW 19 bits of their word as of 08/18 — z @8,
    // x @16, y @20 (see module doc).
    let z = sign_extend(read_u32_le(bytes, 8) & 0x7_FFFF, 19);
    let x = sign_extend(read_u32_le(bytes, 16) & 0x7_FFFF, 19);
    let y = sign_extend(read_u32_le(bytes, 20) & 0x7_FFFF, 19);

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

    // Each axis at its own offset, with the neighbouring bits set, so a word
    // that shifts under a future rearrangement fails loudly instead of reading
    // a plausible number out of the wrong field.
    #[test]
    fn each_coordinate_reads_from_its_own_word() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&0x1151u16.to_le_bytes()); // spawnId 4433

        // The two words that carry nothing: filled so a parser reading a
        // coordinate out of either fails instead of returning a plausible 0.
        buf[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        buf[12..16].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        // Each coordinate in the low 19 of its word, neighbouring bits all set.
        buf[8..12].copy_from_slice(&(42u32 | (0x1FFFu32 << 19)).to_le_bytes()); // z = 42
        buf[16..20].copy_from_slice(&(0x0004_0000u32 | (0x1FFFu32 << 19)).to_le_bytes()); // x = min
        buf[20..24].copy_from_slice(&(300u32 | (0x1FFFu32 << 19)).to_le_bytes()); // y = 300
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1151);
        assert_eq!(p.z, 42);
        assert_eq!(p.x, -262_144);
        assert_eq!(p.y, 300);
    }

    #[test]
    fn x_and_heading_share_the_word_at_16() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // x = -1 (all 19 bits set) and a quarter-circle heading directly above
        // it, with the spare top bit set so a sloppy mask would be caught.
        let quarter = u32::from(HEADING_UNITS) / 4;
        let w = 0x7_FFFFu32 | (quarter << 19) | (0x1u32 << 31);
        buf[16..20].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.x, -1);
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }
}
