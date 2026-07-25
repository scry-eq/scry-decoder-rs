//! Parser for `OP_GuildMemberList` — the full guild roster (Live/Test).
//!
//! Variable-length NetStream walk, ported 1:1 from legacy showeq's
//! `guildshell.cpp` (`upstream/master`, the current-Live reference — byte-for-byte
//! the same as the daemon's C++ copy). Live's own; eql keeps a separate parser in
//! `seq-backend-eql` (its wire diverges — a wider header + a multiclass mask +
//! a trailing zone id, none of which Live has).
//!
//! ```text
//! header  LPText requesterName, skip 4, skip 4, skip 1, u32 count
//! member  LPText name, u32 level, u32 banker, u32 class, u32 rank, u32 lastOn,
//!         u8 tributeOn, u8 trophyOn, u32 tributeDonated, u32 tributeLastDonation,
//!         u8 fullMember, LPText publicNote, skip 6
//! ```
//!
//! `class` is a single class id (Live has no multiclass). `banker` packs two
//! flags: 0 none, 1 banker, 2 alt, 3 alt banker. Live does NOT carry a member's
//! zone in the roster (legacy reads none — the 6-byte tail is skipped), so there
//! is no online/offline state, unlike eql.

use crate::cursor::{Cursor, CursorError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildMemberRow {
    pub name: String,
    pub level: u32,
    /// Single class id (Live is single-class; `class_mask` is always 0).
    pub primary_class: u32,
    pub rank: u32,
    pub last_on: u32,
    pub banker: bool,
    pub alt: bool,
    pub full_member: bool,
    pub public_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildRoster {
    /// Live's roster header does not expose a guild id (legacy skips it), so it
    /// is 0; the consumer associates the roster with the player's own guild.
    pub guild_id: u32,
    pub members: Vec<GuildMemberRow>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuildRosterError {
    #[error("truncated: {0}")]
    Truncated(#[from] CursorError),
    #[error("declared {0} members but the walk did not consume the payload ({1} bytes left)")]
    CountMismatch(usize, usize),
}

fn latin1(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

pub fn parse_guild_member_list(bytes: &[u8]) -> Result<GuildRoster, GuildRosterError> {
    let mut c = Cursor::new(bytes);

    // Header: the requester's own name, three skipped fields (patch-added over
    // the years), then the member count.
    let _requester = c.read_lp_text()?;
    c.skip(4)?;
    c.skip(4)?;
    c.skip(1)?;
    let count = c.read_u32_le()? as usize;

    let mut members = Vec::with_capacity(count.min(2048));
    for _ in 0..count {
        let name = latin1(c.read_lp_text()?);
        let level = c.read_u32_le()?;
        let banker_flag = c.read_u32_le()?;
        let primary_class = c.read_u32_le()?;
        let rank = c.read_u32_le()?;
        let last_on = c.read_u32_le()?;
        let _tribute_on = c.read_u8()?;
        let _trophy_on = c.read_u8()?;
        let _tribute_donated = c.read_u32_le()?;
        let _tribute_last_donation = c.read_u32_le()?;
        let full_member = c.read_u8()? != 0;
        let public_note = latin1(c.read_lp_text()?);
        // 6-byte tail (legacy reads no zone/instance from it).
        c.skip(6)?;

        members.push(GuildMemberRow {
            name,
            level,
            primary_class,
            rank,
            last_on,
            banker: banker_flag % 2 != 0,
            alt: banker_flag > 1,
            full_member,
            public_note,
        });
    }

    // The walk should land exactly on the payload end — the canary that every
    // variable field was read correctly (legacy loops until end, i.e. assumes no
    // trailing bytes). A short landing means the layout drifted.
    if !c.at_end() {
        return Err(GuildRosterError::CountMismatch(count, c.remaining()));
    }

    Ok(GuildRoster { guild_id: 0, members })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    fn member(buf: &mut Vec<u8>, name: &str, level: u32, class: u32, rank: u32, note: &str) {
        lp(buf, name);
        buf.extend_from_slice(&level.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // banker
        buf.extend_from_slice(&class.to_le_bytes());
        buf.extend_from_slice(&rank.to_le_bytes());
        buf.extend_from_slice(&0x6a5d_c783u32.to_le_bytes()); // lastOn
        buf.push(0); // tributeOn
        buf.push(0); // trophyOn
        buf.extend_from_slice(&0u32.to_le_bytes()); // tributeDonated
        buf.extend_from_slice(&0u32.to_le_bytes()); // tributeLastDonation
        buf.push(1); // fullMember
        lp(buf, note);
        buf.extend_from_slice(&[0u8; 6]); // tail
    }

    fn roster(members: &[(&str, u32, u32, u32, &str)]) -> Vec<u8> {
        let mut b = Vec::new();
        lp(&mut b, "Self");
        b.extend_from_slice(&[0u8; 4]);
        b.extend_from_slice(&[0u8; 4]);
        b.push(0);
        b.extend_from_slice(&(members.len() as u32).to_le_bytes());
        for m in members {
            member(&mut b, m.0, m.1, m.2, m.3, m.4);
        }
        b
    }

    #[test]
    fn two_member_roster() {
        let b = roster(&[("Aaaa", 60, 1, 2, ""), ("Bbbbbb", 55, 3, 0, "alt of Aaaa")]);
        let r = parse_guild_member_list(&b).unwrap();
        assert_eq!(r.members.len(), 2);
        assert_eq!(r.members[0].level, 60);
        assert_eq!(r.members[0].primary_class, 1);
        assert_eq!(r.members[0].rank, 2);
        assert!(r.members[0].full_member);
        assert_eq!(r.members[1].primary_class, 3);
        assert_eq!(r.members[1].public_note, "alt of Aaaa");
    }

    #[test]
    fn banker_and_alt_flags() {
        let mut b = Vec::new();
        lp(&mut b, "Self");
        b.extend_from_slice(&[0u8; 9]);
        b.extend_from_slice(&1u32.to_le_bytes()); // count
        // banker_flag = 3 -> banker + alt
        lp(&mut b, "X");
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&3u32.to_le_bytes()); // banker
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0);
        b.push(0);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(1);
        lp(&mut b, "");
        b.extend_from_slice(&[0u8; 6]);
        let r = parse_guild_member_list(&b).unwrap();
        assert!(r.members[0].banker);
        assert!(r.members[0].alt);
    }

    #[test]
    fn empty_roster() {
        assert!(parse_guild_member_list(&roster(&[])).unwrap().members.is_empty());
    }

    #[test]
    fn trailing_bytes_fail_the_canary() {
        let mut b = roster(&[("Aaaa", 60, 1, 0, "")]);
        b.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_guild_member_list(&b),
            Err(GuildRosterError::CountMismatch(..))
        ));
    }

    #[test]
    fn truncated_member_errors() {
        let mut b = roster(&[("Aaaa", 60, 1, 0, "")]);
        b.truncate(b.len() - 4);
        assert!(parse_guild_member_list(&b).is_err());
    }
}
