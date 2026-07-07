//! Parser for `OP_SpawnRename` — payload `spawnRenameStruct`,
//! 195 bytes (3× 64-byte char arrays + 3 placeholder bytes). The
//! `old_name_again` field is identical to `old_name` in observed
//! payloads; both are preserved for parity.

use crate::eqstructs::spawnRenameStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnRenameStruct>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRename {
    pub old_name: String,
    pub old_name_again: String,
    pub new_name: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnRenameError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_spawn_rename(bytes: &[u8]) -> Result<SpawnRename, SpawnRenameError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SpawnRenameError::BadLength(bytes.len()));
    }
    Ok(SpawnRename {
        old_name:       crate::cstr_field(&bytes[0..64]),
        old_name_again: crate::cstr_field(&bytes[64..128]),
        new_name:       crate::cstr_field(&bytes[128..192]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_spawn_rename(&[0; 194]).is_err());
        assert!(parse_spawn_rename(&[0; 196]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 195];
        buf[..6].copy_from_slice(b"orcLvl");
        buf[64..70].copy_from_slice(b"orcLvl");
        buf[128..136].copy_from_slice(b"a goblin");
        let r = parse_spawn_rename(&buf).unwrap();
        assert_eq!(r.old_name, "orcLvl");
        assert_eq!(r.new_name, "a goblin");
    }
}
