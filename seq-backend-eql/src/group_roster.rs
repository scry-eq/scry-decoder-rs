//! Parser for `OP_GroupUpdate` (variable length): the full, authoritative group
//! roster, resent at zone-in (the group analog of the buff list). Layout:
//! `u32 groupId@0 | u32 count@4 | leader name@8`, then `count` 44-byte member
//! records from offset 18 (each a NUL-terminated name, possibly with a leading
//! NUL from the prior record's trailing index). `count < 2` is the solo /
//! not-grouped state. The consumer clears and repopulates. Field map from the
//! legends `GroupMgr` full-roster decoder.

use thiserror::Error;

const REC_OFF: usize = 18;
const REC_STRIDE: usize = 44;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRoster {
    pub group_id: u32,
    /// All member names in wire order (may include the local player); empty
    /// when solo (`count < 2`).
    pub members: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupRosterError {
    #[error("payload too short: {0} bytes")]
    Short(usize),
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse_group_roster(bytes: &[u8]) -> Result<GroupRoster, GroupRosterError> {
    if bytes.len() < 12 {
        return Err(GroupRosterError::Short(bytes.len()));
    }

    let group_id = rd_u32(bytes, 0);
    let count = rd_u32(bytes, 4) as usize;

    // Solo = not in a group.
    if count < 2 {
        return Ok(GroupRoster { group_id, members: vec![] });
    }

    let mut members = Vec::with_capacity(count);
    for r in 0..count {
        let mut off = REC_OFF + r * REC_STRIDE;
        if off >= bytes.len() {
            break;
        }
        while off < bytes.len() && bytes[off] == 0 {
            off += 1;
        }
        let start = off;
        while off < bytes.len() && bytes[off] != 0 {
            off += 1;
        }
        if off > start {
            members.push(String::from_utf8_lossy(&bytes[start..off]).into_owned());
        }
    }

    Ok(GroupRoster { group_id, members })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_is_empty() {
        let mut b = [0u8; 20];
        b[4..8].copy_from_slice(&1u32.to_le_bytes()); // count = 1
        assert_eq!(parse_group_roster(&b).unwrap().members, Vec::<String>::new());
    }

    #[test]
    fn reads_two_members_at_stride_44() {
        let mut b = vec![0u8; REC_OFF + 2 * REC_STRIDE];
        b[4..8].copy_from_slice(&2u32.to_le_bytes()); // count = 2
        b[REC_OFF..REC_OFF + 4].copy_from_slice(b"Hero");
        let s2 = REC_OFF + REC_STRIDE;
        b[s2..s2 + 5].copy_from_slice(b"Alice");
        assert_eq!(parse_group_roster(&b).unwrap().members, vec!["Hero", "Alice"]);
    }
}
