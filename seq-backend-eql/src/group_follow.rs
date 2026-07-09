//! Parser for `OP_GroupFollow` — modern wire format puts the joining
//! member's name as a NUL-terminated string at offset 0 (max 64
//! bytes). The legacy `groupFollowStruct` layout (with invitee at
//! offset 64) doesn't match modern packets, so this parser reads the
//! raw bytes rather than the struct.

use thiserror::Error;

pub const NAME_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupFollow {
    /// Member name, decoded from the first NAME_LEN bytes of the
    /// payload, truncated at the first NUL.
    pub name: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupFollowError {
    #[error("expected at least {NAME_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_group_follow(bytes: &[u8]) -> Result<GroupFollow, GroupFollowError> {
    if bytes.len() < NAME_LEN {
        return Err(GroupFollowError::BadLength(bytes.len()));
    }
    Ok(GroupFollow { name: crate::cstr_field(&bytes[..NAME_LEN]) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_payload() {
        assert!(parse_group_follow(&[0; 63]).is_err());
    }

    #[test]
    fn parses_name() {
        let mut buf = [0u8; 68];
        buf[..3].copy_from_slice(b"Joe");
        let g = parse_group_follow(&buf).unwrap();
        assert_eq!(g.name, "Joe");
    }
}
