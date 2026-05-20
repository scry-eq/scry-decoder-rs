//! Parser for `OP_FormattedMessage` — variable-length payload. The
//! fixed header (13 bytes) carries `messageFormat` and `messageColor`;
//! the daemon feeds the trailing `messages` blob to `EQStr::formatMessage`
//! which interpolates a sequence of {u32 len, len bytes} substitutions
//! into the format string. This parser surfaces only the header — the
//! daemon already has a working formatMessage and can slice the
//! `messages` pointer off the raw payload at `data + 13`.

use seq_eqstructs_live::formattedMessageStruct;
use thiserror::Error;

pub const HEADER_LEN: usize = std::mem::offset_of!(formattedMessageStruct, messages);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormattedMessage {
    pub message_format: u32,
    pub message_color: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormattedMessageError {
    #[error("expected at least {HEADER_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_formatted_message(
    bytes: &[u8],
) -> Result<FormattedMessage, FormattedMessageError> {
    if bytes.len() < HEADER_LEN {
        return Err(FormattedMessageError::BadLength(bytes.len()));
    }
    let raw: formattedMessageStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const formattedMessageStruct) };
    Ok(FormattedMessage {
        message_format: unsafe { std::ptr::addr_of!(raw.messageFormat).read_unaligned() },
        message_color:  unsafe { std::ptr::addr_of!(raw.messageColor).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_payload() {
        assert!(parse_formatted_message(&[0; 12]).is_err());
    }

    #[test]
    fn parses_header() {
        let mut buf = vec![0u8; HEADER_LEN];
        buf[5..9].copy_from_slice(&999u32.to_le_bytes()); // messageFormat
        buf[9..13].copy_from_slice(&0x1au32.to_le_bytes()); // messageColor
        let m = parse_formatted_message(&buf).unwrap();
        assert_eq!(m.message_format, 999);
        assert_eq!(m.message_color, 0x1a);
    }

    #[test]
    fn ignores_trailing_messages_blob() {
        let mut buf = vec![0u8; HEADER_LEN + 32];
        buf[5..9].copy_from_slice(&1u32.to_le_bytes());
        buf[9..13].copy_from_slice(&2u32.to_le_bytes());
        let m = parse_formatted_message(&buf).unwrap();
        assert_eq!(m.message_format, 1);
        assert_eq!(m.message_color, 2);
    }
}
