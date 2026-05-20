//! Parser for `OP_RemoveSpawn` — payload `removeSpawnStruct`, 5 bytes.
//! `removeSpawn` is 0 when the spawn left your update radius.

use seq_eqstructs_live::removeSpawnStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<removeSpawnStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveSpawn {
    pub spawn_id: u32,
    pub remove_spawn: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RemoveSpawnError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_remove_spawn(bytes: &[u8]) -> Result<RemoveSpawn, RemoveSpawnError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(RemoveSpawnError::BadLength(bytes.len()));
    }
    let raw: removeSpawnStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const removeSpawnStruct) };
    Ok(RemoveSpawn {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
        remove_spawn: unsafe { std::ptr::addr_of!(raw.removeSpawn).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_remove_spawn(&[0; 4]).is_err());
        assert!(parse_remove_spawn(&[0; 6]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 5];
        buf[0..4].copy_from_slice(&12345u32.to_le_bytes());
        buf[4] = 1;
        let r = parse_remove_spawn(&buf).unwrap();
        assert_eq!(r.spawn_id, 12345);
        assert_eq!(r.remove_spawn, 1);
    }
}
