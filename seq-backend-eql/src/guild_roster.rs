//! Parser for `OP_GuildMemberList` — the full guild roster, sent on request and
//! at zone-in.
//!
//! The eql wire diverges from the stock live struct in three places, so the
//! shared parser cannot be reused: the header is one field wider (`u32 + u32 +
//! u16` rather than live's `u32 + u32 + u8`, the single byte that desynced the
//! whole stock walk), the class slot carries the eql MULTICLASS BITMASK instead
//! of a class id, and each record ends with a `u16` zone id that live left
//! unread. Layout per the legends branch's `guildshell.cpp`.
//!
//! ```text
//! header  LPText requester name, u32 guildId, u32 unknown, u16, u32 count
//! record  LPText name, u32 level, u32 banker, u32 classMask, u32 rank,
//!         u32 lastOn, u8 tributeOn, u8 trophyOn, u32 tributeDonated,
//!         u32 tributeLastDonation, u8 fullMember, LPText publicNote,
//!         u16 zoneId, 4 unread
//! ```
//!
//! `LPText` is a `u32` length followed by unterminated bytes. Verified against a
//! captured 2-member roster: both records land with zero slack, the second
//! ending exactly on the payload length.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildMemberRow {
    pub name: String,
    pub level: u32,
    /// 0 = none, 1 = banker, 2 = alt, 3 = alt banker. Split into the two flags
    /// below by the consumer; kept raw here so no meaning is lost.
    pub banker_flag: u32,
    /// eql multiclass bitmask (bit N = class N), NOT a class id. A character has
    /// three simultaneous classes, so several bits are set.
    pub class_mask: u32,
    /// 0 = member, 1 = officer, 2 = leader.
    pub rank: u32,
    /// Unix seconds. 0 when never seen.
    pub last_on: u32,
    pub full_member: u8,
    pub public_note: String,
    /// 0 when the member is offline.
    pub zone_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildRoster {
    pub guild_id: u32,
    pub members: Vec<GuildMemberRow>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuildRosterError {
    #[error("truncated at offset {0} (payload is {1} bytes)")]
    Truncated(usize, usize),
    #[error("declared {0} members but the payload holds {1}")]
    CountMismatch(usize, usize),
    #[error("length prefix {0} at offset {1} overruns the {2}-byte payload")]
    BadLength(u32, usize, usize),
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn u8(&mut self) -> Result<u8, GuildRosterError> {
        let v = *self
            .b
            .get(self.p)
            .ok_or(GuildRosterError::Truncated(self.p, self.b.len()))?;
        self.p += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, GuildRosterError> {
        let s = self
            .b
            .get(self.p..self.p + 2)
            .ok_or(GuildRosterError::Truncated(self.p, self.b.len()))?;
        self.p += 2;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    fn u32(&mut self) -> Result<u32, GuildRosterError> {
        let s = self
            .b
            .get(self.p..self.p + 4)
            .ok_or(GuildRosterError::Truncated(self.p, self.b.len()))?;
        self.p += 4;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn skip(&mut self, n: usize) -> Result<(), GuildRosterError> {
        if self.p + n > self.b.len() {
            return Err(GuildRosterError::Truncated(self.p, self.b.len()));
        }
        self.p += n;
        Ok(())
    }

    /// `u32` length + that many unterminated bytes.
    fn lp_text(&mut self) -> Result<String, GuildRosterError> {
        let at = self.p;
        let len = self.u32()? as usize;
        let s = self
            .b
            .get(self.p..self.p + len)
            .ok_or(GuildRosterError::BadLength(len as u32, at, self.b.len()))?;
        self.p += len;
        Ok(String::from_utf8_lossy(s).into_owned())
    }
}

pub fn parse_guild_member_list(b: &[u8]) -> Result<GuildRoster, GuildRosterError> {
    let mut c = Cursor { b, p: 0 };

    // The requester's own name leads the payload; it identifies who asked, not a
    // roster row, so it is walked past rather than returned.
    let _requester = c.lp_text()?;
    let guild_id = c.u32()?;
    let _unknown = c.u32()?;
    c.skip(2)?;
    let count = c.u32()? as usize;

    let mut members = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let name = c.lp_text()?;
        let level = c.u32()?;
        let banker_flag = c.u32()?;
        let class_mask = c.u32()?;
        let rank = c.u32()?;
        let last_on = c.u32()?;
        let _tribute_on = c.u8()?;
        let _trophy_on = c.u8()?;
        let _tribute_donated = c.u32()?;
        let _tribute_last_donation = c.u32()?;
        let full_member = c.u8()?;
        let public_note = c.lp_text()?;
        let zone_id = c.u16()?;
        // u16 unknown + u16 online flag; the roster's zone id already says
        // whether a member is in a zone, so neither is surfaced.
        c.skip(4)?;

        members.push(GuildMemberRow {
            name,
            level,
            banker_flag,
            class_mask,
            rank,
            last_on,
            full_member,
            public_note,
            zone_id,
        });
    }

    // The declared count landing exactly on the payload end is the structural
    // canary that every variable-length field was walked correctly. A roster
    // that parses short means the layout drifted, so fail rather than surface a
    // half-read one.
    if c.p != b.len() {
        return Err(GuildRosterError::CountMismatch(count, members.len()));
    }

    Ok(GuildRoster { guild_id, members })
}

/// Lowest set bit of the multiclass mask, for a consumer that can show only one
/// class. 0 when no bit is set.
pub fn primary_class(class_mask: u32) -> u8 {
    for bit in 0..32 {
        if class_mask & (1 << bit) != 0 {
            return bit as u8;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    fn member(buf: &mut Vec<u8>, name: &str, level: u32, mask: u32, rank: u32, zone: u16) {
        lp(buf, name);
        buf.extend_from_slice(&level.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // banker
        buf.extend_from_slice(&mask.to_le_bytes());
        buf.extend_from_slice(&rank.to_le_bytes());
        buf.extend_from_slice(&0x6a5d_c783u32.to_le_bytes()); // lastOn
        buf.push(0); // tributeOn
        buf.push(0); // trophyOn
        buf.extend_from_slice(&0u32.to_le_bytes()); // tributeDonated
        buf.extend_from_slice(&0u32.to_le_bytes()); // tributeLastDonation
        buf.push(1); // fullMember
        lp(buf, ""); // publicNote
        buf.extend_from_slice(&zone.to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]);
    }

    fn roster(members: &[(&str, u32, u32, u32, u16)]) -> Vec<u8> {
        let mut b = Vec::new();
        lp(&mut b, "Self");
        b.extend_from_slice(&454u32.to_le_bytes()); // guildId
        b.extend_from_slice(&13u32.to_le_bytes()); // unknown
        b.extend_from_slice(&[0u8; 2]);
        b.extend_from_slice(&(members.len() as u32).to_le_bytes());
        for m in members {
            member(&mut b, m.0, m.1, m.2, m.3, m.4);
        }
        b
    }

    #[test]
    fn two_member_roster_matches_the_captured_122_byte_shape() {
        // Mirrors the captured sample: 4-char and 6-char names, both level 24,
        // three-class masks, ranks 1 and 2, one in a zone and one offline.
        let b = roster(&[
            ("Aaaa", 24, 0x4006, 1, 50),
            ("Bbbbbb", 24, 0x0502, 2, 0),
        ]);
        assert_eq!(b.len(), 122, "layout must reproduce the captured length");

        let r = parse_guild_member_list(&b).unwrap();
        assert_eq!(r.guild_id, 454);
        assert_eq!(r.members.len(), 2);
        assert_eq!(r.members[0].level, 24);
        assert_eq!(r.members[0].class_mask, 0x4006);
        assert_eq!(r.members[0].rank, 1);
        assert_eq!(r.members[0].zone_id, 50);
        assert_eq!(r.members[1].rank, 2);
        assert_eq!(r.members[1].zone_id, 0, "offline members carry no zone");
    }

    #[test]
    fn multiclass_mask_yields_its_lowest_class() {
        // Three simultaneous classes; a single-class consumer shows the lowest.
        assert_eq!(primary_class(0x4006), 1);
        assert_eq!(primary_class(0x0502), 1);
        assert_eq!(primary_class(0b1000), 3);
        assert_eq!(primary_class(0), 0);
    }

    #[test]
    fn empty_roster_is_accepted() {
        let r = parse_guild_member_list(&roster(&[])).unwrap();
        assert!(r.members.is_empty());
        assert_eq!(r.guild_id, 454);
    }

    #[test]
    fn variable_length_notes_keep_the_walk_aligned() {
        let mut b = Vec::new();
        lp(&mut b, "Self");
        b.extend_from_slice(&454u32.to_le_bytes());
        b.extend_from_slice(&13u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 2]);
        b.extend_from_slice(&2u32.to_le_bytes());
        for (name, note) in [("Aaaa", "a longer public note"), ("Bb", "")] {
            lp(&mut b, name);
            b.extend_from_slice(&10u32.to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&2u32.to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.push(0);
            b.push(0);
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.push(1);
            lp(&mut b, note);
            b.extend_from_slice(&0u16.to_le_bytes());
            b.extend_from_slice(&[0u8; 4]);
        }
        let r = parse_guild_member_list(&b).unwrap();
        assert_eq!(r.members.len(), 2);
        assert_eq!(r.members[0].public_note, "a longer public note");
        assert_eq!(r.members[1].public_note, "");
    }

    #[test]
    fn a_short_record_fails_rather_than_returning_a_partial_roster() {
        let mut b = roster(&[("Aaaa", 24, 2, 0, 0)]);
        b.truncate(b.len() - 6);
        assert!(parse_guild_member_list(&b).is_err());
    }

    #[test]
    fn trailing_bytes_fail_the_canary() {
        // Extra bytes mean the walk mis-stepped somewhere, even though every
        // field read succeeded — exactly what the end-of-payload check catches.
        let mut b = roster(&[("Aaaa", 24, 2, 0, 0)]);
        b.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_guild_member_list(&b),
            Err(GuildRosterError::CountMismatch(..))
        ));
    }

    #[test]
    fn absurd_length_prefix_is_rejected() {
        let mut b = Vec::new();
        b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        b.extend_from_slice(b"Self");
        assert!(matches!(
            parse_guild_member_list(&b),
            Err(GuildRosterError::BadLength(..))
        ));
    }
}
