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
use seq_decode::mob_update::MobUpdate;
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
fn rd_i16(b: &[u8], o: usize) -> i16 { i16::from_le_bytes([b[o], b[o + 1]]) }
#[inline]
fn rd_f32(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
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

/// `OP_PlayerProfile` (Legends, id 0x62f0 post-patch) S>C: identity header —
/// race u32 @21, class u32 @25, level u8 @33. Truncation to the daemon's u16
/// race / u8 class happens on the C++ side (`setIdentity`), same as the Live
/// path.
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
    Ok(PlayerProfile {
        race: rd_u32(b, 21),
        class_: rd_u32(b, 25),
        level: b[33],
        ..Default::default()
    })
}

/// `OP_ClientUpdate` (Legends) C>S, 42B: IEEE-float position. spawnId u16 @2;
/// **post-2026-07-07 layout**: x @10, y @18, z @30 (/loc-confirmed via a
/// standing-still packet — all other f32 offsets read 0 at rest).
///
/// TODO(2026-07-07 re-map): deltas + heading offsets not yet re-derived (the
/// patch moved position from 22/34/38 onto the old delta/heading offsets). Left
/// 0 → speed reads 0 between updates and the facing arrow doesn't rotate; the
/// dot still tracks correctly. Pin from a turn/jump capture.
pub fn parse_legends_self_pos(b: &[u8]) -> Result<PlayerSelfPos, LegendsError> {
    if b.len() != 42 {
        return Err(LegendsError::BadLength(b.len()));
    }
    Ok(PlayerSelfPos {
        spawn_id: rd_u16(b, 2),
        x: rd_f32(b, 10),
        y: rd_f32(b, 18),
        z: rd_f32(b, 30),
        ..Default::default()
    })
}

/// `OP_NewZone` (Legends, id 0x4bc8 post-2026-07-07) S>C, 14B: the zone is now
/// a NUMERIC id (no name text on the wire — swept all 177 opcodes). zoneId =
/// u32@6 (=25 nektulos in the confirming capture; a u16@12 copies it). Names are
/// left empty; the daemon resolves id -> short/long via zones.h
/// (`ZoneMgr::setZoneById`).
///
/// Single-fire / single-zone confirmation — @6 vs @12 not yet distinguished;
/// re-check the offset against a capture from a different zone.
pub fn parse_legends_new_zone(b: &[u8]) -> Result<NewZone, LegendsError> {
    if b.len() < 10 {
        return Err(LegendsError::Short(b.len()));
    }
    Ok(NewZone {
        zone_id: rd_u32(b, 6),
        ..Default::default()
    })
}

/// `OP_MobUpdate` (Legends) S>C, 14B: spawnId u32 @0; position int16 fixed-point.
/// **post-2026-07-07 layout**: X @4 /8, Z @6 /64, Y @10 (unscaled) — confirmed
/// 4/4 against stationary mobs' `OP_ZoneSpawns` positions.
pub fn parse_legends_mob_update(b: &[u8]) -> Result<MobUpdate, LegendsError> {
    if b.len() != 14 {
        return Err(LegendsError::BadLength(b.len()));
    }
    Ok(MobUpdate {
        spawn_id: rd_u32(b, 0) as u16,
        x: (rd_i16(b, 4) / 8) as i32,
        y: rd_i16(b, 10) as i32,
        z: (rd_i16(b, 6) / 64) as i32,
        heading: 0,
    })
}

/// `OP_ZoneSpawns` (Legends) S>C: null-terminated name, then a variable-length
/// block. Header fields stay at the front: spawnId u32 @0, level u8 @4,
/// curHpPct u8 @44, maxHpPct u8 @45. **post-2026-07-07 layout**: position sits
/// at a FIXED offset from the END of the block (block grew 326→330 NPC / 486
/// rich, but the position triple stays anchored to the tail): Z @(len-95) /8,
/// X @(len-91) /8, Y @(len-87) /8 — /loc-confirmed on two stationary guards
/// across both block sizes.
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
        x: rd_i16(s, l - 91) / 8,
        y: rd_i16(s, l - 87) / 8,
        z: rd_i16(s, l - 95) / 8,
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
        b[10..14].copy_from_slice(&2246.5f32.to_le_bytes()); // x
        b[18..22].copy_from_slice(&(-954.77f32).to_le_bytes()); // y
        b[30..34].copy_from_slice(&(-4.97f32).to_le_bytes()); // z
        let p = parse_legends_self_pos(&b).unwrap();
        assert_eq!(p.spawn_id, 7);
        assert_eq!(p.x, 2246.5);
        assert_eq!(p.y, -954.77);
        assert_eq!(p.z, -4.97);
    }

    #[test]
    fn mob_update_reads_scaled() {
        let mut b = [0u8; 14];
        b[0..4].copy_from_slice(&9u32.to_le_bytes());
        b[4..6].copy_from_slice(&80i16.to_le_bytes()); // x*8 -> 10
        b[6..8].copy_from_slice(&640i16.to_le_bytes()); // z*64 -> 10
        b[10..12].copy_from_slice(&(-5i16).to_le_bytes()); // y unscaled
        let m = parse_legends_mob_update(&b).unwrap();
        assert_eq!(m.spawn_id, 9);
        assert_eq!(m.x, 10);
        assert_eq!(m.z, 10);
        assert_eq!(m.y, -5);
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
        // 100-byte block: pos at len-95/91/87 = 5/9/13.
        let mut b = Vec::new();
        b.extend_from_slice(b"an orc\0");
        let mut block = [0u8; 100];
        block[0..4].copy_from_slice(&123u32.to_le_bytes()); // id
        block[4] = 40; // level
        block[44] = 90; // curHp%
        block[45] = 100; // maxHp%
        block[5..7].copy_from_slice(&(640i16).to_le_bytes()); // z /8 -> 80  (len-95)
        block[9..11].copy_from_slice(&(80i16).to_le_bytes()); // x /8 -> 10  (len-91)
        block[13..15].copy_from_slice(&(-120i16).to_le_bytes()); // y /8 -> -15 (len-87)
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

    #[test]
    fn new_zone_reads_numeric_id() {
        let mut b = [0u8; 14];
        b[6..10].copy_from_slice(&25u32.to_le_bytes()); // zoneId @6 (nektulos)
        let z = parse_legends_new_zone(&b).unwrap();
        assert_eq!(z.zone_id, 25);
        assert!(z.short_name.is_empty());
        assert!(parse_legends_new_zone(&[0u8; 9]).is_err());
    }
}
