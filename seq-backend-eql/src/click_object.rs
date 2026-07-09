//! Parser for `OP_ClickObject` — payload `remDropStruct`, 12 bytes.
//! Fires when a player picks up a ground item; the daemon uses
//! `dropId` (the ground-item slot) and `spawnId` (the picker).

use crate::eqstructs::remDropStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<remDropStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClickObject {
    pub drop_id: u16,
    pub spawn_id: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClickObjectError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_click_object(bytes: &[u8]) -> Result<ClickObject, ClickObjectError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ClickObjectError::BadLength(bytes.len()));
    }
    let raw: remDropStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const remDropStruct) };
    Ok(ClickObject {
        drop_id:  unsafe { std::ptr::addr_of!(raw.dropId).read_unaligned() },
        spawn_id: unsafe { std::ptr::addr_of!(raw.spawnId).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_click_object(&[0; 11]).is_err());
        assert!(parse_click_object(&[0; 13]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; 12];
        buf[0..2].copy_from_slice(&0xCAFEu16.to_le_bytes());
        buf[4..6].copy_from_slice(&0xBEEFu16.to_le_bytes());
        let c = parse_click_object(&buf).unwrap();
        assert_eq!(c.drop_id, 0xCAFE);
        assert_eq!(c.spawn_id, 0xBEEF);
    }
}
