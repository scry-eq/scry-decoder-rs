//! Parser for `OP_GroupDisband` / `OP_GroupDisband2`: `yourname@0` and
//! `membername@64` (both 64-byte NUL-padded char arrays). Read by offset so
//! both the 152B stock and the 168B eql wire parse — `membername == yourname`
//! means the whole group disbanded, otherwise that peer left.

use thiserror::Error;

const YOURNAME_OFF: usize = 0;
const MEMBERNAME_OFF: usize = 64;
const NAME_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupDisband {
    pub yourname: String,
    pub membername: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupDisbandError {
    #[error("expected at least {0} bytes, got {1}")]
    BadLength(usize, usize),
}

pub fn parse_group_disband(bytes: &[u8]) -> Result<GroupDisband, GroupDisbandError> {
    let need = MEMBERNAME_OFF + NAME_LEN; // 128
    if bytes.len() < need {
        return Err(GroupDisbandError::BadLength(need, bytes.len()));
    }
    Ok(GroupDisband {
        yourname: crate::cstr_field(&bytes[YOURNAME_OFF..YOURNAME_OFF + NAME_LEN]),
        membername: crate::cstr_field(&bytes[MEMBERNAME_OFF..MEMBERNAME_OFF + NAME_LEN]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_payload() {
        assert!(parse_group_disband(&[0; 127]).is_err());
    }

    #[test]
    fn parses_both_wire_sizes() {
        for len in [152usize, 168] {
            let mut buf = vec![0u8; len];
            buf[0..3].copy_from_slice(b"Bob");
            buf[64..68].copy_from_slice(b"Sam!");
            let g = parse_group_disband(&buf).unwrap();
            assert_eq!(g.yourname, "Bob");
            assert_eq!(g.membername, "Sam!");
        }
    }
}
