//! Parser for `OP_ExpandedGuildInfo` — "guild ranks and other misc guild data".
//!
//! The opcode is a tagged union: a 4-byte `action` at offset 0 selects the
//! payload shape, so the wire size varies (192 / 352 / 104 / 4184 B observed).
//! We decode only the one we consume — `action == 3`, the guild rank-name table,
//! which arrives as one fixed 192-byte packet per rank (one rank name each). The
//! other actions (0/1 = misc guild config incl. the recruiting URL, 5/6 = other)
//! are recognised but not decoded.
//!
//! action==3 (rank name), 192 bytes, all fixed offsets (NOT a NetStream walk):
//! ```text
//!  @0   u32  action (== 3)
//!  @8   u32  guild_id
//!  @12  u32  server_id
//!  @16  char leader[64]      (guild leader name — not decoded)
//!  @88  u32  rank_index       1-based; matches the roster member `rank` field
//!  @92  char rank_name[48]    NUL-terminated
//!  @140 char note[52]         guild note/description — not decoded
//! ```
//! The rank_index values line up 1:1 with the roster's `rank` (a live capture's
//! roster ranks {1,2,3,5,8} are a subset of these 1..8 ordinals), so a consumer
//! resolves a member's numeric rank straight through this table.

/// The `action` value that carries a rank name.
pub const ACTION_RANK_NAME: u32 = 3;

const OFF_GUILD_ID: usize = 8;
const OFF_RANK_INDEX: usize = 88;
const OFF_RANK_NAME: usize = 92;
const RANK_NAME_MAX: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExpandedGuildInfo {
    /// Tagged-union selector (offset 0). `ACTION_RANK_NAME` is the only one we
    /// decode past the guild id.
    pub action: u32,
    pub guild_id: u32,
    /// 1-based rank ordinal; meaningful only when `action == ACTION_RANK_NAME`.
    pub rank_index: u32,
    /// Rank label (e.g. "Officer"); empty unless `action == ACTION_RANK_NAME`.
    pub rank_name: String,
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read a NUL-terminated latin1 string of at most `max` bytes starting at `o`.
fn cstr_at(b: &[u8], o: usize, max: usize) -> String {
    let end = (o + max).min(b.len());
    let s = &b[o.min(b.len())..end];
    let s = &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())];
    String::from_utf8_lossy(s).into_owned()
}

/// Decode an `OP_ExpandedGuildInfo` payload. Always returns a value: `action`
/// and `guild_id` are read when present; the rank fields are filled only for
/// the rank-name action (others leave them 0 / empty).
pub fn parse_expanded_guild_info(bytes: &[u8]) -> ExpandedGuildInfo {
    let action = u32_at(bytes, 0).unwrap_or(0);
    let guild_id = u32_at(bytes, OFF_GUILD_ID).unwrap_or(0);

    let mut info = ExpandedGuildInfo { action, guild_id, ..Default::default() };

    if action == ACTION_RANK_NAME {
        if let Some(idx) = u32_at(bytes, OFF_RANK_INDEX) {
            info.rank_index = idx;
            info.rank_name = cstr_at(bytes, OFF_RANK_NAME, RANK_NAME_MAX);
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a 192-byte action==3 rank-name packet.
    fn rank_packet(guild_id: u32, rank_index: u32, name: &str) -> Vec<u8> {
        let mut b = vec![0u8; 192];
        b[0..4].copy_from_slice(&ACTION_RANK_NAME.to_le_bytes());
        b[OFF_GUILD_ID..OFF_GUILD_ID + 4].copy_from_slice(&guild_id.to_le_bytes());
        b[OFF_RANK_INDEX..OFF_RANK_INDEX + 4].copy_from_slice(&rank_index.to_le_bytes());
        let n = name.as_bytes().len().min(RANK_NAME_MAX - 1);
        b[OFF_RANK_NAME..OFF_RANK_NAME + n].copy_from_slice(&name.as_bytes()[..n]);
        b
    }

    #[test]
    fn decodes_rank_name() {
        let p = rank_packet(15, 3, "Officer");
        let i = parse_expanded_guild_info(&p);
        assert_eq!(i.action, ACTION_RANK_NAME);
        assert_eq!(i.guild_id, 15);
        assert_eq!(i.rank_index, 3);
        assert_eq!(i.rank_name, "Officer");
    }

    #[test]
    fn stale_tail_after_nul_is_ignored() {
        // Rank-name buffers are reused, so bytes after the NUL can be a prior,
        // longer name's tail. Only the NUL-terminated prefix is the name.
        let mut p = rank_packet(15, 5, "Member");
        // "Member\0" then leftover "Member" tail
        p[OFF_RANK_NAME + 7..OFF_RANK_NAME + 13].copy_from_slice(b"Member");
        assert_eq!(parse_expanded_guild_info(&p).rank_name, "Member");
    }

    #[test]
    fn non_rank_action_leaves_rank_fields_empty() {
        // action==1 (misc guild config) — guild id still read, rank fields not.
        let mut b = vec![0u8; 4184];
        b[0..4].copy_from_slice(&1u32.to_le_bytes());
        b[OFF_GUILD_ID..OFF_GUILD_ID + 4].copy_from_slice(&15u32.to_le_bytes());
        let i = parse_expanded_guild_info(&b);
        assert_eq!(i.action, 1);
        assert_eq!(i.guild_id, 15);
        assert_eq!(i.rank_index, 0);
        assert!(i.rank_name.is_empty());
    }

    #[test]
    fn short_payload_does_not_panic() {
        assert_eq!(parse_expanded_guild_info(&[]).action, 0);
        assert_eq!(parse_expanded_guild_info(&[3, 0, 0, 0]).action, 3);
    }
}
