//! Parser for the 42-byte `playerSelfPosStruct` (`OP_ClientUpdate`, C>S — the
//! local player's own position report), in map-frame IEEE floats with no ×8
//! packing (unlike the S>C packed `playerSpawnPosStruct`).
//!
//! 09/01 layout from upstream 47a4992, unverified on our wire.
//!
//! ```text
//!   /*0000*/ u16  counter    changes every packet
//!   /*0002*/ u16  spawnId    the local player's spawn id (the PHANTOM TWIN's)
//!   /*0004*/ u16  unknown
//!   /*0006*/ u32  animation:10 | pad:22
//!   /*0010*/ f32  deltaY
//!   /*0014*/ f32  deltaX
//!   /*0018*/ u16  heading:11 | pad:5        2048-step circle
//!   /*0020*/ u16  deltaHeading:10 | pad:6
//!   /*0022*/ f32  x
//!   /*0026*/ f32  y
//!   /*0030*/ f32  deltaZ
//!   /*0034*/ f32  z
//!   /*0038*/ u32  flags:20 | pitch:11 | pad:1
//! ```
//!
//! `spawnId` @2 is the PHANTOM TWIN's id, so never adopt the player from it —
//! `self_track::SelfTracker` keeps it provisional until an `OP_ZoneEntry` name
//! match.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 42;

/// Longest body accepted; bytes past [`PAYLOAD_LEN`] are ignored.
pub const MAX_PAYLOAD_LEN: usize = 46;

/// Full circle in wire heading units (11-bit field → 2048 steps). INVERTED
/// like every other heading; calibrate the sense on a turn, not on a bearing.
pub const HEADING_UNITS: u16 = 2048;

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSelfPos {
    /// The local player's spawn id — the phantom twin's (see the module doc).
    pub spawn_id: u16,
    /// Per-packet counter at offset 0. Not in the FFI struct.
    pub counter: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    /// 11-bit unsigned compass value (0..2047, see [`HEADING_UNITS`]).
    pub heading: u16,
    pub delta_heading: i16,
    pub animation: i16,
    /// 11-bit up/down look angle on the same 2048-step circle.
    pub pitch: u16,
    /// The sparse low 20 bits of the trailing dword. Not in the FFI struct.
    pub flags: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlayerSelfPosError {
    #[error("expected {PAYLOAD_LEN}..={MAX_PAYLOAD_LEN} bytes, got {0}")]
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

/// Sign-extend the low `bits` of `v` into an `i16`.
fn sign_extend16(v: u16, bits: u32) -> i16 {
    let shift = 16 - bits;
    ((v << shift) as i16) >> shift
}

pub fn parse_player_self_pos(bytes: &[u8]) -> Result<PlayerSelfPos, PlayerSelfPosError> {
    if !(PAYLOAD_LEN..=MAX_PAYLOAD_LEN).contains(&bytes.len()) {
        return Err(PlayerSelfPosError::BadLength(bytes.len()));
    }

    // Deltas are wire floats now that upstream has located them; they were
    // surfaced as 0 while their home was unknown.
    let tail = read_u32_le(bytes, 38);
    Ok(PlayerSelfPos {
        spawn_id: read_u16_le(bytes, 2),
        counter: read_u16_le(bytes, 0),
        x: read_f32_le(bytes, 22),
        y: read_f32_le(bytes, 26),
        z: read_f32_le(bytes, 34),
        delta_x: read_f32_le(bytes, 14),
        delta_y: read_f32_le(bytes, 10),
        delta_z: read_f32_le(bytes, 30),
        heading: read_u16_le(bytes, 18) & 0x7FF,
        delta_heading: sign_extend16(read_u16_le(bytes, 20) & 0x3FF, 10),
        animation: sign_extend16((read_u32_le(bytes, 6) & 0x3FF) as u16, 10),
        pitch: ((tail >> 20) & 0x7FF) as u16,
        flags: tail & 0xF_FFFF,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_player_self_pos(&[0; 38]).is_err());
        assert!(parse_player_self_pos(&[0; 41]).is_err());
        assert!(parse_player_self_pos(&[0; 47]).is_err());
    }

    // The size has flip-flopped 42 -> 46 -> 42, so both lengths must parse.
    #[test]
    fn tolerates_a_trailing_tail_up_to_46_bytes() {
        for len in PAYLOAD_LEN..=MAX_PAYLOAD_LEN {
            let mut buf = vec![0xFFu8; len];
            buf[22..26].copy_from_slice(&12.5f32.to_le_bytes());
            assert_eq!(parse_player_self_pos(&buf).unwrap().x, 12.5, "len={len}");
        }
    }

    // Layout pin: every vacated offset carries a decoy, so a parser left on the
    // old layout reads the decoy instead of failing on a missing field.
    #[test]
    fn every_field_reads_from_its_own_offset() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&4321u16.to_le_bytes()); // counter
        buf[2..4].copy_from_slice(&25622u16.to_le_bytes()); // spawnId
        buf[6..10].copy_from_slice(&(0x3FFu32 - 6).to_le_bytes()); // animation -7
        buf[10..14].copy_from_slice(&(-1.25f32).to_le_bytes()); // deltaY
        buf[14..18].copy_from_slice(&2.5f32.to_le_bytes()); // deltaX
        buf[18..20].copy_from_slice(&(988u16 | (0x1F << 11)).to_le_bytes()); // heading
        buf[20..22].copy_from_slice(&(0x3FFu16 - 3).to_le_bytes()); // deltaHeading -4
        buf[22..26].copy_from_slice(&1207.625f32.to_le_bytes()); // x
        buf[26..30].copy_from_slice(&(-395.625f32).to_le_bytes()); // y
        buf[30..34].copy_from_slice(&0.5f32.to_le_bytes()); // deltaZ
        buf[34..38].copy_from_slice(&0.875f32.to_le_bytes()); // z
        buf[38..42].copy_from_slice(&(0xABCDEu32 | (300u32 << 20)).to_le_bytes());

        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.counter, 4321);
        assert_eq!(p.spawn_id, 25622);
        assert_eq!((p.x, p.y, p.z), (1207.625, -395.625, 0.875));
        assert_eq!((p.delta_x, p.delta_y, p.delta_z), (2.5, -1.25, 0.5));
        assert_eq!(p.heading, 988);
        assert_eq!(p.delta_heading, -4);
        assert_eq!(p.animation, -7);
        assert_eq!(p.pitch, 300);
        assert_eq!(p.flags, 0xABCDE);
    }

    // Neighbouring high bits are set so a sloppy mask is caught.
    #[test]
    fn decodes_the_facing_as_a_compass_value() {
        let mut buf = [0u8; PAYLOAD_LEN];
        let w = (HEADING_UNITS / 4) | (0x1Fu16 << 11);
        buf[18..20].copy_from_slice(&w.to_le_bytes());
        let p = parse_player_self_pos(&buf).unwrap();
        assert_eq!(p.heading, HEADING_UNITS / 4);
        assert!(p.heading < HEADING_UNITS);
    }

    #[test]
    fn zero_payload_is_origin() {
        let p = parse_player_self_pos(&[0u8; PAYLOAD_LEN]).unwrap();
        assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
        assert_eq!(p.heading, 0);
        assert_eq!(p.spawn_id, 0);
    }
}
