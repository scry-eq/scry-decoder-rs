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

/// The character name lives at a fixed deep offset in the (variable-length,
/// ~41KB) profile: `name\0` then the skills u32 array, ~164B before the zone
/// field. The offset held across two captures (L26 / 41611B and L30 / 41891B —
/// the size delta is entirely in the tail *after* the name), but it sits past
/// the inventory block, so a big inventory change or a patch can shift it.
/// So VALIDATE: an EQ first name is a short NUL-terminated run of ASCII letters.
/// A read that isn't one (offset drifted → binary) yields "" and the daemon
/// falls back to own-spawn adoption (its prior name source — `EqlDispatch` +
/// `SpawnShell::playerChangedID`) instead of surfacing garbage.
const PROFILE_NAME_OFFSET: usize = 36047;

fn read_profile_name(b: &[u8]) -> String {
    if b.len() <= PROFILE_NAME_OFFSET {
        return String::new();
    }
    // Bounded scan: a valid name + NUL fits well inside 32 bytes, and a drifted
    // offset into binary won't produce an early NUL-terminated all-alpha run.
    let tail = &b[PROFILE_NAME_OFFSET..];
    let end = match tail.iter().take(32).position(|&c| c == 0) {
        Some(n) if (1..=20).contains(&n) => n,
        _ => return String::new(),
    };
    let name = &tail[..end];
    if !name.iter().all(|&c| c.is_ascii_alphabetic()) {
        return String::new();
    }
    latin1(name)
}

/// `OP_PlayerProfile` (Legends, id 0x62f0 post-patch) S>C: identity header —
/// race u32 @21, class u32 @25, level u8 @33. Truncation to the daemon's u16
/// race / u8 class happens on the C++ side (`setIdentity`), same as the Live
/// path. The character NAME is also decoded here (see `read_profile_name`) so
/// the eql box is named from its own profile — authoritatively, like Live —
/// rather than from the own-spawn adoption fallback.
///
/// The header offsets survived the 2026-07-07 patch — VERIFIED against a known
/// char (L12, race DarkElf=6@21, level 12@33). `class` @25 is the **primary of
/// three** (Legends chars have 3 simultaneous classes, e.g. SHD/DRU/MNK = 5/6/7);
/// the 2nd/3rd class ids live in a separate block (~@12094), not surfaced — the
/// daemon's neutral `setIdentity` carries a single class. Add a 3-class field to
/// the profile output + proto if/when the client should show all three.
pub fn parse_legends_profile(b: &[u8]) -> Result<PlayerProfile, LegendsError> {
    if b.len() < 34 {
        return Err(LegendsError::Short(b.len()));
    }
    // Identity header only. The CURRENT zone is NOT read here anymore — it comes
    // from OP_NewZone (0x1dbf, `parse_legends_new_zone`), which carries the zone
    // short/long name as text. The old u16@36211 profile read was a fragile deep
    // offset into a ~40KB variable-length payload; OP_NewZone is authoritative.
    Ok(PlayerProfile {
        race: rd_u32(b, 21),
        class_: rd_u32(b, 25),
        level: b[33],
        name: read_profile_name(b),
        ..Default::default()
    })
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

    #[test]
    fn profile_reads_name_at_deep_offset() {
        let mut b = vec![0u8; PROFILE_NAME_OFFSET + 40];
        b[21..25].copy_from_slice(&6u32.to_le_bytes());
        b[33] = 12;
        b[PROFILE_NAME_OFFSET..PROFILE_NAME_OFFSET + 8].copy_from_slice(b"Testname");
        // byte at +8 stays 0 → NUL terminator; skills u32s would follow in the wild.
        let p = parse_legends_profile(&b).unwrap();
        assert_eq!(p.name, "Testname");
        assert_eq!(p.level, 12);
    }

    #[test]
    fn profile_name_rejects_drifted_offset() {
        // Long enough, but the name slot is binary (offset drifted after a patch
        // / big inventory change): a leading non-alpha byte → "" → the daemon
        // keeps its own-spawn-adoption name instead of surfacing garbage.
        let mut g = vec![0u8; PROFILE_NAME_OFFSET + 40];
        g[PROFILE_NAME_OFFSET..PROFILE_NAME_OFFSET + 6]
            .copy_from_slice(&[0x9c, 0x12, 0x40, 0x00, 0x03, 0x00]);
        assert_eq!(parse_legends_profile(&g).unwrap().name, "");

        // No NUL within the bounded window → "" (not a scan over the whole tail).
        let mut n = vec![b'A'; PROFILE_NAME_OFFSET + 40];
        n[0..34].iter_mut().for_each(|x| *x = 0);
        n[21..25].copy_from_slice(&6u32.to_le_bytes());
        assert_eq!(parse_legends_profile(&n).unwrap().name, "");
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
