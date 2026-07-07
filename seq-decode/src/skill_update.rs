//! Parser for `OP_SkillUpdate` — payload `skillIncStruct`, 12 bytes.
//! Fires once per skill-up; `value` is the new absolute skill level
//! (signed only because raw negative values appeared during early
//! probes — practically a u8 0..N).

use crate::eqstructs::skillIncStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<skillIncStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillUpdate {
    pub skill_id: u32,
    pub value: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SkillUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_skill_update(bytes: &[u8]) -> Result<SkillUpdate, SkillUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(SkillUpdateError::BadLength(bytes.len()));
    }
    let raw: skillIncStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const skillIncStruct) };
    Ok(SkillUpdate {
        skill_id: unsafe { std::ptr::addr_of!(raw.skillId).read_unaligned() },
        value:    unsafe { std::ptr::addr_of!(raw.value).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_skill_update(&[0; 11]).is_err());
        assert!(parse_skill_update(&[0; 13]).is_err());
    }

    #[test]
    fn parses_h2h_skill_up() {
        // From 2026-05-01 confirmation log: skillId=30 (H2H), value=12.
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&30u32.to_le_bytes());
        buf[4..8].copy_from_slice(&12i32.to_le_bytes());
        let s = parse_skill_update(&buf).unwrap();
        assert_eq!(s.skill_id, 30);
        assert_eq!(s.value, 12);
    }
}
