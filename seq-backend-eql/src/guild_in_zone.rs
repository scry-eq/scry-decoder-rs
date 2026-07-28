//! Parsers for the guild-in-zone opcodes, which are the only source of guild
//! NAMES: a spawn carries just a (guild_id, server_id) pair.
//!
//! `OP_NewGuildInZone` fires as a guilded player enters; `OP_GuildsInZoneList`
//! is the full in-zone set, sent on zoning. Both are variable-length, ending in
//! NUL-terminated names.
//!
//! The eql wire is byte-identical to the stock Live structs here — verified by
//! dumping 15 payloads across 4 post-rotation captures: every observed size fit
//! the layout exactly, and the list's count field matched its actual entry
//! count in every sample (0, 1, 2, 3, 6, 7).
//!
//! ```text
//! NewGuildInZone   u32 guild_id, u32 server_id, cstring name
//! GuildsInZoneList u32 name_len, name (NO terminator), u32 count,
//!                  count × { u32 guild_id, u32 server_id, cstring name }
//! ```

use seq_events::GuildInZone;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuildInZoneError {
    #[error("expected at least {0} bytes, got {1}")]
    BadLength(usize, usize),
    #[error("declared {0} guilds but the payload holds {1}")]
    CountMismatch(usize, usize),
    #[error("player name length {0} overruns the {1}-byte payload")]
    BadNameLen(usize, usize),
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read one `{u32 guild_id, u32 server_id, cstring name}` at `off`, returning
/// the entry and the offset just past its NUL.
fn entry_at(b: &[u8], off: usize) -> Option<(GuildInZone, usize)> {
    let guild_id = u32_at(b, off)?;
    let server_id = u32_at(b, off + 4)?;
    let name_start = off + 8;
    // An unterminated trailing name means a truncated payload, not a name
    // running to the end — reject rather than inventing one.
    let nul = b.get(name_start..)?.iter().position(|&c| c == 0)?;
    let name = String::from_utf8_lossy(&b[name_start..name_start + nul]).into_owned();
    Some((
        GuildInZone {
            guild_id,
            server_id,
            name,
        },
        name_start + nul + 1,
    ))
}

/// `OP_NewGuildInZone` — a single guild became present in the zone.
pub fn parse_new_guild_in_zone(b: &[u8]) -> Result<GuildInZone, GuildInZoneError> {
    // 8 header bytes + at least the name's NUL.
    if b.len() < 9 {
        return Err(GuildInZoneError::BadLength(9, b.len()));
    }
    let (entry, _) = entry_at(b, 0).ok_or(GuildInZoneError::BadLength(9, b.len()))?;
    Ok(entry)
}

/// `OP_GuildsInZoneList` — every guild present in the zone. The leading player
/// name is length-prefixed and NOT NUL-terminated; it is skipped, not returned
/// (the consumer already knows who it is, and it would be a name we don't want
/// to carry around).
pub fn parse_guilds_in_zone_list(b: &[u8]) -> Result<Vec<GuildInZone>, GuildInZoneError> {
    let name_len = u32_at(b, 0).ok_or(GuildInZoneError::BadLength(4, b.len()))? as usize;
    let count_off = 4 + name_len;
    if count_off + 4 > b.len() {
        return Err(GuildInZoneError::BadNameLen(name_len, b.len()));
    }
    let count = u32_at(b, count_off).unwrap() as usize;

    let mut guilds = Vec::with_capacity(count.min(b.len() / 9 + 1));
    let mut off = count_off + 4;
    for _ in 0..count {
        match entry_at(b, off) {
            Some((entry, next)) => {
                guilds.push(entry);
                off = next;
            }
            // The count is the structural canary: a short read means we
            // mis-parsed, so fail loudly instead of returning a partial list.
            None => return Err(GuildInZoneError::CountMismatch(count, guilds.len())),
        }
    }
    Ok(guilds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(buf: &mut Vec<u8>, id: u32, srv: u32, name: &str) {
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&srv.to_le_bytes());
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
    }

    #[test]
    fn new_guild_matches_the_captured_30_byte_shape() {
        // 8 header + 21-char name + NUL = 30, the largest observed size.
        let mut b = Vec::new();
        entry(&mut b, 454, 13, "Twenty One Chars Here");
        assert_eq!(b.len(), 30);
        let g = parse_new_guild_in_zone(&b).unwrap();
        assert_eq!(g.guild_id, 454);
        assert_eq!(g.server_id, 13);
        assert_eq!(g.name, "Twenty One Chars Here");
    }

    #[test]
    fn empty_list_is_the_12_byte_case() {
        // u32 name_len=4 + 4-byte name + u32 count=0 — the smallest observed.
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Name");
        b.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(b.len(), 12);
        assert_eq!(parse_guilds_in_zone_list(&b).unwrap(), vec![]);
    }

    #[test]
    fn list_walks_variable_length_names() {
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Name");
        b.extend_from_slice(&3u32.to_le_bytes());
        entry(&mut b, 1, 13, "A");
        entry(&mut b, 22, 13, "Much Longer Guild Name");
        entry(&mut b, 333, 13, "Mid Length");

        let g = parse_guilds_in_zone_list(&b).unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].guild_id, 1);
        assert_eq!(g[1].name, "Much Longer Guild Name");
        assert_eq!(g[2].guild_id, 333);
        assert!(g.iter().all(|e| e.server_id == 13));
    }

    #[test]
    fn truncated_entry_fails_rather_than_returning_a_partial_list() {
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Name");
        b.extend_from_slice(&2u32.to_le_bytes());
        entry(&mut b, 1, 13, "First");
        b.extend_from_slice(&2u32.to_le_bytes()); // second entry cut short
        assert!(matches!(
            parse_guilds_in_zone_list(&b),
            Err(GuildInZoneError::CountMismatch(2, 1))
        ));
    }

    #[test]
    fn unterminated_name_is_rejected() {
        let mut b = Vec::new();
        b.extend_from_slice(&454u32.to_le_bytes());
        b.extend_from_slice(&13u32.to_le_bytes());
        b.extend_from_slice(b"NoNulHere");
        assert!(parse_new_guild_in_zone(&b).is_err());
    }

    #[test]
    fn absurd_name_len_is_rejected() {
        let mut b = Vec::new();
        b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        b.extend_from_slice(b"Name");
        assert!(matches!(
            parse_guilds_in_zone_list(&b),
            Err(GuildInZoneError::BadNameLen(..))
        ));
    }
}

#[cfg(test)]
mod alloc_bounds_tests {
    use super::*;

    // After an opcode rotation a stale id hands this parser a foreign payload.
    // A wire count of 4 billion must not become a 4-billion-element
    // reservation — that aborts the process rather than failing the parse.
    #[test]
    fn an_absurd_count_does_not_allocate() {
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Name");
        b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // count
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&13u32.to_le_bytes());
        b.extend_from_slice(b"Guild\0");
        // Fails cleanly (the payload cannot hold 4e9 entries), no abort.
        assert!(parse_guilds_in_zone_list(&b).is_err());
    }
}
