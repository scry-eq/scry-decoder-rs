//! Parser for `OP_Action2` — payload `action2Struct`, 48 bytes.
//! Damage-and-melee event packet (resolved 2026-04-25 from a combat
//! capture). The daemon's combat router uses target, source, damage,
//! spell, and type.

use crate::eqstructs::action2Struct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<action2Struct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action2 {
    pub target: u16,
    pub source: u16,
    pub damage: i32,
    pub spell: i32,
    pub kind: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Action2Error {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_action2(bytes: &[u8]) -> Result<Action2, Action2Error> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(Action2Error::BadLength(bytes.len()));
    }
    let raw: action2Struct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const action2Struct) };
    Ok(Action2 {
        target: unsafe { std::ptr::addr_of!(raw.target).read_unaligned() },
        source: unsafe { std::ptr::addr_of!(raw.source).read_unaligned() },
        damage: unsafe { std::ptr::addr_of!(raw.damage).read_unaligned() },
        spell:  unsafe { std::ptr::addr_of!(raw.spell).read_unaligned() },
        kind:   unsafe { std::ptr::addr_of!(raw.type_).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_action2(&[0; 47]).is_err());
        assert!(parse_action2(&[0; 49]).is_err());
    }

    #[test]
    fn parses_melee_hit() {
        let mut buf = [0u8; 48];
        buf[0..2].copy_from_slice(&100u16.to_le_bytes());     // target
        buf[2..4].copy_from_slice(&200u16.to_le_bytes());     // source
        buf[8..12].copy_from_slice(&42i32.to_le_bytes());     // damage
        buf[20..24].copy_from_slice(&(-1i32).to_le_bytes());  // spell (-1 = melee)
        buf[40] = 7;                                            // type (kick?)
        let a = parse_action2(&buf).unwrap();
        assert_eq!(a.target, 100);
        assert_eq!(a.source, 200);
        assert_eq!(a.damage, 42);
        assert_eq!(a.spell, -1);
        assert_eq!(a.kind, 7);
    }
}
