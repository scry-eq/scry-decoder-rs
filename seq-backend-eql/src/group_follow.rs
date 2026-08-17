//! Parser for `OP_GroupFollow`: one member joined the group. The joining
//! member's name is at offset 64 (`groupFollowStruct.invitee`; the leading 64
//! bytes are zero), with the level at 132. The old 68B/offset-0 layout no
//! longer matches the wire — verified against a live group capture and the
//! legends `groupFollowStruct`.
//!
//! THE SIZE IS THE GATE, and it is the whole struct rather than the last field
//! we happen to read. `groupFollowStruct` is a FIXED 168 bytes on the EQL wire
//! (stock is 152); the opcode carries `sizechecktype = "none"`, so nothing
//! upstream of here rejects a packet that is not one. Accepting the 128 bytes
//! that merely reach the level field let anything larger through, and a
//! non-group payload then decoded as a member: a name made of raw bytes and a
//! level like 3367636051 (0xc8ba0c53), which is what a `u32` read off the
//! wrong packet looks like. Requiring all 168 makes a wrong payload fail
//! closed instead of inventing a group member that never goes away.

use thiserror::Error;

const INVITEE_OFF: usize = 64;
const NAME_LEN: usize = 64;
const LEVEL_OFF: usize = 132;
/// `sizeof(groupFollowStruct)` on the EQL wire — fixed, per the legends header.
const STRUCT_LEN: usize = 168;

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
    if bytes.len() < STRUCT_LEN {
        return Err(GroupFollowError::BadLength(STRUCT_LEN, bytes.len()));
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

    /// The regression: a payload long enough to reach the level field but not a
    /// `groupFollowStruct`. It used to decode into a member with a garbage name
    /// and an absurd level, and nothing ever removed it.
    #[test]
    fn rejects_a_payload_that_only_reaches_the_level_field() {
        assert!(parse_group_follow(&[0xc8; STRUCT_LEN - 1]).is_err());
        assert!(parse_group_follow(&[0xc8; 128]).is_err());
    }

    #[test]
    fn parses_name_at_offset_64_and_level() {
        let mut buf = [0u8; STRUCT_LEN];
        buf[64..67].copy_from_slice(b"Joe");
        buf[132..136].copy_from_slice(&57u32.to_le_bytes());
        let g = parse_group_follow(&buf).unwrap();
        assert_eq!(g.name, "Joe");
        assert_eq!(g.level, 57);
    }
}
