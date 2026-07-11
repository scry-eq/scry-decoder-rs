//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report).
//!
//! **Re-derived 2026-07-10** from `eqlegends-levelup.vpk`. The prior layout
//! (x@26/y@6/z@22, heading in the @10 word) was misaligned — it read the
//! *velocity* fields as position, so `Player::applySelfPosition` painted the PC
//! at ~origin, making the player dot flicker to the map corner (~7% of frames,
//! interleaved with the correct OP_NpcMoveUpdate position). The offsets were
//! wrong on both the 07-07 and 07-10 captures (position has always been at
//! @18/@10/@30), so this is a long-standing bug, not a fresh patch drift.
//!
//! Layout on the current wire (little-endian; floats are game-world units, no
//! ×8 packing — this is C>S, distinct from the S>C packed `playerSpawnPosStruct`):
//!
//! ```text
//!   /*0002*/ u16  spawnId
//!   /*0006*/ f32  deltaY
//!   /*0010*/ f32  y
//!   /*0014*/ u32  heading:11 (0..2047, h2048)   + unmapped high bits
//!   /*0018*/ f32  x
//!   /*0022*/ u32  (unmapped — pitch/flags; unused by the daemon)
//!   /*0026*/ f32  deltaX
//!   /*0030*/ f32  z
//!   /*0034*/ f32  deltaZ
//!   /*0038*/ u32  (unmapped — pitch/anim; unused by the daemon)
//! ```
//!
//! Evidence (see `showeq-daemon/OPCODES_LEGENDS.md`): x@18/y@10 land 3.9u from a
//! mob being meleed (same frame as the mob positions); z@30 reads 31.7 while
//! standing. heading@14 correlates R=0.88 with the player's own movement
//! direction (a player faces where they run) and the daemon's existing
//! `360 - ((h*360) >> 11)` conversion confirms the 11-bit scale. deltaX@26 /
//! deltaY@6 correlate r≈0.83 with the per-axis position delta and yield speed
//! ≈ 1.0 through the daemon's `×80/119` formula. deltaZ@34 (r=0.58, best
//! candidate). pitch/animation/deltaHeading are not consumed by
//! `applySelfPosition`, so the two unmapped `@22`/`@38` words are surfaced as 0.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 42;

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSelfPos {
    pub spawn_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    /// 11-bit unsigned (0..2047, h2048); daemon maps via `360 - ((h*360) >> 11)`.
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

fn read_u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

pub fn parse_player_self_pos(bytes: &[u8]) -> Result<PlayerSelfPos, PlayerSelfPosError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(PlayerSelfPosError::BadLength(bytes.len()));
    }

    let spawn_id = read_u16_le(bytes, 2);

    let delta_y = read_f32_le(bytes, 6);
    let y = read_f32_le(bytes, 10);
    // offset 14: heading in the low 11 bits (h2048, 0..2047); high bits unmapped.
    let heading = (read_u32_le(bytes, 14) & 0x7FF) as u16;
    let x = read_f32_le(bytes, 18);
    let delta_x = read_f32_le(bytes, 26);
    let z = read_f32_le(bytes, 30);
    let delta_z = read_f32_le(bytes, 34);

    Ok(PlayerSelfPos {
        spawn_id,
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
        assert!(parse_player_self_pos(&[0; 41]).is_err());
        assert!(parse_player_self_pos(&[0; 43]).is_err());
    }

    #[test]
    fn parses_position_deltas_and_spawn_id() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[2..4].copy_from_slice(&0x12b7u16.to_le_bytes()); // spawnId
        buf[6..10].copy_from_slice(&1.5f32.to_le_bytes()); // deltaY
        buf[10..14].copy_from_slice(&(-24.6f32).to_le_bytes()); // y
        buf[18..22].copy_from_slice(&(-263.0f32).to_le_bytes()); // x
        buf[26..30].copy_from_slice(&0.5f32.to_le_bytes()); // deltaX
        buf[30..34].copy_from_slice(&31.8f32.to_le_bytes()); // z
        buf[34..38].copy_from_slice(&(-2.0f32).to_le_bytes()); // deltaZ
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x12b7);
        assert_eq!(p.x, -263.0);
        assert_eq!(p.y, -24.6);
        assert_eq!(p.z, 31.8);
        assert_eq!(p.delta_x, 0.5);
        assert_eq!(p.delta_y, 1.5);
        assert_eq!(p.delta_z, -2.0);
    }

    #[test]
    fn heading_is_11_bit_at_offset14() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // heading = 0x7FF (11-bit max); high bits set must be masked off.
        buf[14..18].copy_from_slice(&(0x7FFu32 | (0x1Fu32 << 11)).to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.heading, 0x7FF);
    }

    #[test]
    fn zero_payload_is_origin() {
        let p = parse_player_self_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
        assert_eq!(p.heading, 0);
    }
}
