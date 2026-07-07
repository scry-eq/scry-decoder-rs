//! Parser for the `playerSpawnPosStruct` (`OP_ClientUpdate`, DIR_Server only —
//! for spawns other than the local player).
//!
//! Every position field is a packed bitfield, LSB-first within each 32-bit
//! word. Layout from `everquest.h:playerSpawnPosStruct` (re-derived per patch —
//! the packing shuffles; do not memorize):
//!
//! ```text
//!   /*0000*/ uint16_t spawnId;
//!   /*0002*/ uint16_t spawnId2;
//!   /*0004*/ animation:10, pitch:12, padding:10
//!   /*0008*/ deltaZ:13,    deltaHeading:10, padding:9
//!   /*0012*/ z:19,         heading:12,      padding:1
//!   /*0016*/ deltaX:13,    y:19
//!   /*0020*/ deltaY:13,    x:19
//!   /*0024*/
//! ```
//!
//! The C++ daemon shifts the y/x/z fields right by 3 (1/8-unit fixed point) and
//! the deltas right by 2. This parser surfaces the *raw* sign-extended values;
//! the daemon applies the shift to keep behaviour identical to the pre-rust
//! `pupdate->y >> 3` cast.

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
    /// Raw 13-bit signed; daemon applies `>> 2` for fixed-point conv.
    pub delta_x: i32,
    pub delta_y: i32,
    pub delta_z: i32,
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

    // offset 4: animation:10, pitch:12, padding:10
    let w0 = read_u32_le(bytes, 4);
    let animation = sign_extend(w0 & 0x3FF, 10) as i16;
    let pitch = ((w0 >> 10) & 0xFFF) as u16;

    // offset 8: deltaZ:13, deltaHeading:10, padding:9
    let w1 = read_u32_le(bytes, 8);
    let delta_z = sign_extend(w1 & 0x1FFF, 13);
    let delta_heading = sign_extend((w1 >> 13) & 0x3FF, 10) as i16;

    // offset 12: z:19, heading:12, padding:1
    let w2 = read_u32_le(bytes, 12);
    let z = sign_extend(w2 & 0x7_FFFF, 19);
    let heading = ((w2 >> 19) & 0xFFF) as u16;

    // offset 16: deltaX:13, y:19
    let w3 = read_u32_le(bytes, 16);
    let delta_x = sign_extend(w3 & 0x1FFF, 13);
    let y = sign_extend((w3 >> 13) & 0x7_FFFF, 19);

    // offset 20: deltaY:13, x:19
    let w4 = read_u32_le(bytes, 20);
    let delta_y = sign_extend(w4 & 0x1FFF, 13);
    let x = sign_extend((w4 >> 13) & 0x7_FFFF, 19);

    Ok(PlayerSpawnPos {
        spawn_id,
        spawn_id2,
        x,
        y,
        z,
        delta_x,
        delta_y,
        delta_z,
        heading,
        delta_heading,
        animation,
        pitch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_player_spawn_pos(&[0; 23]).is_err());
        assert!(parse_player_spawn_pos(&[0; 25]).is_err());
    }

    #[test]
    fn zero_payload_is_zero() {
        let p = parse_player_spawn_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!(p.spawn_id, 0);
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
        assert_eq!(p.z, 0);
        assert_eq!(p.delta_x, 0);
        assert_eq!(p.heading, 0);
    }

    #[test]
    fn animation_pitch_pack_offset4() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // animation=-1 (0x3FF, 10-bit), pitch=0xABC (12-bit).
        let w: u32 = 0x3FF | (0xABCu32 << 10);
        buf[4..8].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.animation, -1);
        assert_eq!(p.pitch, 0xABC);
    }

    #[test]
    fn z_heading_pack_offset12() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // z=-1 (0x7_FFFF, 19-bit), heading=0xFFF (12-bit).
        let w: u32 = 0x7_FFFF | (0xFFFu32 << 19);
        buf[12..16].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.z, -1);
        assert_eq!(p.heading, 0xFFF);
    }

    #[test]
    fn deltax_y_pack_offset16() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // deltaX=-2 (0x1FFE, 13-bit), y=42 (19-bit).
        let w: u32 = 0x1FFE | (42u32 << 13);
        buf[16..20].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.delta_x, -2);
        assert_eq!(p.y, 42);
    }

    #[test]
    fn deltay_x_pack_offset20() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // deltaY=7 (13-bit), x=-1 (0x7_FFFF, 19-bit).
        let w: u32 = 7 | (0x7_FFFFu32 << 13);
        buf[20..24].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.delta_y, 7);
        assert_eq!(p.x, -1);
    }
}
