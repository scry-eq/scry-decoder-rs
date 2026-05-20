//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`,
//! both DIR_Client and DIR_Server when `len==42`).
//!
//! Mostly floats; three bitfield-packed `u32` storage units carry
//! pitch (offset 6), animation+heading (offset 26), and deltaHeading
//! (offset 34). Field offsets and bit widths come straight from
//! `everquest.h:playerSelfPosStruct`.
//!
//! With `#pragma pack(1)` on x86-64 GCC/Clang, bitfields within a
//! storage unit are packed LSB-first. So for the `u32` at offset 26
//! containing `signed animation:10, unsigned heading:12, unsigned
//! padding:10`:
//!   - bits 0..10  → animation (signed, sign-extended to i32)
//!   - bits 10..22 → heading
//!   - bits 22..32 → padding (ignored)

use seq_eqstructs_live::sign_extend;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = 42;

#[derive(Debug, Clone, Copy)]
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

    // pitch:12 + padding:20 packed at offset 6.
    let bf_pitch = read_u32_le(bytes, 6);
    let pitch = (bf_pitch & 0xFFF) as u16;

    let delta_x = read_f32_le(bytes, 10);
    let z = read_f32_le(bytes, 14);
    let y = read_f32_le(bytes, 18);
    let delta_z = read_f32_le(bytes, 22);

    // animation:10 + heading:12 + padding:10 packed at offset 26.
    let bf_anim_heading = read_u32_le(bytes, 26);
    let animation = sign_extend(bf_anim_heading & 0x3FF, 10) as i16;
    let heading = ((bf_anim_heading >> 10) & 0xFFF) as u16;

    let delta_y = read_f32_le(bytes, 30);

    // deltaHeading:10 + padding:22 packed at offset 34.
    let bf_dh = read_u32_le(bytes, 34);
    let delta_heading = sign_extend(bf_dh & 0x3FF, 10) as i16;

    let x = read_f32_le(bytes, 38);

    Ok(PlayerSelfPos {
        spawn_id,
        x, y, z,
        delta_x, delta_y, delta_z,
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
        buf[10..14].copy_from_slice(&1.0f32.to_le_bytes()); // deltaX
        buf[14..18].copy_from_slice(&2.0f32.to_le_bytes()); // z
        buf[18..22].copy_from_slice(&3.0f32.to_le_bytes()); // y
        buf[22..26].copy_from_slice(&4.0f32.to_le_bytes()); // deltaZ
        buf[30..34].copy_from_slice(&5.0f32.to_le_bytes()); // deltaY
        buf[38..42].copy_from_slice(&6.0f32.to_le_bytes()); // x
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.spawn_id, 0x1234);
        assert_eq!(p.delta_x, 1.0);
        assert_eq!(p.z, 2.0);
        assert_eq!(p.y, 3.0);
        assert_eq!(p.delta_z, 4.0);
        assert_eq!(p.delta_y, 5.0);
        assert_eq!(p.x, 6.0);
    }

    #[test]
    fn extracts_bitfields_at_heading_word() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // animation = -3 (0x3FD in 10-bit two's complement),
        // heading = 0xABC, padding = 0.
        let bf: u32 = (0x3FD as u32) | (0xABCu32 << 10);
        buf[26..30].copy_from_slice(&bf.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.animation, -3);
        assert_eq!(p.heading, 0xABC);
    }

    #[test]
    fn extracts_delta_heading() {
        let mut buf = [0u8; PAYLOAD_LEN];
        // deltaHeading = -1 (0x3FF in 10-bit).
        let bf: u32 = 0x3FF;
        buf[34..38].copy_from_slice(&bf.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.delta_heading, -1);
    }

    #[test]
    fn extracts_pitch_full_12_bits() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let bf: u32 = 0xFFF;
        buf[6..10].copy_from_slice(&bf.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.pitch, 0xFFF);
    }
}
