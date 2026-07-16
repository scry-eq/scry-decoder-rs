//! Parser for eql's `OP_GroundSpawn` (id 0x7e02, 2026-07-14 rotation).
//!
//! The 07/14 patch moved the ground-object broadcast to 0x7e02 (the old 0x6360
//! 32B placement record is superseded). 0x7e02 fires S>C to spawn the zone's
//! GROUND OBJECTS: static interactable placeables (forge/tradeskill containers,
//! decorative racks, rocks) AND pickup-able ground-loot items (the earring) —
//! NOT personal combat loot, which is instanced in EQL and never on the ground.
//! (0x6360 only fired on a manual drop.) The handler is DIR_Server, so the C>S
//! mouse-drop request on the same id is filtered out.
//!
//! Variable-length; the actorDef model name is embedded, so the fixed fields sit
//! at offsets relative to its NUL terminator:
//! ```text
//!   /*@0*/    u32  dropId
//!   /*@4*/    char actorDef[] NUL-terminated   e.g. "IT63_ACTORDEF"
//!   /*nul+1*/ u32  fieldA        (17 / 30 / 39 — unmapped)
//!   /*nul+5*/ u32  itemId        (real id or 0xffffffff)
//!   /*nul+9*/ u32  fieldC        (unmapped)
//!   /*nul+13*/u32  0
//!   /*nul+17*/u32  0
//!   /*nul+21*/u32  0
//!   /*nul+25*/f32  1.0           (const)
//!   /*nul+29*/f32  x
//!   /*nul+33*/f32  y
//!   /*nul+37*/f32  z             (matches the OP_MobUpdate Z range for the zone)
//!   /*nul+41*/u32  tail
//! ```
//! `id_file` now carries the actorDef model string (e.g. `IT63_ACTORDEF`);
//! resolving it to the real item name ("Pearl Earring") means correlating the
//! itemId with the preceding ~1KB item-def (OP_ItemPacket 0x4d7e) — a future
//! refinement. Position derived from the earring drop (matches the old 0x6360
//! reading 598/569/-685) + 47 combat-loot records across the fight capture.

use thiserror::Error;

pub const ID_FILE_LEN: usize = 30;
/// x/y/z start here, relative to the actorDef NUL terminator.
const POS_AFTER_NUL: usize = 29;

#[derive(Debug, Clone)]
pub struct GroundSpawn {
    pub drop_id: u32,
    /// The actorDef model string (e.g. `IT63_ACTORDEF`); empty if unnamed.
    pub id_file: String,
    pub heading: f32,
    pub y: f32,
    pub x: f32,
    pub z: f32,
    /// Bytes consumed from the input.
    pub bytes_consumed: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroundSpawnError {
    #[error("payload truncated at {0}, need at least {1} more bytes")]
    Truncated(usize, usize),
    #[error("idFile not NUL-terminated within payload")]
    UnterminatedText,
}

fn read_u32_le(bytes: &[u8], at: usize) -> Result<u32, GroundSpawnError> {
    bytes
        .get(at..at + 4)
        .ok_or(GroundSpawnError::Truncated(at, 4))
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_f32_le(bytes: &[u8], at: usize) -> Result<f32, GroundSpawnError> {
    read_u32_le(bytes, at).map(f32::from_bits)
}

pub fn parse_ground_spawn(bytes: &[u8]) -> Result<GroundSpawn, GroundSpawnError> {
    // dropId + at least a 1-char NUL-terminated name.
    if bytes.len() < 6 {
        return Err(GroundSpawnError::Truncated(bytes.len(), 6));
    }
    let drop_id = read_u32_le(bytes, 0)?;
    // actorDef model name: NUL-terminated from @4.
    let name = &bytes[4..];
    let rel_nul = name
        .iter()
        .position(|&b| b == 0)
        .ok_or(GroundSpawnError::UnterminatedText)?;
    let id_file = String::from_utf8_lossy(&name[..rel_nul]).into_owned();
    let nul = 4 + rel_nul;
    let px = nul + POS_AFTER_NUL;
    let x = read_f32_le(bytes, px)?;
    let y = read_f32_le(bytes, px + 4)?;
    let z = read_f32_le(bytes, px + 8)?;
    Ok(GroundSpawn {
        drop_id,
        id_file,
        heading: 0.0, // no heading in the 0x7e02 record
        y,
        x,
        z,
        bytes_consumed: (px + 16) as u32, // x/y/z + trailing u32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 0x7e02 record: dropId, NUL-term name, 28 bytes of fixed fields,
    /// then x/y/z + a trailing u32.
    fn pkt(drop: u32, name: &str, x: f32, y: f32, z: f32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&drop.to_le_bytes());
        b.extend_from_slice(name.as_bytes());
        b.push(0);
        // fixed fields between the NUL and x: nul+1 .. nul+28 (POS_AFTER_NUL puts
        // x at nul+29, and the NUL byte itself is at nul).
        b.extend_from_slice(&[0u8; POS_AFTER_NUL - 1]);
        b.extend_from_slice(&x.to_le_bytes());
        b.extend_from_slice(&y.to_le_bytes());
        b.extend_from_slice(&z.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes()); // tail
        b
    }

    #[test]
    fn rejects_short_payload() {
        assert!(parse_ground_spawn(&[0; 5]).is_err());
    }

    #[test]
    fn rejects_unterminated_name() {
        let mut b = 1u32.to_le_bytes().to_vec();
        b.extend_from_slice(b"NONUL"); // no terminator
        assert!(matches!(
            parse_ground_spawn(&b),
            Err(GroundSpawnError::UnterminatedText)
        ));
    }

    #[test]
    fn parses_earring_drop() {
        // The contarget earring: dropId 4, IT63_ACTORDEF, pos 598/569/-685.
        let b = pkt(4, "IT63_ACTORDEF", 598.35, 569.14, -685.65);
        assert_eq!(b.len(), 62); // 4 + 13 + 1 + 29 + 12 + 4
        let g = parse_ground_spawn(&b).unwrap();
        assert_eq!(g.drop_id, 4);
        assert_eq!(g.id_file, "IT63_ACTORDEF");
        assert_eq!(g.x, 598.35);
        assert_eq!(g.y, 569.14);
        assert_eq!(g.z, -685.65);
        assert_eq!(g.bytes_consumed, 62);
    }

    #[test]
    fn parses_short_name_variant() {
        // IT2_ACTORDEF (12 chars) -> 61-byte record.
        let b = pkt(5, "IT2_ACTORDEF", -159.0, 39.0, 0.06);
        assert_eq!(b.len(), 61);
        let g = parse_ground_spawn(&b).unwrap();
        assert_eq!(g.id_file, "IT2_ACTORDEF");
        assert_eq!(g.x, -159.0);
    }
}
