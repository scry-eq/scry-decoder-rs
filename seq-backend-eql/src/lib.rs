//! EverQuest Legends wire decoders.
//!
//! eql owns no wire-struct types: the Legends wire is read here by byte offset
//! plus per-axis scale (ported 1:1 from the C++ `EqlDispatch`). Live and eql
//! share the daemon's neutral output structs — these parsers fill the
//! eql-relevant fields (the rest stay `Default`) so the uniform
//! `seq::rust::decode_*` bridge surface maps them exactly like the Live
//! decoders. Field offsets/scales are /loc-confirmed — see showeq-daemon
//! `OPCODES_LEGENDS.md`. Layout shuffles per patch; re-derive from captures,
//! don't memorize. **Offsets below are the 2026-07-07 post-patch layout.**
//!
//! This is the eql analogue of `seq-eqstructs-{live,test}`: it encapsulates
//! everything backend-specific about reading eql's wire. `seq-decode` stays the
//! backend-neutral shared decode layer (eql reuses it for the ~38 opcodes whose
//! wire matches Live).

use thiserror::Error;

use seq_decode::consider::Consider;
use seq_decode::new_zone::NewZone;
use seq_decode::player_profile::PlayerProfile;
use seq_decode::player_self_pos::PlayerSelfPos;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegendsError {
    #[error("payload too short: {0} bytes")]
    Short(usize),
    #[error("unexpected payload length: {0} bytes")]
    BadLength(usize),
}

// Little-endian scalar reads at a byte offset. Callers length-guard first, so
// the fixed indexing below stays in bounds.
#[inline]
fn rd_u16(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
#[inline]
fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
#[inline]
fn rd_f32(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Signed 19-bit fixed-point coordinate in the low bits of a u32 word,
/// ×8 sub-unit fraction (the same packing Live's `spawnPositionUpdate` /
/// `spawnStruct` position words use): sign-extend 19 bits, then `>> 3`
/// to integer game units.
#[inline]
fn rd_pos19(b: &[u8], o: usize) -> i16 {
    let w = rd_u32(b, o) & 0x7FFFF;
    let raw = if w & 0x4_0000 != 0 { (w as i32) - (1 << 19) } else { w as i32 };
    (raw >> 3) as i16
}

/// Latin-1 → `String` (each byte is a codepoint), matching the daemon's
/// `QString::fromLatin1`.
fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The eql-relevant subset of a spawn. `Spawn` (the Live struct) can't derive
/// `Default` (it holds a `[u32; 45]` equipment array), so eql uses this small
/// struct and the uniform `decode_spawn` bridge maps it into `ffi::Spawn`
/// (decoded x/y/z + hp; Live's raw equipment/position arrays stay zero).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegendsSpawn {
    pub id: u16,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub level: u8,
    pub cur_hp: u8,
    pub max_hp: u8,
}

// Bounds-checked LE/BE reads at an absolute offset for the variable-length
// tail walk. Unlike the fixed `rd_*` readers above (callers length-guard those
// up front), these return `None` past the end so a truncated tail degrades to
// "identity + name only" instead of panicking.
#[inline]
fn opt_u8(b: &[u8], o: usize) -> Option<u8> { b.get(o).copied() }
#[inline]
fn opt_u16_le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().unwrap()))
}
#[inline]
fn opt_u16_be(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(o..o + 2)?.try_into().unwrap()))
}
#[inline]
fn opt_u32_le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().unwrap()))
}
#[inline]
fn opt_f32_le(b: &[u8], o: usize) -> Option<f32> {
    Some(f32::from_le_bytes(b.get(o..o + 4)?.try_into().unwrap()))
}

/// NUL-terminated latin-1 out of a fixed-width name buffer.
fn cstr_field(buf: &[u8]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    latin1(&buf[..end])
}

/// The name/lastname block is a reliable ABSOLUTE anchor inside the
/// (variable-length, ~40KB) profile: `u32 == 64` + 64-byte NUL-terminated name
/// buffer + `u32 == 32` + 32-byte NUL-terminated lastname buffer. Scanning for
/// that signature — instead of a fixed offset like the old 36047 — survives the
/// inventory/spellbook size drift that shifts the block per character and per
/// patch, and everything from the block onward matches the Live tail layout.
/// Returns the offset of the leading `u32 == 64`, or `None` if not found.
///
/// VALIDATE the candidate: a real first name is a capitalized, printable,
/// NUL-terminated run — binary that happens to carry the two length words
/// won't also spell a name, so this won't false-match.
fn find_profile_name_block(b: &[u8]) -> Option<usize> {
    if b.len() < 104 {
        return None;
    }
    for p in 0..=(b.len() - 104) {
        if b[p] != 0x40 || b[p + 1] != 0 || b[p + 2] != 0 || b[p + 3] != 0 {
            continue;
        }
        if b[p + 68] != 0x20 || b[p + 69] != 0 || b[p + 70] != 0 || b[p + 71] != 0 {
            continue;
        }
        let nb = &b[p + 4..p + 68];
        if !nb[0].is_ascii_uppercase() {
            continue;
        }
        let mut ok = false;
        for &c in nb {
            match c {
                0 => {
                    ok = true; // reached the terminator after >=1 printable byte
                    break;
                }
                0x20..=0x7e => {}
                _ => break,
            }
        }
        if ok {
            return Some(p);
        }
    }
    None
}

/// Parse name + lastname + the positionally-mapped tail (birthday, expansions,
/// languages, current zone/instance, position, guild, carried+bank money)
/// starting at the name block found by `find_profile_name_block`. Byte orders
/// match the Live tail (`fillProfileStruct`): zoneId/instance LE, standState/
/// anon BE, everything else LE. Degrades gracefully on a truncated tail.
fn read_profile_name_and_tail(b: &[u8], p0: usize, prof: &mut PlayerProfile) -> Option<()> {
    let mut p = p0;

    let name_len = opt_u32_le(b, p)? as usize; // == 64
    p += 4;
    prof.name = cstr_field(b.get(p..p + 64)?);
    p += name_len;

    let last_len = opt_u32_le(b, p)? as usize; // == 32
    p += 4;
    prof.last_name = cstr_field(b.get(p..p + 32)?);
    p += last_len;

    prof.birthday_time = opt_u32_le(b, p)?;
    p += 4;
    prof.account_create_date = opt_u32_le(b, p)?;
    p += 4;
    prof.last_save_time = opt_u32_le(b, p)?;
    p += 4;
    prof.time_played_min = opt_u32_le(b, p)?;
    p += 4;
    p += 4; // unknown
    prof.expansions = opt_u32_le(b, p)?;
    p += 4;
    p += 4; // unknown

    let lang_count = opt_u32_le(b, p)? as usize;
    p += 4;
    for _ in 0..lang_count {
        prof.languages.push(opt_u8(b, p)?);
        p += 1;
    }

    prof.zone_id = opt_u16_le(b, p)?;
    p += 2;
    prof.zone_instance = opt_u16_le(b, p)?;
    p += 2;

    // Position wire order is y, x, z, heading (f32 LE), same as Live.
    prof.y = opt_f32_le(b, p)?;
    p += 4;
    prof.x = opt_f32_le(b, p)?;
    p += 4;
    prof.z = opt_f32_le(b, p)?;
    p += 4;
    prof.heading = opt_f32_le(b, p)?;
    p += 4;

    // standState / anon are read BE (`readUInt16`), like Live.
    prof.stand_state = opt_u16_be(b, p)?;
    p += 2;
    prof.anon = opt_u16_be(b, p)?;
    p += 2;

    prof.guild_id = opt_u32_le(b, p)?;
    p += 4;
    prof.guild_server_id = opt_u32_le(b, p)?;
    p += 4;

    // Tail money: 2 unknown bytes, then carried P/G/S/C, then bank P/G/S/C.
    // Cursor coin rides a second money block in the unmapped middle region
    // (recoverable by scanning for the duplicate carried quadruple) — left a
    // follow-up; cursor fields stay 0 here.
    p += 2;
    prof.platinum = opt_u32_le(b, p)?;
    p += 4;
    prof.gold = opt_u32_le(b, p)?;
    p += 4;
    prof.silver = opt_u32_le(b, p)?;
    p += 4;
    prof.copper = opt_u32_le(b, p)?;
    p += 4;
    prof.platinum_bank = opt_u32_le(b, p)?;
    p += 4;
    prof.gold_bank = opt_u32_le(b, p)?;
    p += 4;
    prof.silver_bank = opt_u32_le(b, p)?;
    p += 4;
    prof.copper_bank = opt_u32_le(b, p)?;

    Some(())
}

/// `OP_PlayerProfile` (Legends, id 0x62f0 post-patch) S>C. Two parts:
///
/// 1. Identity header, fixed offsets (patch-VERIFIED against a known char —
///    race DarkElf=6 @21, level 12 @33): gender u8 @20, race u32 @21, class u32
///    @25, level u8 @33. Truncation to the daemon's u16 race / u8 class happens
///    on the C++ side (`setIdentity`), same as the Live path.
///
/// 2. Name/lastname + tail, via `find_profile_name_block`'s absolute anchor-scan
///    (replaces the old fragile fixed offset 36047). This yields the surname
///    and the current zone/instance, position, guild, and carried+bank money —
///    the eql box is named and placed from its own profile, authoritatively,
///    like Live, rather than from the own-spawn adoption fallback.
///
/// The EQ Legends multiclass bitmask sits at @29 (u32, inserted between class
/// and level — that's why `level` is @33 not @29). `class` @25 is the primary
/// of three simultaneous classes; surfacing all three needs a proto field and
/// is deferred (the neutral `setIdentity` carries a single class).
pub fn parse_legends_profile(b: &[u8]) -> Result<PlayerProfile, LegendsError> {
    if b.len() < 34 {
        return Err(LegendsError::Short(b.len()));
    }
    let mut prof = PlayerProfile {
        gender: b[20],
        race: rd_u32(b, 21),
        class_: rd_u32(b, 25),
        level: b[33],
        ..Default::default()
    };
    // Name + tail via absolute anchor-scan. If the block isn't found (heavy
    // drift / truncation) the identity fields above still stand and the C++
    // side falls back to own-spawn name adoption.
    if let Some(p) = find_profile_name_block(b) {
        read_profile_name_and_tail(b, p, &mut prof);
    }
    Ok(prof)
}

/// `OP_ClientUpdate` (Legends) C>S, 42B: IEEE-float position + velocity + heading.
/// **post-2026-07-07 layout** (fully cracked 2026-07-08): spawnId u16 @2;
/// position gameY@10 / gameX@18 / z@30 on the wire (EQ /loc prints Y,X,Z) —
/// bound below as x=gameX=@18, y=gameY=@10, z@30; velocity gameY-vel f32@6 /
/// gameX-vel f32@26 (±2 = full run speed) → deltaX=@26, deltaY=@6; **heading =
/// u16 @14, 11-bit (0–2047 = full circle), North≈0** — confirmed against a Sense
/// Heading capture (N=2043, E=1542, S=1036, W=492, i.e. value falls ~256 per 45°).
/// deltaZ candidate @22 (0 on flat ground, unconfirmed).
pub fn parse_legends_self_pos(b: &[u8]) -> Result<PlayerSelfPos, LegendsError> {
    if b.len() != 42 {
        return Err(LegendsError::BadLength(b.len()));
    }
    // NOTE: @10 is gameY and @18 is gameX (EQ /loc prints Y,X,Z, so the /loc
    // ground truth was in that order). Bind x=gameX=@18, y=gameY=@10 to match
    // the daemon's Spawn convention; likewise deltaX=@26, deltaY=@6.
    Ok(PlayerSelfPos {
        spawn_id: rd_u16(b, 2),
        x: rd_f32(b, 18),
        y: rd_f32(b, 10),
        z: rd_f32(b, 30),
        delta_x: rd_f32(b, 26),
        delta_y: rd_f32(b, 6),
        delta_z: rd_f32(b, 22), // candidate; 0 on flat ground, unconfirmed
        heading: rd_u16(b, 14) & 0x7FF,
        ..Default::default()
    })
}

/// `OP_NewZone` (Legends, id 0x1dbf) S>C, ~340B, once per zone-in. Carries the
/// CURRENT zone as packed NUL-terminated text — `short_name` then `long_name`
/// (then a zonefile repeat + binary tail we ignore). The daemon drives
/// `ZoneMgr::setZoneByName(short, long)` directly, so no classic-id table is
/// needed. Confirmed 3-way (2026-07-08): guktop / "The City of Guk",
/// nektulos / "Nektulos Forest", unrest / "The Estate of Unrest" — each the
/// correct current zone, each a different length (packed C-strings, not
/// fixed-width arrays, so the fields sit at zone-dependent offsets).
///
/// The pre-2026-07-08 mapping pointed OP_NewZone at 0x4bc8, whose `u32@6` is the
/// BIND zone (identical across zones); that opcode is not OP_NewZone and is no
/// longer decoded here.
pub fn parse_legends_new_zone(b: &[u8]) -> Result<NewZone, LegendsError> {
    // short_name @0, long_name after its NUL. Two packed C-strings name the zone
    // + drive the map; the binary tail (safe point, exp mult, …) is unused.
    let n0 = b.iter().position(|&c| c == 0).ok_or(LegendsError::Short(b.len()))?;
    if n0 == 0 {
        return Err(LegendsError::Short(b.len()));
    }
    let rest = &b[n0 + 1..];
    let n1 = rest.iter().position(|&c| c == 0).ok_or(LegendsError::Short(b.len()))?;
    Ok(NewZone {
        short_name: latin1(&b[..n0]),
        long_name: latin1(&rest[..n1]),
        ..Default::default()
    })
}

// NOTE: eql has NO local OP_MobUpdate parser — the Legends wire turned out to
// be byte-identical to Live's `spawnPositionUpdate` (14B, packed
// y:19/z:19/u3:7/x:19/heading:12, fixed-point ×8), so `seq-bridge` routes
// eql's `decode_mob_update` to the shared `seq_decode::parse_mob_update`.
// The earlier per-offset i16 parser here (X@10, Y@4/8, Z@6/64) was reading
// truncated windows of those bitfields — correct inside ±4095 game units but
// wrapping by 8192 (Y) / 1024 (Z) beyond, the "north<->south teleport" bug.

/// `OP_ZoneSpawns` (Legends) S>C: null-terminated name, then a variable-length
/// block. Header fields stay at the front: spawnId u32 @0, level u8 @4,
/// curHpPct u8 @44, maxHpPct u8 @45. **post-2026-07-07 layout**: position sits
/// at a FIXED offset from the END of the block (block grew 326→330 NPC / 486
/// rich, but the position triple stays anchored to the tail): three u32 words
/// Z @(len-95), Y @(len-91), X @(len-87), each a **signed 19-bit fixed-point
/// (×8) coordinate in the word's low bits** (same packing as Live's
/// `spawnStruct` position words; the upper 13 bits carry other subfields).
/// /loc-confirmed on two stationary guards across both block sizes; the
/// 19-bit width (not i16) confirmed by sign-fill analysis 2026-07-08 —
/// an i16 read wraps coordinates past ±4095 by 8192 game units.
pub fn parse_legends_zone_spawn(b: &[u8]) -> Result<LegendsSpawn, LegendsError> {
    let name_len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    if name_len == 0 || name_len >= b.len() {
        return Err(LegendsError::Short(b.len()));
    }
    let s = &b[name_len + 1..];
    // Need the front header (through hp@45) AND the tail position triple.
    if s.len() < 96 {
        return Err(LegendsError::BadLength(s.len()));
    }
    let l = s.len();
    Ok(LegendsSpawn {
        id: rd_u32(s, 0) as u16,
        name: latin1(&b[..name_len]),
        // @(l-91) is gameY, @(l-87) is gameX (/loc is Y,X,Z); bind x=gameX, y=gameY.
        x: rd_pos19(s, l - 87),
        y: rd_pos19(s, l - 91),
        z: rd_pos19(s, l - 95),
        level: s[4],
        cur_hp: s[44],
        max_hp: s[45],
    })
}

/// `OP_Consider` (Legends) 24B: `{u32 self, u32 target, u32 faction, u32 =7,
/// pad, pad}`. C>S request has faction=0; the S>C reply fills faction (observed
/// 2=warmly, 4=amiably — the friendliness word; **level is NOT here**, the
/// client reads it from the spawn). Maps to the shared `Consider` (level=0) so
/// the daemon's `SpawnShell::consMessage` path is uniform with Live.
pub fn parse_legends_consider(b: &[u8]) -> Result<Consider, LegendsError> {
    if b.len() != 24 {
        return Err(LegendsError::BadLength(b.len()));
    }
    Ok(Consider {
        player_id: rd_u32(b, 0),
        target_id: rd_u32(b, 4),
        faction: rd_u32(b, 8) as i32,
        level: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_reads_identity() {
        let mut b = [0u8; 34];
        b[21..25].copy_from_slice(&6u32.to_le_bytes()); // race
        b[25..29].copy_from_slice(&5u32.to_le_bytes()); // class
        b[33] = 42; // level
        let p = parse_legends_profile(&b).unwrap();
        assert_eq!(p.race, 6);
        assert_eq!(p.class_, 5);
        assert_eq!(p.level, 42);
        // 34-byte identity-only buffer has no name slot → empty, no panic.
        assert_eq!(p.name, "");
    }

    /// Synthetic profile: 34-byte identity header, then unmapped-middle junk
    /// (zeros — no 0x40, so no false name-block match), then the name block
    /// (`u32 64` + 64B name + `u32 32` + 32B lastname) and the positional tail.
    fn profile_with_name_block(name: &str, last: &str) -> Vec<u8> {
        let mut b = vec![0u8; 34];
        b[20] = 1; // gender
        b[21..25].copy_from_slice(&6u32.to_le_bytes()); // race
        b[25..29].copy_from_slice(&5u32.to_le_bytes()); // class (primary)
        b[29..33].copy_from_slice(&0b111u32.to_le_bytes()); // classMask (not surfaced)
        b[33] = 12; // level
        b.extend_from_slice(&[0u8; 200]); // unmapped middle

        // name block
        b.extend_from_slice(&64u32.to_le_bytes());
        let mut nbuf = [0u8; 64];
        nbuf[..name.len()].copy_from_slice(name.as_bytes());
        b.extend_from_slice(&nbuf);
        b.extend_from_slice(&32u32.to_le_bytes());
        let mut lbuf = [0u8; 32];
        lbuf[..last.len()].copy_from_slice(last.as_bytes());
        b.extend_from_slice(&lbuf);

        // tail
        for v in [111u32, 222, 333, 444] {
            b.extend_from_slice(&v.to_le_bytes()); // birthday/create/save/played
        }
        b.extend_from_slice(&[0u8; 4]); // unknown
        b.extend_from_slice(&0xFFu32.to_le_bytes()); // expansions
        b.extend_from_slice(&[0u8; 4]); // unknown
        b.extend_from_slice(&1u32.to_le_bytes()); // langCount
        b.push(100); // one language
        b.extend_from_slice(&55u16.to_le_bytes()); // zoneId (LE)
        b.extend_from_slice(&0u16.to_le_bytes()); // zoneInstance (LE)
        b.extend_from_slice(&(-12.5f32).to_le_bytes()); // y
        b.extend_from_slice(&34.5f32.to_le_bytes()); // x
        b.extend_from_slice(&7.0f32.to_le_bytes()); // z
        b.extend_from_slice(&90.0f32.to_le_bytes()); // heading
        b.extend_from_slice(&100u16.to_be_bytes()); // standState (BE)
        b.extend_from_slice(&0u16.to_be_bytes()); // anon (BE)
        b.extend_from_slice(&999u32.to_le_bytes()); // guildID
        b.extend_from_slice(&1u32.to_le_bytes()); // guildServerID
        b.extend_from_slice(&[0u8; 2]); // 2 unknown
        for v in [10u32, 20, 30, 40] {
            b.extend_from_slice(&v.to_le_bytes()); // carried P/G/S/C
        }
        for v in [50u32, 60, 70, 80] {
            b.extend_from_slice(&v.to_le_bytes()); // bank P/G/S/C
        }
        b
    }

    #[test]
    fn profile_anchor_scans_name_surname_and_tail() {
        let b = profile_with_name_block("Testchar", "Surname");
        let p = parse_legends_profile(&b).unwrap();
        // identity header (fixed offsets)
        assert_eq!(p.gender, 1);
        assert_eq!(p.race, 6);
        assert_eq!(p.class_, 5);
        assert_eq!(p.level, 12);
        // name block reached by the absolute anchor-scan, not a fixed offset
        assert_eq!(p.name, "Testchar");
        assert_eq!(p.last_name, "Surname");
        // positional tail
        assert_eq!(p.expansions, 0xFF);
        assert_eq!(p.languages, vec![100]);
        assert_eq!(p.zone_id, 55);
        assert_eq!(p.x, 34.5);
        assert_eq!(p.y, -12.5);
        assert_eq!(p.z, 7.0);
        assert_eq!(p.heading, 90.0);
        assert_eq!(p.stand_state, 100);
        assert_eq!(p.guild_id, 999);
        assert_eq!(p.guild_server_id, 1);
        assert_eq!(p.platinum, 10);
        assert_eq!(p.copper, 40);
        assert_eq!(p.platinum_bank, 50);
        assert_eq!(p.copper_bank, 80);
    }

    #[test]
    fn profile_without_name_block_keeps_identity_only() {
        // No name-block signature anywhere → name stays empty, identity stands,
        // and the daemon falls back to own-spawn name adoption.
        let mut b = vec![0u8; 500];
        b[21..25].copy_from_slice(&6u32.to_le_bytes());
        b[33] = 12;
        let p = parse_legends_profile(&b).unwrap();
        assert_eq!(p.name, "");
        assert_eq!(p.last_name, "");
        assert_eq!(p.race, 6);
        assert_eq!(p.level, 12);
    }

    #[test]
    fn profile_truncated_tail_still_yields_name() {
        // Name block intact but the tail is cut off: name/lastname still land,
        // the truncated tail fields degrade to Default without panicking.
        let mut b = profile_with_name_block("Halfway", "");
        b.truncate(b.len() - 30);
        let p = parse_legends_profile(&b).unwrap();
        assert_eq!(p.name, "Halfway");
    }

    #[test]
    fn new_zone_reads_packed_names() {
        // 0x1dbf layout: short\0 long\0 <binary tail we ignore>.
        let mut b = Vec::new();
        b.extend_from_slice(b"guktop\0");
        b.extend_from_slice(b"The City of Guk\0");
        b.extend_from_slice(&[0u8; 40]);
        let z = parse_legends_new_zone(&b).unwrap();
        assert_eq!(z.short_name, "guktop");
        assert_eq!(z.long_name, "The City of Guk");
    }

    #[test]
    fn new_zone_rejects_unterminated() {
        assert!(parse_legends_new_zone(b"noterminator").is_err());
        assert!(parse_legends_new_zone(b"short\0").is_err()); // no long name
    }

    #[test]
    fn self_pos_rejects_wrong_len() {
        assert!(parse_legends_self_pos(&[0u8; 41]).is_err());
        assert!(parse_legends_self_pos(&[0u8; 43]).is_err());
    }

    #[test]
    fn self_pos_reads_floats() {
        let mut b = [0u8; 42];
        b[2..4].copy_from_slice(&7u16.to_le_bytes());
        b[18..22].copy_from_slice(&2246.5f32.to_le_bytes()); // @18 = x (gameX)
        b[10..14].copy_from_slice(&(-954.77f32).to_le_bytes()); // @10 = y (gameY)
        b[30..34].copy_from_slice(&(-4.97f32).to_le_bytes()); // z
        b[26..30].copy_from_slice(&1.5f32.to_le_bytes()); // @26 = deltaX
        b[6..10].copy_from_slice(&(-2.0f32).to_le_bytes()); // @6 = deltaY
        b[14..16].copy_from_slice(&512u16.to_le_bytes()); // heading (11-bit)
        let p = parse_legends_self_pos(&b).unwrap();
        assert_eq!(p.spawn_id, 7);
        assert_eq!(p.x, 2246.5);
        assert_eq!(p.y, -954.77);
        assert_eq!(p.z, -4.97);
        assert_eq!(p.delta_x, 1.5);
        assert_eq!(p.delta_y, -2.0);
        assert_eq!(p.heading, 512);
    }

    /// Encode a game-unit coordinate as the wire's u32 position word:
    /// signed 19-bit fixed-point ×8 in the low bits, `extra` in the upper 13.
    fn pos19(game_units: i32, extra: u32) -> [u8; 4] {
        let raw = (game_units * 8) as u32 & 0x7FFFF;
        (raw | (extra << 19)).to_le_bytes()
    }

    #[test]
    fn spawn_rejects_short_block() {
        let mut b = Vec::new();
        b.extend_from_slice(b"orc\0");
        b.extend_from_slice(&[0u8; 50]); // block < 96
        assert!(parse_legends_zone_spawn(&b).is_err());
    }

    #[test]
    fn spawn_reads_variable_block_position_from_end() {
        // 100-byte block: pos words at len-95/91/87 = 5/9/13. Upper 13 bits of
        // each word carry unrelated subfields — must not bleed into the coord.
        let mut b = Vec::new();
        b.extend_from_slice(b"an orc\0");
        let mut block = [0u8; 100];
        block[0..4].copy_from_slice(&123u32.to_le_bytes()); // id
        block[4] = 40; // level
        block[44] = 90; // curHp%
        block[45] = 100; // maxHp%
        block[5..9].copy_from_slice(&pos19(80, 0x1FFF)); // z (len-95)
        block[9..13].copy_from_slice(&pos19(-15, 715)); // y (len-91)
        block[13..17].copy_from_slice(&pos19(10, 4096)); // x (len-87)
        b.extend_from_slice(&block);
        let s = parse_legends_zone_spawn(&b).unwrap();
        assert_eq!(s.id, 123);
        assert_eq!(s.name, "an orc");
        assert_eq!(s.x, 10);
        assert_eq!(s.y, -15);
        assert_eq!(s.z, 80);
        assert_eq!(s.level, 40);
        assert_eq!(s.cur_hp, 90);
        assert_eq!(s.max_hp, 100);
    }

    #[test]
    fn spawn_position_survives_past_i16_window() {
        // A far-south spawn (|y·8| > i16::MAX): an i16 read would wrap it by
        // 8192 game units; the 19-bit decode must not.
        let mut b = Vec::new();
        b.extend_from_slice(b"a_gorge_hopper\0");
        let mut block = [0u8; 100];
        block[9..13].copy_from_slice(&pos19(-4700, 0)); // y (len-91)
        block[13..17].copy_from_slice(&pos19(5200, 0)); // x (len-87)
        b.extend_from_slice(&block);
        let s = parse_legends_zone_spawn(&b).unwrap();
        assert_eq!(s.y, -4700);
        assert_eq!(s.x, 5200);
    }

    #[test]
    fn consider_reads_self_target_faction() {
        let mut b = [0u8; 24];
        b[0..4].copy_from_slice(&27090u32.to_le_bytes()); // self
        b[4..8].copy_from_slice(&11626u32.to_le_bytes()); // target
        b[8..12].copy_from_slice(&4u32.to_le_bytes()); // faction (amiably)
        let c = parse_legends_consider(&b).unwrap();
        assert_eq!(c.player_id, 27090);
        assert_eq!(c.target_id, 11626);
        assert_eq!(c.faction, 4);
        assert_eq!(c.level, 0);
        assert!(parse_legends_consider(&[0u8; 23]).is_err());
    }
}
