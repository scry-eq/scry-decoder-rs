//! Parser for `OP_SimpleMessage` — fixed 12-byte payload. The daemon
//! reads `message_format` (looked up in the eqstr table) and
//! `message_color` (mapped to a MessageType + forwarded to the web
//! client). The trailing `unknown` u32 is consumed but unused.

use crate::eqstructs::simpleMessageStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<simpleMessageStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleMessage {
    pub message_format: u32,
    pub message_color: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SimpleMessageError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_simple_message(bytes: &[u8]) -> Result<SimpleMessage, SimpleMessageError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SimpleMessageError::BadLength(bytes.len()));
    }
    let raw: simpleMessageStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const simpleMessageStruct) };
    Ok(SimpleMessage {
        message_format: unsafe { std::ptr::addr_of!(raw.messageFormat).read_unaligned() },
        message_color:  unsafe { std::ptr::addr_of!(raw.messageColor).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_simple_message(&[0; 11]).is_err());
        assert!(parse_simple_message(&[0; 13]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&12345u32.to_le_bytes());
        buf[4..8].copy_from_slice(&0x12u32.to_le_bytes()); // CC_Default
        let m = parse_simple_message(&buf).unwrap();
        assert_eq!(m.message_format, 12345);
        assert_eq!(m.message_color, 0x12);
    }
}
