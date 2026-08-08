//! Parser for `OP_GroupMemberList` — the modern group roster
//! broadcast. The slot-0 (leader) record has variable, capture-dependent width
//! (individually null-padded name + assist-name fields), so rather than a fixed
//! struct we scan forward for printable-ASCII name runs — every non-empty run
//! of EQ-name characters between nulls is a member name. The daemon dedups,
//! drops its own name, and diffs against the tracked roster (that needs the
//! Player + SpawnShell, so it stays C++).

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMemberList {
    pub group_id: u32,
    pub member_count: u32,
    /// Scanned member-name runs, in wire order (raw: may include the leader
    /// twice and the local player — the daemon dedups + self-filters).
    pub names: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupMemberListError {
    #[error("payload too short: {0} bytes")]
    Short(usize),
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

// EQ names are capitalized and >= 3 chars; guarding the start against 'A'..'Z'
// keeps stray printable bytes (e.g. 0x3c '<' from an adjacent i32 level field)
// from being mis-scanned as names.
fn is_name_start(b: u8) -> bool {
    b.is_ascii_uppercase()
}
fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub fn parse_group_member_list(bytes: &[u8]) -> Result<GroupMemberList, GroupMemberListError> {
    if bytes.len() < 8 {
        return Err(GroupMemberListError::Short(bytes.len()));
    }
    let group_id = rd_u32(bytes, 0);
    let member_count = rd_u32(bytes, 4);

    let n = bytes.len();
    let mut names = Vec::new();
    let mut pos = 8;
    while pos < n {
        while pos < n && !is_name_start(bytes[pos]) {
            pos += 1;
        }
        if pos >= n {
            break;
        }
        let start = pos;
        while pos < n && is_name_byte(bytes[pos]) {
            pos += 1;
        }
        if pos - start < 3 {
            continue; // too short to be an EQ name
        }
        names.push(bytes[start..pos].iter().map(|&c| c as char).collect());
        if pos < n {
            pos += 1; // consume terminator
        }
    }

    Ok(GroupMemberList {
        group_id,
        member_count,
        names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(group: u32, count: u32, body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&group.to_le_bytes());
        v.extend_from_slice(&count.to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn rejects_short() {
        assert!(parse_group_member_list(&[0u8; 7]).is_err());
    }

    #[test]
    fn scans_names_between_nulls() {
        // Leader "Aello", assist "Borin", peer "Cazic" with null padding + a
        // stray '<' (0x3c) and a 2-char run that must be skipped.
        let mut body = Vec::new();
        body.extend_from_slice(b"Aello\0\0\0");
        body.extend_from_slice(b"Borin\0<\0");
        body.extend_from_slice(b"Xy\0"); // 2 chars -> skipped
        body.extend_from_slice(b"Cazic\0");
        let buf = frame(42, 3, &body);
        let out = parse_group_member_list(&buf).unwrap();
        assert_eq!(out.group_id, 42);
        assert_eq!(out.member_count, 3);
        assert_eq!(out.names, vec!["Aello", "Borin", "Cazic"]);
    }

    #[test]
    fn empty_body_yields_no_names() {
        let buf = frame(1, 1, &[0u8; 16]);
        let out = parse_group_member_list(&buf).unwrap();
        assert!(out.names.is_empty());
    }
}
