//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
//!
//! **Re-derived 2026-08-04.** The 08/04 rotation rearranged the body again; the
//! size stayed 42B, so no size gate could catch it. None of the 07/29 offsets
//! survive. Positions are IEEE floats in game-world units (no ×8 packing —
//! this is C>S, distinct from the S>C packed `playerSpawnPosStruct`):
//!
//! ```text
//!   /*0000*/ u16  ctr        update counter
//!   /*0002*/ u16  spawnId    the local player's spawn id (the PHANTOM TWIN's)
//!   /*0004*/ u8   unknown0004[14]
//!   /*0018*/ f32  y          gameY
//!   /*0022*/ u32  { heading:11 (low) | hi:21 }
//!   /*0026*/ u32  unknown
//!   /*0030*/ f32  z          gameZ
//!   /*0034*/ f32  unknown    velocity candidate
//!   /*0038*/ f32  x          gameX
//! ```
//!
//! Upstream declares this struct 44B (`tail[2]` past the `x` float at 38). The
//! wire is **42B** — 703 C>S bodies in the 08/04 capture, none at 44 — so the
//! tail is dropped here and `PAYLOAD_LEN` stays 42. Both payloads are gated
//! `none`, so an over-long declaration would not warn; it would just hand this
//! parser a short buffer.
//!
//! **How the axes were pinned.** The three position floats fall out of a range
//! comparison against the `OP_SelfPos` breadcrumb (which reports the player's
//! real path, and which was independently re-confirmed for 08/04). Over 703
//! self-reports the field ranges match the breadcrumb's per-axis ranges
//! essentially exactly — @18 [-2627.32, 1086.62] vs the breadcrumb's
//! [-2627.32, 1086.62], @38 [-428.68, 1041.24] vs [-429.72, 1041.24], @30
//! [-905.81, 51.44] vs [-905.81, 58.79]. Every other float offset in the packet
//! spans at most ±5 (the velocities) or is pinned near 0. The 07/29 offsets
//! (@10/@22/@34) now read zero or ±4, which is the tell that they moved.
//!
//! Which of @18/@38 is X and which is Y is NOT taken from the breadcrumb's own
//! labels — those are in `/loc` order and transpose against the map frame, which
//! is exactly the trap that produced a silently-swapped read in an earlier patch.
//! It is settled by matching each field's observed RANGE to the corresponding
//! breadcrumb axis range (above): the three ranges are distinct enough that the
//! assignment is unambiguous, and a transposed reading would put @18's
//! 3714-unit span against an axis spanning 1470.
//!
//! **A spawnId is back at offset 2** (the 07/14 patch had dropped it; this
//! matches upstream's `eqlClientSelfPosStruct`, which kept declaring it). Over a
//! 161-report capture it takes exactly 3 values, switching at precisely the two
//! zone transitions, and none ever appears in the S>C broadcast stream — which is
//! how a self-id behaves, since the server never broadcasts your own position
//! back to you.
//!
//! **But it is the PHANTOM TWIN's id, not the live copy's — do not adopt from
//! it.** eql announces the local player twice per zone (a live copy that moves
//! and a static phantom the client hides, the phantom's id a few higher). On the
//! same capture this field read 15707 / 15719 while zoneEntry name-match adopted
//! 15701 / 15715, and dumping OP_ZoneEntry shows each pair sharing one name.
//! That is consistent with the twin being what eql keys self *stats* to. Pinning
//! the player to this id would attach it to the hidden phantom and leave the live
//! copy loose in the spawn list, so the daemon surfaces the field but keeps
//! zoneEntry name-match as the only adoption source.
//!
//! Velocities are deliberately NOT decoded — see the parser body.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 42;

/// Full circle in wire heading units (11-bit field → 2048 steps).
///
/// The facing is the low 11 bits of the dword at offset 22 (moved from 26 by
/// the 08/04 rotation, and agreeing with upstream's re-derivation). Scored
/// against travel bearing over 453 movement legs it lands at a **2.14 degree**
/// median; the next-best window in the whole 42B body scores 32.6 and a random
/// field would score ~90, so the location is not in doubt.
///
/// INVERTED like every other heading: `heading_deg(field, 11)`. Read
/// uninverted it mirrors — a left turn rotates the marker right. Only the
/// OFFSET moved this patch; width, scale and sense are unchanged, so the
/// downstream inversion carried over untouched, and that was **confirmed
/// in-game on 2026-08-05**: the reticle tracks the turn instead of mirroring
/// it.
///
/// Calibrate the sense on a TURN, never on facing-vs-travel-bearing: the
/// bearing shares the frame, so it cannot see a mirror. That is why the
/// 2.14-degree fit above pins the field's LOCATION but says nothing about its
/// sense.
pub const HEADING_UNITS: u16 = 2048;

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSelfPos {
    /// The local player's spawn id, back on the wire at offset 2 as of 07/29.
    pub spawn_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Not decoded on the 42B wire (see the parser body) — 0.
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    /// 11-bit unsigned compass value (0..2047, see [`HEADING_UNITS`]).
    pub heading: u16,
    /// Not carried on eql's C>S wire (unused by the daemon) — surfaced as 0.
    pub delta_heading: i16,
    /// Not carried on eql's C>S wire (unused by the daemon) — surfaced as 0.
    pub animation: i16,
    /// Not carried on eql's C>S wire (unused by the daemon) — surfaced as 0.
    pub pitch: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlayerSelfPosError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

fn read_u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_f32_le(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(read_u32_le(bytes, at))
}

pub fn parse_player_self_pos(bytes: &[u8]) -> Result<PlayerSelfPos, PlayerSelfPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSelfPosError::BadLength(bytes.len()));
    }

    let spawn_id = read_u16_le(bytes, 2);

    // Axis labels are the map frame's — MobUpdate's / the spawn record's — NOT
    // the breadcrumb's /loc ordering. See the module doc for how the X/Y
    // assignment was settled physically rather than from field labels.
    let y = read_f32_le(bytes, 18);
    let z = read_f32_le(bytes, 30);
    let x = read_f32_le(bytes, 38);

    // The velocity components have NOT been located for this patch and are
    // deliberately surfaced as 0 rather than read from a plausible-looking
    // offset: a wrong velocity would smear the player marker between updates.
    // Candidates are the three small-range floats at 14, 18 and 30 (all within
    // ±2.1, the right magnitude for the ±2.26 units/tick of a full run); the
    // capture has too little sustained movement to tell which maps to which
    // axis.
    let heading = (read_u32_le(bytes, 22) & 0x7FF) as u16;

    Ok(PlayerSelfPos {
        spawn_id,
        x,
        y,
        z,
        delta_x: 0.0,
        delta_y: 0.0,
        delta_z: 0.0,
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
        assert!(parse_player_self_pos(&[0; 38]).is_err()); // the pre-07/29 size is rejected
        assert!(parse_player_self_pos(&[0; 41]).is_err());
        assert!(parse_player_self_pos(&[0; 43]).is_err());
    }

    #[test]
    fn parses_floats_y18_z30_x38() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[18..22].copy_from_slice(&941.50f32.to_le_bytes()); // y
        buf[30..34].copy_from_slice(&190.01f32.to_le_bytes()); // z
        buf[38..42].copy_from_slice(&654.25f32.to_le_bytes()); // x
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.x, 654.25);
        assert_eq!(p.y, 941.50);
        assert_eq!(p.z, 190.01);
    }

    // A real self-report off the 08/04 wire. Ground truth for the same moment
    // comes from the OP_SelfPos breadcrumb — a different opcode with a totally
    // different encoding (IEEE floats in 17-byte tiled records) — and the two
    // agree to 0.0000 units on all three axes. That cross-opcode agreement is
    // what pins these offsets; the 07/29 offsets decode this same packet to
    // zero.
    #[test]
    fn decodes_a_captured_self_report() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0x14, 0x08, 0x3B, 0x26, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xD4, 0x87, 0x44, 0x00, 0x04, 0xC0, 0x24, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x54, 0xC2, 0x30, 0x00, 0x00, 0x00, 0x00, 0x60, 0xB0, 0xC3,
        ];
        let p = parse_player_self_pos(&bytes).unwrap();
        assert_eq!(p.x, -352.75);
        assert_eq!(p.y, 1086.625);
        assert_eq!(p.z, -53.0);
        assert_eq!(p.spawn_id, 9787);
        assert_eq!(p.heading, 1024); // exactly 180 degrees
    }

    // Facing is an 11-bit compass value in the low bits at 22: 0 = N, a quarter
    // circle = E. The neighbouring high bits are set so a sloppy mask is caught.
    #[test]
    fn decodes_the_facing_as_a_compass_value() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let w = u32::from(HEADING_UNITS) / 4 | (0x1F_FFFFu32 << 11);
        buf[22..26].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }

    // The velocities are still unmapped for this patch; surfacing a stale field
    // would smear the marker between updates. Pinned so re-deriving them is a
    // deliberate change.
    #[test]
    fn velocity_is_not_decoded_this_patch() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[6..10].copy_from_slice(&2.26f32.to_le_bytes());
        buf[10..14].copy_from_slice(&2.26f32.to_le_bytes());
        buf[34..38].copy_from_slice(&2.26f32.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!((p.delta_x, p.delta_y, p.delta_z), (0.0, 0.0, 0.0));
    }

    #[test]
    fn zero_payload_is_origin() {
        let p = parse_player_self_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
        assert_eq!(p.heading, 0);
        assert_eq!(p.spawn_id, 0);
    }
}
