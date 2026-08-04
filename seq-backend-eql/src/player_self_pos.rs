//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
//!
//! **Re-derived 2026-07-29.** The 07/29 rotation rotated the whole opcode table
//! AND grew this report 38B -> 42B with a fully rearranged body; none of the 38B
//! offsets survive. Positions are IEEE floats in game-world units (no ×8 packing
//! — this is C>S, distinct from the S>C packed `playerSpawnPosStruct`):
//!
//! ```text
//!   /*0000*/ u16  ctr        update counter (0..5181 over a capture)
//!   /*0002*/ u16  spawnId    the local player's spawn id — BACK on the wire
//!   /*0004*/ u8   unknown0004[6]
//!   /*0010*/ f32  y          gameY
//!   /*0014*/ f32  unknown    velocity candidate (-1.32 .. 0.43)
//!   /*0018*/ f32  unknown    velocity candidate (-1.79 .. 1.92)
//!   /*0022*/ f32  x          gameX
//!   /*0026*/ u32  { heading:11 (low) | hi:21 }
//!   /*0030*/ f32  unknown    velocity candidate (-2.01 .. 1.87)
//!   /*0034*/ f32  z          gameZ
//!   /*0038*/ u32  unknown
//! ```
//!
//! **How the axes were pinned.** The three position floats fall out of a range
//! comparison against the `OP_SelfPos` breadcrumb (which reports the player's
//! real path): over 161 self-reports the field ranges match the breadcrumb's
//! per-axis ranges essentially exactly — @10 [-1559.64, 2552.56] vs the
//! breadcrumb's [-1559.64, 2552.56], @22 [-197.76, 296.00] vs [-199.73, 296.00],
//! @34 [-84.30, -43.72] vs [-84.68, -43.38]. Every other float offset in the
//! packet spans at most ±2 (the velocities) or is pinned near 0.
//!
//! Which of @10/@22 is X and which is Y is NOT taken from the breadcrumb's own
//! labels — those are in `/loc` order and transpose against the map frame, which
//! is exactly the trap that produced a silently-swapped read in an earlier patch.
//! It is settled physically instead: position updates are range-limited, so the
//! player must sit inside the cloud of spawns the server is streaming them. Under
//! `@10 = y, @22 = x` the player is within 300 units of a visible spawn in
//! **490 of 518** samples (median 104); transposed, **0 of 518** (median 946).
//! The ground-truth cloud is the untouched `OP_MobUpdate` / `OP_NpcMoveUpdate`
//! streams, i.e. the same map frame `SpawnShell` and the map use.
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
/// The facing is the low 11 bits of the dword at offset 26. Width and scale
/// carry over from upstream's `eqlClientSelfPosStruct` (an 11-bit facing), and
/// the location was re-measured for the 42B body: scored against travel bearing
/// over 46 movement legs it lands at a 6.8 degree median, and the same field read
/// with 12/13/15/16-bit windows yields the identical angle (they are the same
/// value with extra low bits), which is what fixes the low edge at bit 0 of @26.
///
/// INVERTED like every other heading: `heading_deg(field, 11)`. Read
/// uninverted it mirrors — a left turn rotates the marker right. Calibrate the
/// sense on a TURN; facing-vs-travel-bearing can't see a mirror, since the
/// bearing shares the frame.
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
    let y = read_f32_le(bytes, 10);
    let x = read_f32_le(bytes, 22);
    let z = read_f32_le(bytes, 34);

    // The velocity components have NOT been located for this patch and are
    // deliberately surfaced as 0 rather than read from a plausible-looking
    // offset: a wrong velocity would smear the player marker between updates.
    // Candidates are the three small-range floats at 14, 18 and 30 (all within
    // ±2.1, the right magnitude for the ±2.26 units/tick of a full run); the
    // capture has too little sustained movement to tell which maps to which
    // axis.
    let heading = (read_u32_le(bytes, 26) & 0x7FF) as u16;

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
    fn parses_floats_y10_x22_z34() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[10..14].copy_from_slice(&941.50f32.to_le_bytes()); // y
        buf[22..26].copy_from_slice(&654.25f32.to_le_bytes()); // x
        buf[34..38].copy_from_slice(&190.01f32.to_le_bytes()); // z
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.x, 654.25);
        assert_eq!(p.y, 941.50);
        assert_eq!(p.z, 190.01);
    }

    // A real self-report off the 07/29 wire. Ground truth for the same moment
    // comes from the OP_SelfPos breadcrumb, which agrees to 0.0000 on all three
    // axes.
    #[test]
    fn decodes_a_captured_self_report() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0x00, 0x00, 0x5B, 0x3D, 0x00, 0x00, 0x00, 0x00, 0xED, 0xA3, 0x00, 0xD6, 0x1D, 0x45,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x57, 0x43, 0x1B, 0x02,
            0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x32, 0xC2, 0x00, 0x00, 0x4F, 0xCF,
        ];
        let p = parse_player_self_pos(&bytes).unwrap();
        assert_eq!(p.x, 215.0);
        assert_eq!(p.y, 2525.375);
        assert_eq!(p.z, -44.625);
        assert_eq!(p.spawn_id, 15707);
        assert_eq!(p.heading, 539); // ~94.7 degrees, i.e. due east
    }

    // Facing is an 11-bit compass value in the low bits at 26: 0 = N, a quarter
    // circle = E. The neighbouring high bits are set so a sloppy mask is caught.
    #[test]
    fn decodes_the_facing_as_a_compass_value() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let w = u32::from(HEADING_UNITS) / 4 | (0x1F_FFFFu32 << 11);
        buf[26..30].copy_from_slice(&w.to_le_bytes());
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
        buf[18..22].copy_from_slice(&2.26f32.to_le_bytes());
        buf[30..34].copy_from_slice(&2.26f32.to_le_bytes());
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
