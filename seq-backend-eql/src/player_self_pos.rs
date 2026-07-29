//! Parser for the 38-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
//!
//! **Positions re-derived 2026-07-28** (07/28 rotation) against the OP_SelfPos
//! breadcrumb as ground truth — the breadcrumb reports the player's real path,
//! so the self-report's fields are SOLVED against it rather than guessed:
//!
//! ```text
//!   /*0006*/ f32 y      (was deltaY)
//!   /*0010*/ f32 z      (unchanged)
//!   /*0034*/ f32 x      (was at 14)
//! ```
//!
//! Scored over 8000 self-reports by whether the decoded triple lands on a
//! position the player actually occupied: the pre-patch layout (x@14, y@26,
//! z@10) hits 0%, this one hits 100%. The struct SIZE is unchanged at 38B, so
//! no size check could have caught this — the symptom was a self position stuck
//! at {0,0,4} while every other spawn decoded correctly.
//!
//! Heading and the velocities are NOT re-derived and read 0 (see the parser
//! body).
//!
//! **The facing field is LOCATED but not calibrated (2026-07-28).** A stationary
//! spin capture (112 consecutive self-reports at one position, player rotating)
//! puts it at **bytes 24..25, u16 LE**: over that window it steps monotonically
//! 80 times in one direction with a single reversal, wrapping cleanly — the only
//! field in the packet that behaves like a rotation. Observed range 189..32576
//! and bit 15 is never set across the capture, so the field is 15-bit or the top
//! bit lives elsewhere.
//!
//! What is still missing is the SCALE and zero-point: the swept total works out
//! to ~3.8 revolutions if a circle is 32768 units, or ~1.9 if it is 65536, and
//! the capture has no known facing to break the tie (the player span but never
//! ran, and the server does not echo the local player's own heading back). To
//! finish it, capture a straight run on a cardinal — travel direction from the
//! breadcrumb then gives the facing in degrees for free, and one leg fixes both
//! scale and offset. Until then this reads 0 rather than guessing a convention.
//!
//! **Re-cracked 2026-07-14** against a `/loc` ground-truth capture
//! (`eql-locref.vpk`, 3 known points, exact match). The 2026-07-14 patch rotated
//! the opcode id (0x7171 -> 0x5188) and shrank this report 42B -> 38B with a
//! **fully rearranged layout** (the old 42B had x@18/y@10/z@30/heading@14; none
//! of those survive). Positions are IEEE floats in game-world units (no ×8
//! packing — this is C>S, distinct from the S>C packed `playerSpawnPosStruct`):
//!
//! ```text
//!   /*0000*/ f32  counter    (~16640, +~0.1/tick; unused)
//!   /*0006*/ f32  deltaY     (Y-velocity; cracked 2026-07-17)
//!   /*0010*/ f32  z          (gameZ)
//!   /*0014*/ f32  x          (gameX)
//!   /*0018*/ u32  { lowfrac:8 | heading:13 @bit8 | hi:11 (NOT a turn rate) }
//!   /*0022*/ f32  deltaX     (X-velocity)
//!   /*0026*/ f32  y          (gameY)
//!   /*0030*/ f32  deltaZ     (Z-velocity)
//! ```
//!
//! Verified: X@14/Y@26/Z@10 match all 3 `/loc` points exactly (X 654/744/1156,
//! Y 942/388/373, Z 190/33/58).
//!
//! **Heading re-cracked 2026-07-15** against a stationary full-360 spin capture
//! (position pinned, two clockwise rotations). The facing is a
//! **13-bit field at bit 8** (`(w>>8) & 0x1FFF`, 8192 per circle), NOT the low 12
//! bits: `(w>>8)&0x1FFF` sweeps two clean monotonic cycles across the spin and
//! puts the /loc south/west walks at 4072/2100 → 90° apart. The earlier "low 12"
//! read was wrong — bit 0..7 (`lowfrac`) is a separate sub-field that reads 0 when
//! the player isn't translating, which made a *rotating-but-stationary* facing
//! collapse to 4 cardinals (only bits 10-11 of the low-12 window survived). The
//! bits above the heading (21..31) read 0 on most packets and spike to garbage on a
//! few — **there is no turn-rate field** (confirmed 2026-07-17 vs the spin capture:
//! every delta reads 0 while the facing sweeps ~245 units/frame, so EQL sends the
//! absolute heading each frame and no rate). The daemon maps the facing via
//! `360 - ((h*360) >> 13)` → /loc S=180, W=270; delta_heading stays 0.
//!
//! **No spawnId field** — unlike the old 42B (`spawnId@2`), the client's self
//! report carries no id (the server keys the connection). The daemon's
//! `EqlDispatch::playerUpdateSelf` therefore applies this to the local player
//! directly and the self-id is adopted elsewhere (`SpawnShell::zoneEntry`
//! name-match), so `spawn_id` is surfaced as 0. Velocity cracked 2026-07-17
//! (run-south-then-west /loc capture): deltaY@6, deltaX@22, deltaZ@30 (f32,
//! ±~2.26 units/tick = full run). delta_heading has no wire field (see above).

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 38;

/// Full circle in wire heading units (11-bit field → 2048 steps).
///
/// Field boundaries per upstream's `eqlClientSelfPosStruct` (legends branch,
/// 2026-07-28): the facing is bits 20..30 of the dword at offset 22. Deriving
/// it locally could only bound it as "the u16 at 24..25, top bit never set",
/// i.e. 15 bits — the low 4 bits of that window are a separate sub-field, not
/// facing precision, which the capture cannot reveal but the struct does. Same
/// bits, same angle to within 0.18 degrees; upstream's split is the right one.
///
/// The SENSE is ours and is measured, not assumed: calibrated against travel
/// direction (a running player faces where they go, so each breadcrumb leg's
/// bearing IS the facing). Compass degrees are `field * 360 / 2048` with NO
/// inversion — 0 = N, 512 = E, 1024 = S, 1536 = W — verified end to end at a
/// median error of 4.5 degrees over 278 run legs. Note this is the opposite
/// sense to the spawn headings, which DO invert via `heading_deg`.
pub const HEADING_UNITS: u16 = 2048;

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSelfPos {
    /// Not carried on the 38B C>S wire (the client's self report has no id) — 0.
    pub spawn_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Velocity units/tick (deltaY@6, deltaX@22, deltaZ@30); ±~2.26 = full run.
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    /// 13-bit unsigned (0..8191, 8192 per circle); daemon maps via
    /// `360 - ((h*360) >> 13)`. Valid on every packet — turning included.
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

fn read_f32_le(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(read_u32_le(bytes, at))
}

pub fn parse_player_self_pos(bytes: &[u8]) -> Result<PlayerSelfPos, PlayerSelfPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSelfPosError::BadLength(bytes.len()));
    }

    // Position offsets re-derived 2026-07-28 against the breadcrumb (see the
    // module doc). z held its offset; y and x moved.
    let y = read_f32_le(bytes, 6);
    let z = read_f32_le(bytes, 10);
    let x = read_f32_le(bytes, 34);

    // The velocity components have NOT been re-derived for this patch and are
    // deliberately surfaced as 0 rather than read from their pre-patch offsets:
    // a wrong velocity would smear the player marker between updates. Candidates
    // are offsets 14 and 30, both nonzero on ~49% of self-reports (this
    // capture's movement duty cycle) while 0/22/26 are nonzero on ~100%.
    // Facing: bits 20..30 of the dword at 22 (see HEADING_UNITS).
    let heading = ((read_u32_le(bytes, 22) >> 20) & 0x7FF) as u16;
    let delta_x = 0.0;
    let delta_y = 0.0;
    let delta_z = 0.0;

    Ok(PlayerSelfPos {
        spawn_id: 0,
        x,
        y,
        z,
        delta_x,
        delta_y,
        delta_z,
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
        assert!(parse_player_self_pos(&[0; 42]).is_err()); // the pre-07/14 size is rejected
        assert!(parse_player_self_pos(&[0; 37]).is_err());
        assert!(parse_player_self_pos(&[0; 39]).is_err());
    }

    #[test]
    fn parses_floats_y6_z10_x34() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[6..10].copy_from_slice(&941.50f32.to_le_bytes()); // y
        buf[10..14].copy_from_slice(&190.01f32.to_le_bytes()); // z
        buf[34..38].copy_from_slice(&654.25f32.to_le_bytes()); // x
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.x, 654.25);
        assert_eq!(p.y, 941.50);
        assert_eq!(p.z, 190.01);
        assert_eq!(p.spawn_id, 0);
    }

    // A real self-report off the wire, post-07/28. Ground truth for the same
    // moment comes from the OP_SelfPos breadcrumb: the player stood at
    // x 2037, y -1889, z 1.
    #[test]
    fn decodes_a_captured_self_report() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0xB8, 0x4E, 0xBD, 0x3A, 0x00, 0x00, 0x00, 0x20, 0xEC, 0xC4, 0x00, 0x00, 0x80, 0x3F,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x9D, 0xC7,
            0xF7, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0xFE, 0x44,
        ];
        let p = parse_player_self_pos(&bytes).unwrap();
        assert_eq!(p.x, 2037.0);
        assert_eq!(p.y, -1889.0);
        assert_eq!(p.z, 1.0);
    }

    // Facing is a 15-bit compass value at 24..25: 0 = N, a quarter circle = E.
    #[test]
    fn decodes_the_facing_as_a_compass_value() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[22..26].copy_from_slice(&((u32::from(HEADING_UNITS) / 4) << 20).to_le_bytes());
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
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!((p.delta_x, p.delta_y, p.delta_z), (0.0, 0.0, 0.0));
    }

    #[test]
    fn zero_payload_is_origin() {
        let p = parse_player_self_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
        assert_eq!(p.heading, 0);
    }
}
