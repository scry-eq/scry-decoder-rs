//! Parser for `OP_Action` (alt 88-byte payload `actionAltStruct`).
//! The wire field set is identical to `actionStruct` — only the
//! trailing placeholder grew. Same surfaced fields as `action.rs`.

use crate::eqstructs::actionAltStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<actionAltStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAlt {
    pub target: u16,
    pub source: u16,
    pub spell: u16, // unsigned: modern spell IDs exceed 32767
    pub level: u8,
    pub kind: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionAltError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_action_alt(bytes: &[u8]) -> Result<ActionAlt, ActionAltError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ActionAltError::BadLength(bytes.len()));
    }
    let raw: actionAltStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const actionAltStruct) };
    Ok(ActionAlt {
        target: unsafe { std::ptr::addr_of!(raw.target).read_unaligned() },
        source: unsafe { std::ptr::addr_of!(raw.source).read_unaligned() },
        spell: unsafe { std::ptr::addr_of!(raw.spell).read_unaligned() },
        level: unsafe { std::ptr::addr_of!(raw.level).read_unaligned() },
        kind: unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_action_alt(&[0; 87]).is_err());
        assert!(parse_action_alt(&[0; 89]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&100u16.to_le_bytes());
        buf[2..4].copy_from_slice(&200u16.to_le_bytes());
        buf[4..6].copy_from_slice(&7i16.to_le_bytes());
        buf[12] = 50;
        buf[56] = 0xe7;
        let a = parse_action_alt(&buf).unwrap();
        assert_eq!(a.target, 100);
        assert_eq!(a.source, 200);
        assert_eq!(a.spell, 7);
        assert_eq!(a.level, 50);
        assert_eq!(a.kind, 0xe7);
    }
}
