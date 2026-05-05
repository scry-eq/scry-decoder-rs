//! Parser for `OP_SpawnRename` — payload `spawnRenameStruct`,
//! 195 bytes (3× 64-byte char arrays + 3 placeholder bytes). The
//! `old_name_again` field is identical to `old_name` in observed
//! payloads; both are preserved for parity.

use seq_eqstructs::spawnRenameStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<spawnRenameStruct>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRename {
    pub old_name: [u8; 64],
    pub old_name_again: [u8; 64],
    pub new_name: [u8; 64],
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
    let mut out = SpawnRename {
        old_name: [0; 64], old_name_again: [0; 64], new_name: [0; 64],
    };
    out.old_name.copy_from_slice(&bytes[0..64]);
    out.old_name_again.copy_from_slice(&bytes[64..128]);
    out.new_name.copy_from_slice(&bytes[128..192]);
    Ok(out)
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
        assert_eq!(&r.old_name[..6], b"orcLvl");
        assert_eq!(&r.new_name[..8], b"a goblin");
    }
}
