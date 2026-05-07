//! Parser for the 28-byte `playerSpawnPosStruct` (`OP_ClientUpdate`,
//! DIR_Server only — for spawns other than the local player).
//!
//! Every position field is a packed bitfield. Layout from
//! `everquest.h:playerSpawnPosStruct`:
//!
//! ```text
//!   /*0000*/ uint16_t spawnId;
//!   /*0002*/ uint16_t spawnId2;
//!   /*0004*/ pitch:12, y:19, padding:1
//!   /*0008*/ heading:12, animation:10, padding:10
//!   /*0012*/ x:19, padding:13
//!   /*0016*/ z:19, deltaZ:13
//!   /*0020*/ deltaHeading:10, deltaY:13, padding:9
//!   /*0024*/ deltaX:13, padding:19
//! ```
//!
//! The C++ daemon shifts the y/x/z fields right by 3 to convert from
//! 1/8-unit fixed point. This parser surfaces the *raw* sign-extended
//! values; the bridge layer applies the shift to keep behaviour
//! identical to the existing C++ `pupdate->y >> 3` cast.

use seq_eqstructs::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 28;

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

    let spawn_id  = read_u16_le(bytes, 0);
    let spawn_id2 = read_u16_le(bytes, 2);

    // pitch:12, y:19, padding:1 at offset 4.
    let w0 = read_u32_le(bytes, 4);
    let pitch = (w0 & 0xFFF) as u16;
    let y = sign_extend((w0 >> 12) & 0x7_FFFF, 19);

    // heading:12, animation:10, padding:10 at offset 8.
    let w1 = read_u32_le(bytes, 8);
    let heading = (w1 & 0xFFF) as u16;
    let animation = sign_extend((w1 >> 12) & 0x3FF, 10) as i16;

    // x:19, padding:13 at offset 12.
    let w2 = read_u32_le(bytes, 12);
    let x = sign_extend(w2 & 0x7_FFFF, 19);

    // z:19, deltaZ:13 at offset 16.
    let w3 = read_u32_le(bytes, 16);
    let z = sign_extend(w3 & 0x7_FFFF, 19);
    let delta_z = sign_extend((w3 >> 19) & 0x1FFF, 13);

    // deltaHeading:10, deltaY:13, padding:9 at offset 20.
    let w4 = read_u32_le(bytes, 20);
    let delta_heading = sign_extend(w4 & 0x3FF, 10) as i16;
    let delta_y = sign_extend((w4 >> 10) & 0x1FFF, 13);

    // deltaX:13, padding:19 at offset 24.
    let w5 = read_u32_le(bytes, 24);
    let delta_x = sign_extend(w5 & 0x1FFF, 13);

    Ok(PlayerSpawnPos {
        spawn_id, spawn_id2,
        x, y, z,
        delta_x, delta_y, delta_z,
        heading, delta_heading, animation, pitch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_player_spawn_pos(&[0; 27]).is_err());
        assert!(parse_player_spawn_pos(&[0; 29]).is_err());
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
    fn extracts_y_in_first_packed_word() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // pitch=0xABC (12 bits), y=-1 (0x7_FFFF in 19-bit two's complement).
        let w: u32 = 0xABC | (0x7_FFFFu32 << 12);
        buf[4..8].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.pitch, 0xABC);
        assert_eq!(p.y, -1);
    }

    #[test]
    fn extracts_delta_z_signed_13_bit() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // z=0, deltaZ=-2 (0x1FFE in 13-bit two's complement).
        let w: u32 = 0x1FFEu32 << 19;
        buf[16..20].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.z, 0);
        assert_eq!(p.delta_z, -2);
    }

    #[test]
    fn extracts_heading_animation_pack() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // heading=0xFFF (12-bit), animation=-1 (0x3FF in 10-bit).
        let w: u32 = 0xFFF | (0x3FFu32 << 12);
        buf[8..12].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.heading, 0xFFF);
        assert_eq!(p.animation, -1);
    }

    #[test]
    fn delta_heading_and_delta_y_pack() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // deltaHeading=-3 (0x3FD), deltaY=42.
        let w: u32 = 0x3FD | (42u32 << 10);
        buf[20..24].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_spawn_pos(&buf).unwrap();
        assert_eq!(p.delta_heading, -3);
        assert_eq!(p.delta_y, 42);
    }
}
