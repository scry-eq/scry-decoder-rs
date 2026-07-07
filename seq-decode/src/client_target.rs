//! Parser for `OP_TargetMouse` — payload `clientTargetStruct`,
//! 4 bytes (single u32 newTarget). Trivial twin of
//! `parse_delete_spawn`; kept as its own opcode-named module for
//! consistency with the rest of the Stage A+ batch.

use crate::eqstructs::clientTargetStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<clientTargetStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientTarget {
    pub new_target: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientTargetError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_client_target(bytes: &[u8]) -> Result<ClientTarget, ClientTargetError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ClientTargetError::BadLength(bytes.len()));
    }
    let raw: clientTargetStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const clientTargetStruct) };
    Ok(ClientTarget {
        new_target: unsafe { std::ptr::addr_of!(raw.newTarget).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_client_target(&[0; 3]).is_err());
        assert!(parse_client_target(&[0; 5]).is_err());
    }

    #[test]
    fn parses_target_id() {
        let buf = 0xDEADBEEFu32.to_le_bytes();
        let t = parse_client_target(&buf).unwrap();
        assert_eq!(t.new_target, 0xDEADBEEF);
    }
}
