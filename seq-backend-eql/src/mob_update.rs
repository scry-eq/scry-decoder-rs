//! Parser for `OP_MobUpdate` — payload `spawnPositionUpdate`, 18 bytes.
//!
//! Layout lives on `crate::eqstructs::spawnPositionUpdate`; its accessors mask
//! but don't sign-extend, so the 19-bit coords go through `sign_extend` and
//! the `>> 3` fixed-point conversion here. Fields are named in the MAP frame.

use crate::eqstructs::{sign_extend, spawnPositionUpdate};
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnPositionUpdate>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MobUpdate {
    pub spawn_id: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// 12-bit facing on a **4096**-step circle (bits 131..142).
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

    // Packed coords are y@bit0 | z@19 | 7-bit gap | x@45, wire frame; the
    // transpose to map frame happens here.
    let raw: spawnPositionUpdate =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const spawnPositionUpdate) };
    let spawn_id = unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() } as u16;
    let coord = |v: u64| sign_extend(v as u32, 19) >> 3;
    let y = coord(raw.y());
    let z = coord(raw.z());
    let x = coord(raw.x());
    let heading = raw.heading() as u16;

    Ok(MobUpdate {
        spawn_id,
        x,
        y,
        z,
        heading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(parse_mob_update(&[]), Err(ParseError::BadLength(0)));
        assert_eq!(parse_mob_update(&[0; 17]), Err(ParseError::BadLength(17)));
        assert_eq!(parse_mob_update(&[0; 19]), Err(ParseError::BadLength(19)));
        // the pre-08/25 size is now rejected
        assert_eq!(parse_mob_update(&[0; 14]), Err(ParseError::BadLength(14)));
    }

    #[test]
    fn all_zero_payload() {
        assert_eq!(
            parse_mob_update(&[0; 18]).unwrap(),
            MobUpdate {
                spawn_id: 0,
                x: 0,
                y: 0,
                z: 0,
                heading: 0
            },
        );
    }

    #[test]
    fn sign_extension_negative_y() {
        // 0x4_0000 is the 19-bit signed minimum, -262144 before the >> 3.
        let mut bytes = [0u8; 18];
        let bf: u64 = 0x4_0000u64;
        bytes[8..16].copy_from_slice(&bf.to_le_bytes());
        let m = parse_mob_update(&bytes).unwrap();
        assert_eq!(m.y, -32768);
        assert_eq!(m.x, 0);
        assert_eq!(m.z, 0);
    }

    // Distinct value per axis, so a re-shifted binding or a transpose fails
    // loudly; `<< 0` is the first field's bit POSITION, so clippy is silenced.
    #[allow(clippy::identity_op)]
    #[test]
    fn each_coordinate_reads_from_its_own_field() {
        let mut bytes = [0u8; 18];
        let bf: u64 = (800u64 << 0) | (16u64 << 19) | (2400u64 << 45);
        bytes[8..16].copy_from_slice(&bf.to_le_bytes());
        let m = parse_mob_update(&bytes).unwrap();
        assert_eq!(m.y, 100); // 800 >> 3  — bits 0..18
        assert_eq!(m.z, 2); //  16 >> 3  — bits 19..37
        assert_eq!(m.x, 300); // 2400 >> 3 — bits 45..63
    }

    // Layout pin: neighbours are set, so the old bit-132 read fails here.
    #[test]
    fn heading_is_twelve_bits_at_bit_131() {
        let mut bytes = [0u8; PAYLOAD_LEN];
        let quarter = 4096u16 / 4;
        bytes[16..18].copy_from_slice(&((quarter << 3) | 0x8007).to_le_bytes());
        assert_eq!(parse_mob_update(&bytes).unwrap().heading, quarter);
    }

    // A real broadcast; the 09/01 rotation left the coordinate bytes alone.
    #[test]
    fn decodes_a_captured_update() {
        let bytes: [u8; PAYLOAD_LEN] = [
            0x0C, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x33, 0xF3, 0x07, 0x01, 0x00, 0xA0,
            0x6B, 0xFF, 0xC0, 0x7F,
        ];
        let m = parse_mob_update(&bytes).unwrap();
        assert_eq!(m.spawn_id, 25612);
        assert_eq!((m.x, m.y, m.z), (-149, -410, 4));
        assert_eq!(m.heading, 4088);
    }
}
