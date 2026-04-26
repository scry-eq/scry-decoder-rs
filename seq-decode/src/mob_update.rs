//! Parser for `OP_MobUpdate` — payload `spawnPositionUpdate`, 14 bytes.
//!
//! C struct (under `#pragma pack(1)`, GCC, little-endian, LSB-first
//! bitfield order):
//!
//! ```text
//! offset 0..2   : int16_t spawnId
//! offset 2..4   : uint8_t unk1[2]                       (discarded)
//! offset 4..12  : int64_t y:19, z:19, u3:7, x:19
//! offset 12..14 : unsigned heading:12, signed unused2:4
//! ```
//!
//! Coordinates are 19-bit signed values stored in an int64_t bitfield
//! and then arithmetic-shifted right by 3 by the C handler — that's the
//! 1/8-unit fixed-point → integer game-world conversion shared with the
//! rest of `Spawn`. Heading is 12-bit unsigned (0..=4095), passed
//! through unchanged.

use thiserror::Error;

pub const PAYLOAD_LEN: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobUpdate {
    pub spawn_id: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub heading: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_mob_update(bytes: &[u8]) -> Result<MobUpdate, ParseError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ParseError::BadLength(bytes.len()));
    }

    let spawn_id = u16::from_le_bytes([bytes[0], bytes[1]]);

    let bf64 = u64::from_le_bytes([
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11],
    ]);
    // y:19 at bits 0..19, z:19 at 19..38, u3:7 at 38..45 (unused),
    // x:19 at 45..64. Mirror the C bitfield's signed extraction.
    let y = sign_extend_19((bf64 & 0x7_FFFF) as u32) >> 3;
    let z = sign_extend_19(((bf64 >> 19) & 0x7_FFFF) as u32) >> 3;
    let x = sign_extend_19(((bf64 >> 45) & 0x7_FFFF) as u32) >> 3;

    let bf16 = u16::from_le_bytes([bytes[12], bytes[13]]);
    let heading = bf16 & 0x0FFF;

    Ok(MobUpdate { spawn_id, x, y, z, heading })
}

#[inline]
fn sign_extend_19(v: u32) -> i32 {
    let m = v & 0x7_FFFF;
    if m & 0x4_0000 != 0 {
        (m | 0xFFF8_0000) as i32
    } else {
        m as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(parse_mob_update(&[]), Err(ParseError::BadLength(0)));
        assert_eq!(parse_mob_update(&[0; 13]), Err(ParseError::BadLength(13)));
        assert_eq!(parse_mob_update(&[0; 15]), Err(ParseError::BadLength(15)));
    }

    #[test]
    fn all_zero_payload() {
        assert_eq!(
            parse_mob_update(&[0; 14]).unwrap(),
            MobUpdate { spawn_id: 0, x: 0, y: 0, z: 0, heading: 0 },
        );
    }

    #[test]
    fn sign_extend_boundary() {
        assert_eq!(sign_extend_19(0), 0);
        assert_eq!(sign_extend_19(1), 1);
        assert_eq!(sign_extend_19(0x3_FFFF), 0x3_FFFF);
        assert_eq!(sign_extend_19(0x4_0000), -262_144);
        assert_eq!(sign_extend_19(0x7_FFFF), -1);
    }
}
