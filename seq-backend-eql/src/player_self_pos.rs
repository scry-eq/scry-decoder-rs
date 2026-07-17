//! Parser for the 38-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
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
//!   /*0010*/ f32  z          (gameZ)
//!   /*0014*/ f32  x          (gameX)
//!   /*0018*/ u32  { lowfrac:8 | heading:13 @bit8 | turnrate:11 }
//!   /*0026*/ f32  y          (gameY)
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
//! collapse to 4 cardinals (only bits 10-11 of the low-12 window survived) and the
//! turn-rate bits above the heading (24..31) look like corruption. Bits 24..31 are
//! the **turn rate** (nonzero only while turning), not garbage; there is nothing to
//! reject. The daemon maps via `360 - ((h*360) >> 13)` → /loc S=180, W=270.
//!
//! **No spawnId field** — unlike the old 42B (`spawnId@2`), the client's self
//! report carries no id (the server keys the connection). The daemon's
//! `EqlDispatch::playerUpdateSelf` therefore applies this to the local player
//! directly and the self-id is adopted elsewhere (`SpawnShell::zoneEntry`
//! name-match), so `spawn_id` is surfaced as 0. Deltas are not yet located in the
//! 38B form and are surfaced as 0 (only the speed indicator uses them).

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 38;

/// Full circle in wire heading units (13-bit field → 8192 steps).
pub const HEADING_UNITS: u16 = 8192;

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSelfPos {
    /// Not carried on the 38B C>S wire (the client's self report has no id) — 0.
    pub spawn_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Not yet located in the 38B form — surfaced as 0 (only speed uses them).
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

    let z = read_f32_le(bytes, 10);
    let x = read_f32_le(bytes, 14);
    // offset 18 packs `lowfrac:8 | heading:13 @bit8 | turnrate:11`. The facing is
    // the 13-bit field at bit 8 (8192 per circle) — reliable on every packet,
    // moving or stationary. The low 8 bits are a separate sub-fraction that reads
    // 0 when not translating; the bits above the heading are the turn rate. See
    // the module doc: verified against a stationary 360-spin capture.
    let w = read_u32_le(bytes, 18);
    let heading = ((w >> 8) & 0x1FFF) as u16;
    let y = read_f32_le(bytes, 26);
    // Velocity (units/tick, ±~2.26 = full run speed) — cracked 2026-07-17 vs a
    // run-south-then-run-west /loc capture: deltaY@6 lit up (−2.27) only during
    // the south leg, deltaX@22 (+2.26) only during the west leg, both ~0 while
    // still; deltaZ@30 is small and nonzero only while translating (slope bob).
    // deltaY@6 held its offset across the 07/14 rearrangement (the old 42B form
    // also carried Y-velocity @6); only deltaX moved (@26 → @22).
    let delta_y = read_f32_le(bytes, 6);
    let delta_x = read_f32_le(bytes, 22);
    let delta_z = read_f32_le(bytes, 30);

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
    fn parses_floats_x14_y26_z10() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[10..14].copy_from_slice(&190.01f32.to_le_bytes()); // z
        buf[14..18].copy_from_slice(&654.25f32.to_le_bytes()); // x
        buf[26..30].copy_from_slice(&941.50f32.to_le_bytes()); // y
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.x, 654.25);
        assert_eq!(p.y, 941.50);
        assert_eq!(p.z, 190.01);
        assert_eq!(p.spawn_id, 0);
    }

    #[test]
    fn heading_is_13_bit_at_bit8() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // heading = 0x1FFF (13-bit max) at bit 8; the low 8 bits (sub-fraction) and
        // the turn-rate bits above must not bleed into it.
        buf[18..22].copy_from_slice(&(0xFFu32 | (0x1FFFu32 << 8) | (0x7FFu32 << 21)).to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.heading, 0x1FFF);
        assert!(p.heading < HEADING_UNITS);
    }

    #[test]
    fn turning_packet_still_decodes_heading() {
        // A real turning packet has nonzero turn-rate bits (24..31); the heading is
        // still valid — nothing to reject. idx-72 of the spin capture: w=0xebcef000
        // -> heading = (0xebcef000 >> 8) & 0x1FFF = 0x1cef0 & 0x1FFF ... exercised
        // here with a crafted value so the field is unambiguous.
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[18..22].copy_from_slice(&(0x1234u32 << 8 | 0xdd_u32 << 24).to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.heading, 0x1234 & 0x1FFF);
    }

    #[test]
    fn zero_payload_is_origin() {
        let p = parse_player_self_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
        assert_eq!(p.heading, 0);
    }
}
