//! Parser for `OP_NpcMoveUpdate` — a variable-length 13..24 byte payload in the
//! legacy MSB-first `BitStream` packing (not the `#[repr(C, packed)]` bitfield
//! convention used elsewhere), shifted on the way out (coords `>> 3`, deltas
//! `>> 2`) so the fields drop straight into `updateSpawn`.
//!
//! ```text
//!   16 bits — spawnId (big-endian within the bit stream)
//!   32 bits — garbage / reserved
//!    6 bits — fieldSpecifier bitmask
//!   19 bits — y (signed, sign-magnitude — NOT two's complement)
//!   19 bits — x (signed)
//!   19 bits — z (signed)
//!   12 bits — path bearing toward the next waypoint, NOT a facing
//!   [optional, in this order if the corresponding mask bit is set]
//!     0x01  → 12 bits pitch (read but unused by daemon)
//!     0x02  → 10 bits deltaHeading (signed)
//!     0x04  → 10 bits velocity / animation (signed)
//!     0x08  → 13 bits deltaY (signed)
//!     0x10  → 13 bits deltaX (signed)
//!     0x20  → 13 bits deltaZ (signed)
//! ```
//!
//! 09/01 layout from upstream 47a4992, unverified on our wire: the 12-bit angle
//! is a path bearing, so [`NpcMoveUpdate::heading`] stays 0.

use thiserror::Error;

const MASK_PITCH: u8 = 0x01;
const MASK_DELTA_HEADING: u8 = 0x02;
const MASK_ANIMATION: u8 = 0x04;
const MASK_DELTA_Y: u8 = 0x08;
const MASK_DELTA_X: u8 = 0x10;
const MASK_DELTA_Z: u8 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcMoveUpdate {
    pub spawn_id: u16,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    /// This channel carries no facing — always 0 (see the module doc).
    pub heading: i16,
    /// The 12-bit angle: a bearing toward the next waypoint, not a facing.
    pub path_bearing: i16,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub delta_heading: i8,
    pub animation: i16,
    pub has_delta_x: bool,
    pub has_delta_y: bool,
    pub has_delta_z: bool,
    pub has_delta_heading: bool,
    pub has_animation: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NpcMoveUpdateError {
    #[error("expected 13..=24 bytes, got {0}")]
    BadLength(usize),
    #[error("bit stream exhausted after {0} bits (payload too short for fieldSpecifier)")]
    Truncated(usize),
    #[error("coordinate {0} outside the +-16000 game world")]
    OutOfRange(i16),
}

/// MSB-first bit reader matching the daemon's `BitStream`
/// implementation in `netstream.cpp`.
struct BitStream<'a> {
    data: &'a [u8],
    cur: usize,   // bit index of next bit to read
    total: usize, // bit length of the buffer
}

impl<'a> BitStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            cur: 0,
            total: data.len() * 8,
        }
    }

    /// Mirrors `BitStream::readUInt`: 0 on under-read, so a malformed packet
    /// degrades instead of crashing; callers check [`Self::cur`]/[`Self::total`].
    fn read_uint(&mut self, bit_count: usize) -> u32 {
        if self.cur + bit_count > self.total {
            return 0;
        }
        let mut byte_idx = self.cur >> 3;
        let mut out: u32 = 0;

        let lead_partial = if self.cur % 8 == 0 {
            0
        } else {
            8 - (self.cur % 8)
        };

        if lead_partial > bit_count {
            // All bits live in the partial lead byte.
            let raw = (self.data[byte_idx] as u32) & ((1u32 << lead_partial) - 1);
            self.cur += bit_count;
            return raw >> (lead_partial - bit_count);
        }

        let middle = (bit_count - lead_partial) / 8;
        let tail_partial = bit_count - lead_partial - middle * 8;

        if lead_partial > 0 {
            out |= (self.data[byte_idx] as u32) & ((1u32 << lead_partial) - 1);
            byte_idx += 1;
        }

        for _ in 0..middle {
            out = (out << 8) | (self.data[byte_idx] as u32);
            byte_idx += 1;
        }

        if tail_partial > 0 {
            out = (out << tail_partial) | ((self.data[byte_idx] as u32) >> (8 - tail_partial));
        }

        self.cur += bit_count;
        out
    }

    /// Mirrors `BitStream::readInt`: 1 sign bit + (bit_count - 1)
    /// magnitude bits (sign-magnitude, not two's complement).
    fn read_int(&mut self, bit_count: usize) -> i32 {
        let sign = self.read_uint(1);
        let mag = self.read_uint(bit_count - 1) as i32;
        if sign != 0 {
            -mag
        } else {
            mag
        }
    }
}

pub fn parse_npc_move_update(bytes: &[u8]) -> Result<NpcMoveUpdate, NpcMoveUpdateError> {
    if bytes.len() < 16 || bytes.len() > 24 {
        return Err(NpcMoveUpdateError::BadLength(bytes.len()));
    }
    let mut s = BitStream::new(bytes);

    let spawn_id = s.read_uint(16) as u16;
    // 08/25 widened this lead field 16 -> 32 bits; upstream agrees.
    let _garbage = s.read_uint(32);
    let field_specifier = s.read_uint(6) as u8;

    let y = (s.read_int(19) >> 3) as i16;
    let x = (s.read_int(19) >> 3) as i16;
    let z = (s.read_int(19) >> 3) as i16;
    let path_bearing = s.read_int(12) as i16;

    // Upstream's guard: a decode that slipped a bit lands far outside any zone.
    for axis in [x, y, z] {
        if axis.abs() > 16000 {
            return Err(NpcMoveUpdateError::OutOfRange(axis));
        }
    }

    let mut delta_x: i16 = 0;
    let mut delta_y: i16 = 0;
    let mut delta_z: i16 = 0;
    let mut delta_heading: i8 = 0;
    let mut animation: i16 = 0;

    if field_specifier & MASK_PITCH != 0 {
        let _pitch = s.read_int(12);
    }
    if field_specifier & MASK_DELTA_HEADING != 0 {
        delta_heading = (s.read_int(10) >> 2) as i8;
    }
    if field_specifier & MASK_ANIMATION != 0 {
        animation = (s.read_int(10) >> 2) as i16;
    }
    if field_specifier & MASK_DELTA_Y != 0 {
        delta_y = (s.read_int(13) >> 2) as i16;
    }
    if field_specifier & MASK_DELTA_X != 0 {
        delta_x = (s.read_int(13) >> 2) as i16;
    }
    if field_specifier & MASK_DELTA_Z != 0 {
        delta_z = (s.read_int(13) >> 2) as i16;
    }

    if s.cur > s.total {
        return Err(NpcMoveUpdateError::Truncated(s.cur));
    }

    Ok(NpcMoveUpdate {
        spawn_id,
        x,
        y,
        z,
        heading: 0,
        path_bearing,
        delta_x,
        delta_y,
        delta_z,
        delta_heading,
        animation,
        has_delta_x: field_specifier & MASK_DELTA_X != 0,
        has_delta_y: field_specifier & MASK_DELTA_Y != 0,
        has_delta_z: field_specifier & MASK_DELTA_Z != 0,
        has_delta_heading: field_specifier & MASK_DELTA_HEADING != 0,
        has_animation: field_specifier & MASK_ANIMATION != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_npc_move_update(&[0; 12]).is_err());
        assert!(parse_npc_move_update(&[0; 25]).is_err());
    }

    #[test]
    fn read_uint_byte_aligned() {
        // 0x12 0x34 → readUInt(16) should be 0x1234 (big-endian).
        let mut s = BitStream::new(&[0x12, 0x34]);
        assert_eq!(s.read_uint(16), 0x1234);
    }

    #[test]
    fn read_uint_sub_byte() {
        // 0xF0 → readUInt(4) should be 0x0F (top nibble first).
        let mut s = BitStream::new(&[0xF0]);
        assert_eq!(s.read_uint(4), 0xF);
        assert_eq!(s.read_uint(4), 0x0);
    }

    #[test]
    fn read_int_sign_magnitude() {
        // sign=1, mag=5 in 4 bits (0b1101) → readInt(4) = -5.
        let mut s = BitStream::new(&[0xD0]);
        assert_eq!(s.read_int(4), -5);
    }

    /// MSB-first writer mirroring [`BitStream`], for building layout fixtures.
    fn put_bits(buf: &mut [u8], at: usize, count: usize, value: u32) {
        for i in 0..count {
            let bit = (value >> (count - 1 - i)) & 1;
            let pos = at + i;
            buf[pos >> 3] |= (bit as u8) << (7 - (pos & 7));
        }
    }

    fn fixture(y: i32, x: i32, z: i32, bearing: u32) -> [u8; 16] {
        let mut buf = [0u8; 16];
        put_bits(&mut buf, 0, 16, 0x4321);
        let mut put_coord = |at: usize, v: i32| {
            put_bits(&mut buf, at, 1, u32::from(v < 0));
            put_bits(&mut buf, at + 1, 18, v.unsigned_abs());
        };
        put_coord(54, y);
        put_coord(73, x);
        put_coord(92, z);
        put_bits(&mut buf, 111, 12, bearing);
        buf
    }

    // Layout pin: the 12-bit angle lands in `path_bearing`, never `heading`.
    #[test]
    fn the_twelve_bit_angle_is_a_path_bearing_not_a_facing() {
        let r = parse_npc_move_update(&fixture(800, -2400, 16, 0x123)).unwrap();
        assert_eq!((r.x, r.y, r.z), (-300, 100, 2));
        assert_eq!(r.path_bearing, 0x123);
        assert_eq!(r.heading, 0);
    }

    #[test]
    fn rejects_a_coordinate_outside_the_game_world() {
        assert_eq!(
            parse_npc_move_update(&fixture(200_000, 0, 0, 0)),
            Err(NpcMoveUpdateError::OutOfRange(25_000)),
        );
    }

    #[test]
    fn parses_minimum_packet_no_optional_fields() {
        // 16+32+6+19+19+19+12 = 123 bits -> 16 bytes, the length floor:
        // under 16 the fixed block cannot fit.
        let mut buf = [0u8; 16];
        buf[0] = 0x43;
        buf[1] = 0x21;
        let r = parse_npc_move_update(&buf).unwrap();
        assert_eq!(r.spawn_id, 0x4321);
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.z, 0);
        assert_eq!(r.heading, 0);
        assert_eq!(r.path_bearing, 0);
        assert_eq!(r.delta_x, 0);
        assert!(!r.has_delta_x);
        assert!(!r.has_delta_y);
        assert!(!r.has_delta_z);
        assert!(!r.has_delta_heading);
        assert!(!r.has_animation);
    }
}
