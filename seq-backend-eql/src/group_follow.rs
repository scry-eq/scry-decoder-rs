//! Parser for `OP_GroupFollow` (168B): one member joined the group. The
//! joining member's name is at offset 64 (`groupFollowStruct.invitee`; the
//! leading 64 bytes are zero), with the level at 132. The old 68B/offset-0
//! layout no longer matches the wire — verified against a live group capture
//! and the legends `groupFollowStruct`.

use thiserror::Error;

const INVITEE_OFF: usize = 64;
const NAME_LEN: usize = 64;
const LEVEL_OFF: usize = 132;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupFollow {
    /// Joining member's name (`invitee@64`, NUL-terminated).
    pub name: String,
    /// Joining member's level (0 if the payload is short).
    pub level: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupFollowError {
    #[error("expected at least {0} bytes, got {1}")]
    BadLength(usize, usize),
}

pub fn parse_group_follow(bytes: &[u8]) -> Result<GroupFollow, GroupFollowError> {
    let need = INVITEE_OFF + NAME_LEN;
    if bytes.len() < need {
        return Err(GroupFollowError::BadLength(need, bytes.len()));
    }

    let name = crate::cstr_field(&bytes[INVITEE_OFF..INVITEE_OFF + NAME_LEN]);
    let level = bytes
        .get(LEVEL_OFF..LEVEL_OFF + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or(0);

    Ok(GroupFollow { name, level })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_payload() {
        assert!(parse_group_follow(&[0; 127]).is_err());
    }

    #[test]
    fn parses_name_at_offset_64_and_level() {
        let mut buf = [0u8; 168];
        buf[64..67].copy_from_slice(b"Joe");
        buf[132..136].copy_from_slice(&57u32.to_le_bytes());
        let g = parse_group_follow(&buf).unwrap();
        assert_eq!(g.name, "Joe");
        assert_eq!(g.level, 57);
    }
}
