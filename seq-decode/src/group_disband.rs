//! Parser for `OP_GroupDisband` / `OP_GroupDisband2` —
//! payload `groupDisbandStruct`, 152 bytes. Daemon reads
//! yourname + membername (both 64-byte NUL-padded char arrays).

use crate::eqstructs::groupDisbandStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<groupDisbandStruct>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupDisband {
    pub yourname: String,
    pub membername: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupDisbandError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_group_disband(bytes: &[u8]) -> Result<GroupDisband, GroupDisbandError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(GroupDisbandError::BadLength(bytes.len()));
    }
    let raw: groupDisbandStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const groupDisbandStruct) };
    let yourname_raw: [u8; 64] = unsafe { std::ptr::addr_of!(raw.yourname).read_unaligned() };
    let membername_raw: [u8; 64] = unsafe { std::ptr::addr_of!(raw.membername).read_unaligned() };
    Ok(GroupDisband {
        yourname: crate::cstr_field(&yourname_raw),
        membername: crate::cstr_field(&membername_raw),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_group_disband(&[0; 151]).is_err());
        assert!(parse_group_disband(&[0; 153]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..3].copy_from_slice(b"Bob");
        buf[64..68].copy_from_slice(b"Sam!");
        let g = parse_group_disband(&buf).unwrap();
        assert_eq!(g.yourname, "Bob");
        assert_eq!(g.membername, "Sam!");
    }
}
