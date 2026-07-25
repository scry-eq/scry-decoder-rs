//! Parsers for the guild-in-zone opcodes (Live/Test) — the only source of guild
//! NAMES (a spawn carries just a guild_id/server_id pair). `OP_NewGuildInZone`
//! fires as a guilded player enters; `OP_GuildsInZoneList` is the full in-zone
//! set, sent on zoning. Live's own copy — the wire is the stock layout, verified
//! against a 121-byte / 4-guild live capture.
//!
//! ```text
//! NewGuildInZone   u32 guild_id, u32 server_id, cstring name
//! GuildsInZoneList u32 name_len, name (NO terminator), u32 count,
//!                  count × { u32 guild_id, u32 server_id, cstring name }
//! ```

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInZone {
    pub guild_id: u32,
    pub server_id: u32,
    pub name: String,
}

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

/// One `{u32 guild_id, u32 server_id, cstring name}` at `off`; returns the entry
/// and the offset just past its NUL.
fn entry_at(b: &[u8], off: usize) -> Option<(GuildInZone, usize)> {
    let guild_id = u32_at(b, off)?;
    let server_id = u32_at(b, off + 4)?;
    let name_start = off + 8;
    let nul = b.get(name_start..)?.iter().position(|&c| c == 0)?;
    let name = String::from_utf8_lossy(&b[name_start..name_start + nul]).into_owned();
    Some((
        GuildInZone { guild_id, server_id, name },
        name_start + nul + 1,
    ))
}

/// `OP_NewGuildInZone` — a single guild became present in the zone.
pub fn parse_new_guild_in_zone(b: &[u8]) -> Result<GuildInZone, GuildInZoneError> {
    if b.len() < 9 {
        return Err(GuildInZoneError::BadLength(9, b.len()));
    }
    let (entry, _) = entry_at(b, 0).ok_or(GuildInZoneError::BadLength(9, b.len()))?;
    Ok(entry)
}

/// `OP_GuildsInZoneList` — every guild present in the zone. The leading player
/// name is length-prefixed and NOT NUL-terminated; it is skipped.
pub fn parse_guilds_in_zone_list(b: &[u8]) -> Result<Vec<GuildInZone>, GuildInZoneError> {
    let name_len = u32_at(b, 0).ok_or(GuildInZoneError::BadLength(4, b.len()))? as usize;
    let count_off = 4 + name_len;
    if count_off + 4 > b.len() {
        return Err(GuildInZoneError::BadNameLen(name_len, b.len()));
    }
    let count = u32_at(b, count_off).unwrap() as usize;

    let mut guilds = Vec::with_capacity(count.min(4096));
    let mut off = count_off + 4;
    for _ in 0..count {
        match entry_at(b, off) {
            Some((entry, next)) => {
                guilds.push(entry);
                off = next;
            }
            // The count is the structural canary — a short read means a mis-parse.
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
    fn list_matches_captured_shape() {
        // 4-char player name, 4 guilds — the live-guild capture's shape.
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Self");
        b.extend_from_slice(&4u32.to_le_bytes());
        entry(&mut b, 15, 180, "Guild Fifteen Chars");
        entry(&mut b, 3, 180, "Third Guild");
        entry(&mut b, 32, 180, "G");
        entry(&mut b, 7, 180, "Seven");
        let g = parse_guilds_in_zone_list(&b).unwrap();
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].guild_id, 15);
        assert_eq!(g[0].server_id, 180);
        assert_eq!(g[3].name, "Seven");
    }

    #[test]
    fn single_new_guild() {
        let mut b = Vec::new();
        entry(&mut b, 15, 180, "Gnomes Inc");
        let g = parse_new_guild_in_zone(&b).unwrap();
        assert_eq!(g.guild_id, 15);
        assert_eq!(g.name, "Gnomes Inc");
    }

    #[test]
    fn short_count_fails() {
        let mut b = Vec::new();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Self");
        b.extend_from_slice(&2u32.to_le_bytes()); // claims 2
        entry(&mut b, 1, 180, "One"); // only 1 present
        assert!(matches!(
            parse_guilds_in_zone_list(&b),
            Err(GuildInZoneError::CountMismatch(2, 1))
        ));
    }
}
