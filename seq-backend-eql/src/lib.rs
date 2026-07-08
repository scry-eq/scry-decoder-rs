//! EverQuest Legends wire decoders.
//!
//! eql owns no wire-struct types: the Legends wire is read here by byte offset
//! plus per-axis scale (ported 1:1 from the C++ `EqlDispatch`). Live and eql
//! share the daemon's neutral output structs — these parsers fill the
//! eql-relevant fields (the rest stay `Default`) so the uniform
//! `seq::rust::decode_*` bridge surface maps them exactly like the Live
//! decoders. Field offsets/scales are /loc-confirmed — see showeq-daemon
//! `OPCODES_LEGENDS.md`. Layout shuffles per patch; re-derive from captures,
//! don't memorize.
//!
//! This is the eql analogue of `seq-eqstructs-{live,test}`: it encapsulates
//! everything backend-specific about reading eql's wire. `seq-decode` stays the
//! backend-neutral shared decode layer (eql reuses it for the ~38 opcodes whose
//! wire matches Live).

use thiserror::Error;

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

/// `OP_PlayerProfile` (Legends) S>C: identity header only — race u32 @21,
/// class u32 @25, level u8 @33. Truncation to the daemon's u16 race / u8 class
/// happens on the C++ side (`setIdentity`), same as the Live path.
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

/// `OP_ClientUpdate` (Legends) C>S, 42B: IEEE-float position + deltas (unlike
/// Live's bit-packed struct). spawnId u16 @2; x @22, z @34, y @38; deltaX @30,
/// deltaY @10, deltaZ @14; heading = 11-bit field `(u32 @26 >> 10) & 0x7FF`.
pub fn parse_legends_self_pos(b: &[u8]) -> Result<PlayerSelfPos, LegendsError> {
    if b.len() != 42 {
        return Err(LegendsError::BadLength(b.len()));
    }
    Ok(PlayerSelfPos {
        spawn_id: rd_u16(b, 2),
        x: rd_f32(b, 22),
        y: rd_f32(b, 38),
        z: rd_f32(b, 34),
        delta_x: rd_f32(b, 30),
        delta_y: rd_f32(b, 10),
        delta_z: rd_f32(b, 14),
        heading: ((rd_u32(b, 26) >> 10) & 0x7FF) as u16,
        ..Default::default()
    })
}

/// `OP_NewZone` (Legends) S>C: null-terminated short name, then long name.
pub fn parse_legends_new_zone(b: &[u8]) -> Result<NewZone, LegendsError> {
    if b.len() < 2 {
        return Err(LegendsError::Short(b.len()));
    }
    let nul0 = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    if nul0 == 0 {
        return Err(LegendsError::Short(0)); // empty short name
    }
    let short_name = latin1(&b[..nul0]);
    let ls = nul0 + 1;
    let long_name = if ls >= b.len() {
        String::new()
    } else {
        let nul1 = b[ls..].iter().position(|&c| c == 0).map_or(b.len(), |p| ls + p);
        latin1(&b[ls..nul1])
    };
    Ok(NewZone { short_name, long_name, ..Default::default() })
}

/// `OP_MobUpdate` (Legends) S>C, 14B: spawnId u32 @0; position int16 fixed-point
/// — X unscaled @10, Y×8 @4, Z×64 @6.
pub fn parse_legends_mob_update(b: &[u8]) -> Result<MobUpdate, LegendsError> {
    if b.len() != 14 {
        return Err(LegendsError::BadLength(b.len()));
    }
    Ok(MobUpdate {
        spawn_id: rd_u32(b, 0) as u16,
        x: rd_i16(b, 10) as i32,
        y: (rd_i16(b, 4) / 8) as i32,
        z: (rd_i16(b, 6) / 64) as i32,
        heading: 0,
    })
}

/// `OP_ZoneSpawns` (Legends) S>C: null-terminated name, then a fixed 326-byte
/// NPC block. Block offsets: spawnId u32 @0, level u8 @4, curHpPct u8 @44,
/// maxHpPct u8 @45; position int16 mixed-scale — X @231 /8, Z @227 /8, Y @241.
pub fn parse_legends_zone_spawn(b: &[u8]) -> Result<LegendsSpawn, LegendsError> {
    let name_len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    if name_len == 0 || name_len >= b.len() {
        return Err(LegendsError::Short(b.len()));
    }
    let s = &b[name_len + 1..];
    if s.len() != 326 {
        return Err(LegendsError::BadLength(s.len()));
    }
    Ok(LegendsSpawn {
        id: rd_u32(s, 0) as u16,
        name: latin1(&b[..name_len]),
        x: rd_i16(s, 231) / 8,
        y: rd_i16(s, 241),
        z: rd_i16(s, 227) / 8,
        level: s[4],
        cur_hp: s[44],
        max_hp: s[45],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_rejects_short() {
        assert!(parse_legends_profile(&[0u8; 33]).is_err());
    }

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
        b[22..26].copy_from_slice(&100.0f32.to_le_bytes()); // x
        b[38..42].copy_from_slice(&200.0f32.to_le_bytes()); // y
        b[34..38].copy_from_slice(&50.0f32.to_le_bytes()); // z
        b[26..30].copy_from_slice(&(3u32 << 10).to_le_bytes()); // heading = 3
        let p = parse_legends_self_pos(&b).unwrap();
        assert_eq!(p.spawn_id, 7);
        assert_eq!(p.x, 100.0);
        assert_eq!(p.y, 200.0);
        assert_eq!(p.z, 50.0);
        assert_eq!(p.heading, 3);
    }

    #[test]
    fn new_zone_reads_names() {
        let mut b = Vec::new();
        b.extend_from_slice(b"qeynos\0");
        b.extend_from_slice(b"North Qeynos\0");
        let z = parse_legends_new_zone(&b).unwrap();
        assert_eq!(z.short_name, "qeynos");
        assert_eq!(z.long_name, "North Qeynos");
    }

    #[test]
    fn mob_update_reads_scaled() {
        let mut b = [0u8; 14];
        b[0..4].copy_from_slice(&9u32.to_le_bytes());
        b[10..12].copy_from_slice(&(-5i16).to_le_bytes()); // x
        b[4..6].copy_from_slice(&80i16.to_le_bytes()); // y*8 -> 10
        b[6..8].copy_from_slice(&640i16.to_le_bytes()); // z*64 -> 10
        let m = parse_legends_mob_update(&b).unwrap();
        assert_eq!(m.spawn_id, 9);
        assert_eq!(m.x, -5);
        assert_eq!(m.y, 10);
        assert_eq!(m.z, 10);
    }

    #[test]
    fn spawn_rejects_wrong_block() {
        let mut b = Vec::new();
        b.extend_from_slice(b"orc\0");
        b.extend_from_slice(&[0u8; 100]); // wrong block size
        assert!(parse_legends_zone_spawn(&b).is_err());
    }

    #[test]
    fn spawn_reads_npc_block() {
        let mut b = Vec::new();
        b.extend_from_slice(b"an orc\0");
        let mut block = [0u8; 326];
        block[0..4].copy_from_slice(&123u32.to_le_bytes()); // id
        block[4] = 40; // level
        block[44] = 90; // curHp%
        block[45] = 100; // maxHp%
        block[231..233].copy_from_slice(&80i16.to_le_bytes()); // x /8 -> 10
        block[241..243].copy_from_slice(&(-15i16).to_le_bytes()); // y
        block[227..229].copy_from_slice(&640i16.to_le_bytes()); // z /8 -> 80
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
}
