//! Parser for eql's `OP_GroundSpawn` (id 0x6360, post-2026-07-14).
//!
//! A ground drop is a fixed **32-byte** placement record — it carries only the
//! object's id + world position, NOT the item name/stats (those arrive
//! separately as the ~1KB item-definition, OP_ItemPacket-style). Confirmed
//! drop-specific: it fires exactly once per dropped item and is absent from
//! walk / combat captures with no drops.
//!
//! Layout (little-endian):
//! ```text
//!   /*0000*/ u32  dropId
//!   /*0004*/ f32  x
//!   /*0008*/ f32  y
//!   /*0012*/ f32  z         (matches the OP_MobUpdate Z range for the zone)
//!   /*0016*/ f32  heading
//!   /*0020*/ 12 bytes pad
//! ```
//! `id_file` (the legacy actor/model string) is surfaced empty — the daemon's
//! `SpawnShell::newGroundItem` places the drop from the position; naming it
//! would require correlating the preceding item-def, a future refinement.

use thiserror::Error;

pub const ID_FILE_LEN: usize = 30;
const PAYLOAD_MIN: usize = 20; // dropId + 4 floats

#[derive(Debug, Clone)]
pub struct GroundSpawn {
    pub drop_id: u32,
    /// Empty on the 32B eql record (the model/name is not carried here).
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
    if bytes.len() < PAYLOAD_MIN {
        return Err(GroundSpawnError::Truncated(bytes.len(), PAYLOAD_MIN));
    }
    let drop_id = read_u32_le(bytes, 0)?;
    let x = read_f32_le(bytes, 4)?;
    let y = read_f32_le(bytes, 8)?;
    let z = read_f32_le(bytes, 12)?;
    let heading = read_f32_le(bytes, 16)?;
    Ok(GroundSpawn {
        drop_id,
        id_file: String::new(),
        heading,
        y,
        x,
        z,
        bytes_consumed: PAYLOAD_MIN as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_payload() {
        assert!(parse_ground_spawn(&[0; 19]).is_err());
    }

    #[test]
    fn parses_fields() {
        // dropId=42, x=601.0, y=570.0, z=-682.5, heading=204.0, + pad.
        let mut buf = Vec::new();
        buf.extend_from_slice(&42u32.to_le_bytes());
        buf.extend_from_slice(&601.0f32.to_le_bytes());
        buf.extend_from_slice(&570.0f32.to_le_bytes());
        buf.extend_from_slice(&(-682.5f32).to_le_bytes());
        buf.extend_from_slice(&204.0f32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 12]); // pad
        let g = parse_ground_spawn(&buf).unwrap();
        assert_eq!(g.drop_id, 42);
        assert_eq!(g.x, 601.0);
        assert_eq!(g.y, 570.0);
        assert_eq!(g.z, -682.5);
        assert_eq!(g.heading, 204.0);
        assert!(g.id_file.is_empty());
    }
}
