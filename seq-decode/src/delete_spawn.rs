//! Parser for `OP_DeleteSpawn` — payload `deleteSpawnStruct`, 4 bytes.
//!
//! Trivially a single little-endian `u32 spawnId`. The bindgen mirror
//! is overkill for the field count but keeps the layout tied to
//! `everquest.h` in case the wire ever grows fields (a `reason` byte,
//! e.g.) — bindgen would catch the size change at build time via the
//! `delete_spawn_struct_layout` test in `seq-eqstructs-live`.

use seq_eqstructs_live::deleteSpawnStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<deleteSpawnStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSpawn {
    pub spawn_id: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeleteSpawnError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_delete_spawn(bytes: &[u8]) -> Result<DeleteSpawn, DeleteSpawnError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(DeleteSpawnError::BadLength(bytes.len()));
    }
    // SAFETY: deleteSpawnStruct is #[repr(C)] POD with one u32 field;
    // length checked above.
    let raw: deleteSpawnStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const deleteSpawnStruct) };
    Ok(DeleteSpawn {
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(parse_delete_spawn(&[]), Err(DeleteSpawnError::BadLength(0)));
        assert_eq!(parse_delete_spawn(&[0; 3]), Err(DeleteSpawnError::BadLength(3)));
        assert_eq!(parse_delete_spawn(&[0; 5]), Err(DeleteSpawnError::BadLength(5)));
    }

    #[test]
    fn all_zero_payload() {
        assert_eq!(parse_delete_spawn(&[0; 4]).unwrap(), DeleteSpawn { spawn_id: 0 });
    }

    #[test]
    fn little_endian_u32() {
        // 0xDEADBEEF in LE bytes.
        let bytes = [0xEF, 0xBE, 0xAD, 0xDE];
        assert_eq!(parse_delete_spawn(&bytes).unwrap().spawn_id, 0xDEADBEEF);
    }

    #[test]
    fn max_u32() {
        let bytes = [0xFF; 4];
        assert_eq!(parse_delete_spawn(&bytes).unwrap().spawn_id, u32::MAX);
    }
}
