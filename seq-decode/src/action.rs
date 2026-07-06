//! Parser for `OP_Action` — payload `actionStruct` (64 bytes).
//! Daemon reads target, source, spell, level, type. The two-handler
//! dispatch in the C++ daemon also accepts `actionAltStruct` (88
//! bytes) — see `action_alt.rs`.

use seq_eqstructs_live::actionStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<actionStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub target: u16,
    pub source: u16,
    pub spell: u16,   // unsigned: modern spell IDs exceed 32767
    pub level: u8,
    pub kind: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_action(bytes: &[u8]) -> Result<Action, ActionError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(ActionError::BadLength(bytes.len()));
    }
    let raw: actionStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const actionStruct) };
    Ok(Action {
        target: unsafe { std::ptr::addr_of!(raw.target).read_unaligned() },
        source: unsafe { std::ptr::addr_of!(raw.source).read_unaligned() },
        spell:  unsafe { std::ptr::addr_of!(raw.spell).read_unaligned() },
        level:  unsafe { std::ptr::addr_of!(raw.level).read_unaligned() },
        kind:   unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_action(&[0; 63]).is_err());
        assert!(parse_action(&[0; 65]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..2].copy_from_slice(&100u16.to_le_bytes());
        buf[2..4].copy_from_slice(&200u16.to_le_bytes());
        buf[4..6].copy_from_slice(&40000u16.to_le_bytes());  // > 32767
        buf[12] = 65;
        buf[56] = 0xe7;
        let a = parse_action(&buf).unwrap();
        assert_eq!(a.target, 100);
        assert_eq!(a.source, 200);
        assert_eq!(a.spell, 40000);
        assert_eq!(a.level, 65);
        assert_eq!(a.kind, 0xe7);
    }
}
