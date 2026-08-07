//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
//!
//! **Re-derived 2026-08-06 for the 08/05 mini-patch.** This body has now been
//! rearranged on two consecutive patches while keeping its 42-byte size, so no
//! size gate can ever catch the change — only a range check against the
//! breadcrumb will. Positions are IEEE floats in game-world units (no ×8
//! packing — this is C>S, distinct from the S>C packed `playerSpawnPosStruct`):
//!
//! ```text
//!   /*0000*/ u16  ctr        update counter
//!   /*0002*/ u16  spawnId    the local player's spawn id (the PHANTOM TWIN's)
//!   /*0004*/ u8   unknown0004[6]
//!   /*0010*/ f32  x          gameX
//!   /*0014*/ f32  unknown    velocity candidate
//!   /*0018*/ f32  y          gameY
//!   /*0022*/ f32  z          gameZ
//!   /*0026*/ u32  unknown    (upstream: stale quiet-NaN facing payload)
//!   /*0030*/ f32  unknown    velocity candidate
//!   /*0034*/ f32  unknown    velocity candidate
//!   /*0038*/ u32  { heading:11 (low) | hi:21 }
//! ```
//!
//! Upstream declares this struct 44B (`tail[2]` past the heading word). The
//! wire is **42B** — 1054 C>S bodies in the 08/06 capture, none at 44 — so the
//! tail is dropped here and `PAYLOAD_LEN` stays 42. Both payloads are gated
//! `none`, so an over-long declaration would not warn; it would just hand this
//! parser a short buffer. That was true of the 08/04 wire too.
//!
//! **How the axes were pinned.** The three position floats fall out of a range
//! comparison against the `OP_SelfPos` breadcrumb (which reports the player's
//! real path, and which is re-confirmed each rotation by its own `1 + N*17`
//! invariant). Over 1054 self-reports vs 22350 breadcrumb records the ranges
//! match essentially exactly — @10 [-547.8, 488.0] vs the breadcrumb's
//! [-547.8, 488.0], @18 [-2627.3, 1152.4] vs [-2627.3, 1152.6], @22
//! [-67.6, 15.5] vs [-67.8, 15.6]. Every other float offset spans at most ±3.6
//! (the velocities) or is pinned near 0.
//!
//! Which offset is X and which is Y is NOT taken from the breadcrumb's own
//! labels — those are in `/loc` order and transpose against the map frame, which
//! is exactly the trap that produced a silently-swapped read in an earlier patch.
//! It is settled by matching each field's observed RANGE to the corresponding
//! breadcrumb axis range (above); the three spans (1036 / 3780 / 83 units) are
//! far enough apart that the assignment is unambiguous.
//!
//! Previous layouts, kept so a re-derivation can tell drift from a bad read:
//! 08/04 was y@18 / z@30 / x@38 / heading@22; 07/29 was y@10 / x@22 / z@34.
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
/// The facing is the low 11 bits of the dword at offset 38 (moved from 22 by
/// the 08/05 mini-patch, agreeing with upstream's re-derivation). Scored
/// against travel bearing over 469 movement legs it lands at a **0.64 degree**
/// median, and the same field read as a 10/12/13-bit window gives the identical
/// angle (the extra bits are fractional), which is what fixes the low edge at
/// bit 0 of @38. A random field would score ~90.
///
/// INVERTED like every other heading: `heading_deg(field, 11)`. Read
/// uninverted it mirrors — a left turn rotates the marker right. Only the
/// OFFSET moved this patch; width, scale and sense are unchanged, so the
/// downstream inversion carries over untouched, and that was **confirmed
/// in-game on 2026-08-05**: the reticle tracks the turn instead of mirroring
/// it. Re-measured on the 08/05 wire, uninverted scores 0.64 deg against travel
/// bearing and inverted 71.19 — but that is about the FIELD's sense, not the
/// daemon's convention, which the turn test settled.
///
/// Calibrate the sense on a TURN, never on facing-vs-travel-bearing: the
/// bearing shares the frame, so it cannot see a mirror. That is why the
/// 0.64-degree fit above pins the field's LOCATION but says nothing about the
/// daemon-frame convention.
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
    let x = read_f32_le(bytes, 10);
    let y = read_f32_le(bytes, 18);
    let z = read_f32_le(bytes, 22);

    // The velocity components have NOT been located for this patch and are
    // deliberately surfaced as 0 rather than read from a plausible-looking
    // offset: a wrong velocity would smear the player marker between updates.
    // Candidates are the three small-range floats at 14, 30 and 34 (all within
    // ±3.6, the right magnitude for the ±2.26 units/tick of a full run); the
    // capture has too little sustained movement to tell which maps to which
    // axis.
    let heading = (read_u32_le(bytes, 38) & 0x7FF) as u16;

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
    fn parses_floats_x10_y18_z22() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[10..14].copy_from_slice(&654.25f32.to_le_bytes()); // x
        buf[18..22].copy_from_slice(&941.50f32.to_le_bytes()); // y
        buf[22..26].copy_from_slice(&190.01f32.to_le_bytes()); // z
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.x, 654.25);
        assert_eq!(p.y, 941.50);
        assert_eq!(p.z, 190.01);
    }

    // A real self-report off the 08/05 wire. Ground truth for the same moment
    // comes from the OP_SelfPos breadcrumb — a different opcode with a totally
    // different encoding (IEEE floats in 17-byte tiled records) — and the two
    // agree to 0.0000 units on all three axes. That cross-opcode agreement is
    // what pins these offsets; the 08/04 offsets decode this same packet to
    // zero or garbage.
    #[test]
    fn decodes_a_captured_self_report() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0x00, 0x00, 0xC3, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x96, 0x43,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x5C, 0xDE, 0xC4, 0x00, 0x00, 0x38, 0x40, 0x00, 0x10,
            0xF6, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFA, 0x07, 0x00, 0xD4,
        ];
        let p = parse_player_self_pos(&bytes).unwrap();
        assert_eq!(p.x, 301.875);
        assert_eq!(p.y, -1778.875);
        assert_eq!(p.z, 2.875);
        assert_eq!(p.spawn_id, 963);
        assert_eq!(p.heading, 2042); // ~358.9 degrees, i.e. just west of north
    }

    // Facing is an 11-bit compass value in the low bits at 38: 0 = N, a quarter
    // circle = E. The neighbouring high bits are set so a sloppy mask is caught.
    #[test]
    fn decodes_the_facing_as_a_compass_value() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let w = u32::from(HEADING_UNITS) / 4 | (0x1F_FFFFu32 << 11);
        buf[38..42].copy_from_slice(&w.to_le_bytes());
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
        buf[14..18].copy_from_slice(&2.26f32.to_le_bytes());
        buf[30..34].copy_from_slice(&2.26f32.to_le_bytes());
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
