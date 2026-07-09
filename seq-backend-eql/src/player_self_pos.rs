//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`,
//! both DIR_Client and DIR_Server when `len==42`).
//!
//! Mostly floats, with three bitfield-packed `u32` storage units. Field offsets
//! and bit widths come straight from `everquest.h:playerSelfPosStruct`
//! (re-derived per patch — the field order shuffles; do not memorize):
//!
//! ```text
//!   /*0002*/ uint16_t spawnId;
//!   /*0006*/ float    y;
//!   /*0010*/ pitch:12, heading:12, padding:8
//!   /*0014*/ float    deltaY;
//!   /*0018*/ animation:10, padding:22
//!   /*0022*/ float    z;
//!   /*0026*/ float    x;
//!   /*0030*/ float    deltaX;
//!   /*0034*/ deltaHeading:10, padding:22
//!   /*0038*/ float    deltaZ;
//! ```
//!
//! With `#pragma pack(1)`, bitfields within a storage unit pack LSB-first, so
//! the first-declared field occupies the low bits.

use crate::eqstructs::sign_extend;
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
    /// 12-bit unsigned (full 0..4095 turn range).
    pub heading: u16,
    /// 10-bit signed.
    pub delta_heading: i16,
    /// 10-bit signed.
    pub animation: i16,
    /// 12-bit unsigned pitch.
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

    let y = read_f32_le(bytes, 6);

    // offset 10: pitch:12, heading:12, padding:8
    let w10 = read_u32_le(bytes, 10);
    let pitch = (w10 & 0xFFF) as u16;
    let heading = ((w10 >> 12) & 0xFFF) as u16;

    let delta_y = read_f32_le(bytes, 14);

    // offset 18: animation:10, padding:22
    let w18 = read_u32_le(bytes, 18);
    let animation = sign_extend(w18 & 0x3FF, 10) as i16;

    let z = read_f32_le(bytes, 22);
    let x = read_f32_le(bytes, 26);
    let delta_x = read_f32_le(bytes, 30);

    // offset 34: deltaHeading:10, padding:22
    let w34 = read_u32_le(bytes, 34);
    let delta_heading = sign_extend(w34 & 0x3FF, 10) as i16;

    let delta_z = read_f32_le(bytes, 38);

    Ok(PlayerSelfPos {
        spawn_id,
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
        assert!(parse_player_self_pos(&[0; 41]).is_err());
        assert!(parse_player_self_pos(&[0; 43]).is_err());
    }

    #[test]
    fn parses_floats_and_spawn_id() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        buf[6..10].copy_from_slice(&3.0f32.to_le_bytes()); // y
        buf[14..18].copy_from_slice(&5.0f32.to_le_bytes()); // deltaY
        buf[22..26].copy_from_slice(&2.0f32.to_le_bytes()); // z
        buf[26..30].copy_from_slice(&6.0f32.to_le_bytes()); // x
        buf[30..34].copy_from_slice(&1.0f32.to_le_bytes()); // deltaX
        buf[38..42].copy_from_slice(&4.0f32.to_le_bytes()); // deltaZ
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1234);
        assert_eq!(p.y, 3.0);
        assert_eq!(p.delta_y, 5.0);
        assert_eq!(p.z, 2.0);
        assert_eq!(p.x, 6.0);
        assert_eq!(p.delta_x, 1.0);
        assert_eq!(p.delta_z, 4.0);
    }

    #[test]
    fn pitch_heading_pack_offset10() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // pitch=0xFFF (12-bit), heading=0xABC (12-bit).
        let w: u32 = 0xFFF | (0xABCu32 << 12);
        buf[10..14].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.pitch, 0xFFF);
        assert_eq!(p.heading, 0xABC);
    }

    #[test]
    fn animation_offset18() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // animation = -3 (0x3FD in 10-bit two's complement).
        let w: u32 = 0x3FD;
        buf[18..22].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.animation, -3);
    }

    #[test]
    fn delta_heading_offset34() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // deltaHeading = -1 (0x3FF in 10-bit).
        let w: u32 = 0x3FF;
        buf[34..38].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.delta_heading, -1);
    }
}
