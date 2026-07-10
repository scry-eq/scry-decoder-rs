//! Self-contained EverQuest Legends decode surface.
//!
//! **eql depends on nothing from the Live decode stack.** EQ Legends is a
//! separate server that merely shares wire ancestry with Live *today*; to keep
//! a Live-only wire patch from silently corrupting eql, this crate vendors its
//! own copy of every parser + output struct (the modules below, forked from
//! `seq-decode`) and reads them through its own PINNED `seq-eqstructs-eql`
//! layouts. `seq-bridge`'s `backend-eql` feature routes every `decode_*` here —
//! there is no eql → seq-decode edge.
//!
//! Two kinds of module live here:
//!   * eql's OWN byte-offset parsers for the opcodes whose Legends wire diverges
//!     from Live — `parse_zone_spawn` / `parse_player_profile` /
//!     `parse_player_self_pos` / `parse_new_zone` / `parse_consider` below. They
//!     take the same canonical names as the vendored Live copies (which stay
//!     reachable via their module path), so the bridge's `backend` alias routes
//!     to them with no per-opcode cfg. /loc-confirmed; see `OPCODES_LEGENDS.md`.
//!   * pinned copies of the shared parsers (identical to Live *today*) that we
//!     now OWN — when eql and Live diverge, edit only the copy here.
//!
//! Field offsets/scales shuffle per patch; re-derive from captures, don't
//! memorize. **eql offsets below are the 2026-07-07 post-patch layout.**

use thiserror::Error;

/// eql's OWN pinned struct layouts (module `eqstructs`, a frozen fork of the
/// live bindings — see `eqstructs.rs`/`bindings.rs`). The vendored parser
/// modules reference `crate::eqstructs::<name>`; nothing here tracks Live's
/// generated bindings.
pub(crate) mod eqstructs;

// Vendored parser + output-struct modules (forked from seq-decode; eql-owned).
pub mod action;
pub mod action2;
pub mod action_alt;
pub mod buff;
pub mod channel_message;
pub mod click_object;
pub mod client_target;
pub mod consider;
pub mod corpse_loc;
pub mod cursor;
pub mod death;
pub mod delete_spawn;
pub mod dz_info;
pub mod dz_switch_info;
pub mod end_update;
pub mod exp_update;
pub mod formatted_message;
pub mod ground_spawn;
pub mod group_disband;
pub mod group_follow;
pub mod group_member_list;
pub mod hp_update;
pub mod illusion;
pub mod level_update;
pub mod mana_change;
pub mod mob_health;
pub mod mob_update;
pub mod new_zone;
pub mod npc_move_update;
pub mod player_profile;
pub mod player_self_pos;
pub mod player_spawn_pos;
pub mod remove_spawn;
pub mod simple_message;
pub mod skill_update;
pub mod spawn;
pub mod spawn_appearance;
pub mod spawn_door;
pub mod spawn_rename;
pub mod special_message;
pub mod stamina;
pub mod start_cast;
pub mod wear_change;
pub mod zone_change;
pub mod zone_point;

// Full public-API mirror of seq-decode, so seq-bridge can alias this crate in
// place of seq-decode for the shared decoders (identical fn + struct names).
pub use action::{parse_action, Action, ActionError};
pub use action2::{parse_action2, Action2, Action2Error};
pub use action_alt::{parse_action_alt, ActionAlt, ActionAltError};
pub use buff::{parse_buff, Buff, BuffError};
pub use channel_message::{parse_channel_message, ChannelMessage, ChannelMessageError};
pub use click_object::{parse_click_object, ClickObject, ClickObjectError};
pub use client_target::{parse_client_target, ClientTarget, ClientTargetError};
// consider: eql provides the canonical `parse_consider` itself (below); the
// vendored Live parser stays available as `consider::parse_consider`.
pub use consider::{Consider, ConsiderError};
pub use corpse_loc::{parse_corpse_loc, CorpseLoc, CorpseLocError};
pub use death::{parse_death, Death, DeathError};
pub use delete_spawn::{
    parse_delete_spawn, DeleteSpawn, DeleteSpawnError, PAYLOAD_LEN as DELETE_SPAWN_LEN,
};
pub use dz_info::{parse_dz_info, DzInfo, DzInfoError};
pub use dz_switch_info::{parse_dz_switch_info, DzSwitch, DzSwitchError};
pub use end_update::{parse_end_update, EndUpdate, EndUpdateError};
pub use exp_update::{parse_exp_update, ExpUpdate, ExpUpdateError};
pub use formatted_message::{
    parse_formatted_message, FormattedMessage, FormattedMessageError,
};
pub use ground_spawn::{parse_ground_spawn, GroundSpawn, GroundSpawnError};
pub use group_disband::{parse_group_disband, GroupDisband, GroupDisbandError};
pub use group_follow::{parse_group_follow, GroupFollow, GroupFollowError};
pub use hp_update::{HpUpdate, HpUpdateError}; // eql owns canonical `parse_hp_update` (below)
pub use illusion::{parse_illusion, Illusion, IllusionError};
pub use level_update::{parse_level_update, LevelUpdate, LevelUpdateError};
pub use mana_change::{parse_mana_change, ManaChange, ManaChangeError};
pub use mob_health::{parse_mob_health, MobHealth, MobHealthError};
pub use mob_update::{
    parse_mob_update, MobUpdate, ParseError, PAYLOAD_LEN as MOB_UPDATE_LEN,
};
pub use new_zone::{NewZone, NewZoneError}; // eql owns canonical `parse_new_zone` (below)
pub use npc_move_update::{parse_npc_move_update, NpcMoveUpdate, NpcMoveUpdateError};
pub use player_profile::{PlayerProfile, PlayerProfileError}; // eql owns canonical `parse_player_profile` (below)
pub use player_self_pos::{PlayerSelfPos, PlayerSelfPosError}; // eql owns canonical `parse_player_self_pos` (below)
pub use player_spawn_pos::{parse_player_spawn_pos, PlayerSpawnPos, PlayerSpawnPosError};
pub use remove_spawn::{parse_remove_spawn, RemoveSpawn, RemoveSpawnError};
pub use simple_message::{parse_simple_message, SimpleMessage, SimpleMessageError};
pub use skill_update::{parse_skill_update, SkillUpdate, SkillUpdateError};
pub use spawn::{parse_spawn, Spawn, SpawnError};
pub use spawn_appearance::{
    parse_spawn_appearance, SpawnAppearance, SpawnAppearanceError,
};
pub use spawn_door::{parse_door, Door, DoorError};
pub use spawn_rename::{parse_spawn_rename, SpawnRename, SpawnRenameError};
pub use special_message::{parse_special_message, SpecialMessage, SpecialMessageError};
pub use stamina::{parse_stamina, Stamina, StaminaError};
pub use start_cast::{parse_start_cast, StartCast, StartCastError};
pub use wear_change::{parse_wear_change, WearChange, WearChangeError};
pub use zone_change::{parse_zone_change, ZoneChange, ZoneChangeError};
pub use zone_point::{parse_zone_point, ZonePoint, ZonePointError};

/// Decode a NUL-padded byte buffer into an owned `String`. The vendored parser
/// modules call this as `crate::cstr_field` (copied from seq-decode's helper).
pub(crate) fn cstr_field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// eql's own diverged parsers below return the vendored output structs, brought
// into scope by the `pub use` re-exports above: Consider, NewZone,
// PlayerProfile, PlayerSelfPos.

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
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
pub struct ZoneSpawn {
    pub id: u16,
    pub name: String,
    pub last_name: String,
    pub title: String,
    pub suffix: String,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    /// h2048 heading (0..2047) — high 13 bits of the middle coord word.
    pub heading: u16,
    pub level: u8,
    pub cur_hp: u8,
    pub max_hp: u8,
    pub race: u32,
    pub class_: u32,
    pub deity: u32,
    pub guild_id: u32,
    pub guild_server_id: u32,
    pub pet_owner_id: u32,
    pub npc: u8,
    pub body_type: u32,
    pub holding: u8,
    pub state: u8,
    pub light: u8,
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

/// NUL-terminated latin-1 out of a fixed-width name buffer (eql profile names
/// use latin-1, matching the daemon's `QString::fromLatin1`; distinct from the
/// crate-root utf8-lossy `cstr_field` the vendored modules use).
fn cstr_latin1(buf: &[u8]) -> String {
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
    prof.name = cstr_latin1(b.get(p..p + 64)?);
    p += name_len;

    let last_len = opt_u32_le(b, p)? as usize; // == 32
    p += 4;
    prof.last_name = cstr_latin1(b.get(p..p + 32)?);
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
pub fn parse_player_profile(b: &[u8]) -> Result<PlayerProfile, DecodeError> {
    if b.len() < 34 {
        return Err(DecodeError::Short(b.len()));
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
pub fn parse_player_self_pos(b: &[u8]) -> Result<PlayerSelfPos, DecodeError> {
    if b.len() != 42 {
        return Err(DecodeError::BadLength(b.len()));
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
pub fn parse_new_zone(b: &[u8]) -> Result<NewZone, DecodeError> {
    // short_name @0, long_name after its NUL. Two packed C-strings name the zone
    // + drive the map; the binary tail (safe point, exp mult, …) is unused.
    let n0 = b.iter().position(|&c| c == 0).ok_or(DecodeError::Short(b.len()))?;
    if n0 == 0 {
        return Err(DecodeError::Short(b.len()));
    }
    let rest = &b[n0 + 1..];
    let n1 = rest.iter().position(|&c| c == 0).ok_or(DecodeError::Short(b.len()))?;
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
/// Sequential reader mirroring the daemon's `NetStream` (LE `readUInt*NC`,
/// NUL-terminated `readText`), bounds-checked: any overrun ends the walk with
/// `BadLength` rather than panicking (a dropped spawn, not a crash).
struct Walk<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Walk<'a> {
    fn new(b: &'a [u8]) -> Self { Walk { b, p: 0 } }
    fn pos(&self) -> usize { self.p }
    fn need(&self, n: usize) -> Result<(), DecodeError> {
        if self.p + n > self.b.len() {
            Err(DecodeError::BadLength(self.b.len()))
        } else {
            Ok(())
        }
    }
    fn skip(&mut self, n: usize) -> Result<(), DecodeError> {
        self.need(n)?;
        self.p += n;
        Ok(())
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        self.need(1)?;
        let v = self.b[self.p];
        self.p += 1;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        self.need(4)?;
        let v = rd_u32(self.b, self.p);
        self.p += 4;
        Ok(v)
    }
    /// NUL-terminated latin-1 string; advances past the NUL.
    fn text(&mut self) -> Result<String, DecodeError> {
        let start = self.p;
        while self.p < self.b.len() && self.b[self.p] != 0 {
            self.p += 1;
        }
        if self.p >= self.b.len() {
            return Err(DecodeError::BadLength(self.b.len()));
        }
        let s = latin1(&self.b[start..self.p]);
        self.p += 1; // consume NUL
        Ok(s)
    }
}

/// Decode a signed 19-bit ×8 fixed-point coordinate out of a full position word
/// (low 19 bits; upper 13 carry unrelated subfields). Same packing as
/// `rd_pos19`, but taking the whole word.
#[inline]
fn pos19_word(w: u32) -> i16 {
    let v = w & 0x7FFFF;
    let raw = if v & 0x4_0000 != 0 { (v as i32) - (1 << 19) } else { v as i32 };
    (raw >> 3) as i16
}

/// `OP_ZoneSpawns` (Legends, id 0x4606) S>C, one per spawn. Full front walk,
/// ported 1:1 from the community patch's `SpawnShell::fillSpawnStruct` (verified
/// against a 1617-record eql corpus). Supersedes the old tail-anchored partial,
/// which assumed a fixed 95-byte tail and so mis-read position and dropped
/// titles on any spawn carrying a title/suffix string block.
pub fn parse_zone_spawn(b: &[u8]) -> Result<ZoneSpawn, DecodeError> {
    let mut w = Walk::new(b);

    let name = w.text()?;
    if name.is_empty() {
        return Err(DecodeError::Short(b.len()));
    }
    let id = w.u32()? as u16;
    let level = w.u8()?;
    w.skip(16)?;
    let npc = w.u8()?;
    let _misc_data = w.u32()?;
    let _other_data = w.u8()?;
    w.skip(8)?; // unknown3, unknown4
    // (EQ Legends aura-flagged spawns carry no aura block on the wire.)

    // bodytype: `charProperties` count-prefixed u32s; the first is the bodytype.
    let char_properties = w.u8()?;
    let mut body_type = 0u32;
    if char_properties != 0 {
        for i in 0..char_properties {
            let n = w.u32()?;
            if i == 0 {
                body_type = n;
            }
        }
    }

    // EQ Legends: the HP percent sits 8 bytes past Live's slot (Live's slot
    // reads 0 for every eql spawn).
    w.skip(3)?;
    let appearance_count = w.u8()? as usize;
    w.skip(4)?;
    let cur_hp = w.u8()?;
    w.skip(33 + 4 * appearance_count)?;

    let race = w.u32()?;
    let holding = w.u8()?;
    let deity = w.u32()?;
    let guild_id = w.u32()?;
    let guild_server_id = w.u32()?;
    let class_ = w.u32()?;
    let _class_mask = w.u32()?; // EQ Legends multiclass bitmask
    w.skip(1)?;
    let state = w.u8()?;
    let light = w.u8()?;
    w.skip(1)?;

    let last_name = w.text()?;
    w.skip(2)?;
    let pet_owner_id = w.u32()?;

    // 12 extra bytes on NPCs (added 2013-06-19).
    w.skip(if npc == 1 { 49 } else { 37 })?;

    // Equipment block (skipped — not surfaced). The client's own read gate:
    // full 9-slot layout for PCs + a few humanoid NPC races, 2-slot otherwise.
    if npc == 0 || race <= 12 || race == 128 || race == 130 || race == 330 || race == 522 {
        w.skip(36 + 9 * 5 * 4)?;
    } else {
        w.skip(20 + 2 * 5 * 4)?;
    }

    // 2026-07-07 EQL insert (8 bytes) between equipment and the position words.
    w.skip(8)?;

    // posData: 4 words. Each coord is the low 19 bits (×8 fixed-point) of a
    // word; the MIDDLE word additionally carries the heading as h2048 (0..2047)
    // in its high 13 bits — per the validated `playerPosUpdateEQLStruct`
    // writeup in everquest.h ("h2048 heading in x-word high bits"; the spawn
    // union's separate word-3 heading is the unmapped 0x6000 candidate). Word 4
    // holds unmapped delta/animation fields.
    let z = pos19_word(w.u32()?);
    let mid = w.u32()?;
    let y = pos19_word(mid);
    let x = pos19_word(w.u32()?);
    let _delta_word = w.u32()?;
    let heading = ((mid >> 19) & 0x1FFF) as u16;

    // Title/suffix string block: 4 strings on ordinary spawns, 6 (title, suffix,
    // then the 4) on titled ones — no reliable presence flag. Anchor on the
    // tail: the record ends with 4 fixed bytes, u8 isMercenary, an ASCII digit
    // string ('0' run), then 53 fixed bytes. Read strings up to that anchor;
    // the first two non-empty are title then suffix.
    let mut title = String::new();
    let mut suffix = String::new();
    if b.len() >= 55 {
        let d_end = b.len() - 54; // digit-string NUL sits 54 bytes from the end
        let mut d_start = d_end;
        while d_start > 0 && b[d_start - 1] == b'0' {
            d_start -= 1;
        }
        if d_start >= 5 {
            let text_end = d_start - 1 /*isMercenary*/ - 4 /*fixed*/;
            let mut str_index = 0;
            while w.pos() < text_end {
                let s = w.text()?;
                match str_index {
                    0 => title = s,
                    1 => suffix = s,
                    _ => {}
                }
                str_index += 1;
                if str_index > 6 {
                    break; // safety: never more than 6 strings
                }
            }
        }
    }

    Ok(ZoneSpawn {
        id,
        name,
        last_name,
        title,
        suffix,
        x,
        y,
        z,
        heading,
        level,
        cur_hp,
        max_hp: 100, // curHp is a percentage; base is 100
        race,
        class_,
        deity,
        guild_id,
        guild_server_id,
        pet_owner_id,
        npc,
        body_type,
        holding,
        state,
        light,
    })
}

/// `OP_Consider` (Legends) 24B: `{u32 self, u32 target, u32 faction, u32 =7,
/// pad, pad}`. C>S request has faction=0; the S>C reply fills faction (observed
/// 2=warmly, 4=amiably — the friendliness word; **level is NOT here**, the
/// client reads it from the spawn). Maps to the shared `Consider` (level=0) so
/// the daemon's `SpawnShell::consMessage` path is uniform with Live.
pub fn parse_consider(b: &[u8]) -> Result<Consider, DecodeError> {
    if b.len() != 24 {
        return Err(DecodeError::BadLength(b.len()));
    }
    Ok(Consider {
        player_id: rd_u32(b, 0),
        target_id: rd_u32(b, 4),
        faction: rd_u32(b, 8) as i32,
        level: 0,
    })
}

/// Payload size overrides for the daemon's `SZC_Match` size registry: toml
/// `typename`s whose eql wire size diverges from the daemon's compiled (Live)
/// `everquest.h` `sizeof`. The daemon applies these over its C++ size table so a
/// diverged payload keeps its real struct NAME and size-gates on eql's real
/// size — not a hardcoded Live `sizeof`, and not a `uint8_t`/`none` placeholder.
/// Sourced from the pinned `eqstructs` sizes so a size and its decoder move
/// together. (live/test diverge from nothing; the bridge ships them an empty
/// list.)
pub fn size_overrides() -> Vec<(&'static str, u32)> {
    vec![
        // eql /consider is 24B both ways; Live's considerStruct is 32B.
        ("considerStruct", core::mem::size_of::<eqstructs::considerStruct>() as u32),
    ]
}

/// eql `OP_HPUpdate` (0x2735) — a multiplexed stat channel keyed by a subtype
/// byte at offset 4. The 6-byte subtype-0x02 packet is the HP-bar feed:
/// `u16 spawn_id, u16 0, u8 subtype=0x02, u8 hp_percent`. The daemon's spawns
/// carry percentage HP (max=100), so this maps directly. Other sizes/subtypes
/// (21/37/53-byte i64 cur/max stat pairs) are not the HP-bar feed and return an
/// error so the bridge drops them (ok=false).
pub fn parse_hp_update(b: &[u8]) -> Result<HpUpdate, DecodeError> {
    if b.len() == 6 && b[4] == 0x02 {
        return Ok(HpUpdate {
            spawn_id: rd_u16(b, 0),
            cur_hp: b[5] as i32,
            max_hp: 100,
        });
    }
    Err(DecodeError::BadLength(b.len()))
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
        let p = parse_player_profile(&b).unwrap();
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
        let p = parse_player_profile(&b).unwrap();
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
        let p = parse_player_profile(&b).unwrap();
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
        let p = parse_player_profile(&b).unwrap();
        assert_eq!(p.name, "Halfway");
    }

    #[test]
    fn new_zone_reads_packed_names() {
        // 0x1dbf layout: short\0 long\0 <binary tail we ignore>.
        let mut b = Vec::new();
        b.extend_from_slice(b"guktop\0");
        b.extend_from_slice(b"The City of Guk\0");
        b.extend_from_slice(&[0u8; 40]);
        let z = parse_new_zone(&b).unwrap();
        assert_eq!(z.short_name, "guktop");
        assert_eq!(z.long_name, "The City of Guk");
    }

    #[test]
    fn new_zone_rejects_unterminated() {
        assert!(parse_new_zone(b"noterminator").is_err());
        assert!(parse_new_zone(b"short\0").is_err()); // no long name
    }

    #[test]
    fn self_pos_rejects_wrong_len() {
        assert!(parse_player_self_pos(&[0u8; 41]).is_err());
        assert!(parse_player_self_pos(&[0u8; 43]).is_err());
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
        let p = parse_player_self_pos(&b).unwrap();
        assert_eq!(p.spawn_id, 7);
        assert_eq!(p.x, 2246.5);
        assert_eq!(p.y, -954.77);
        assert_eq!(p.z, -4.97);
        assert_eq!(p.delta_x, 1.5);
        assert_eq!(p.delta_y, -2.0);
        assert_eq!(p.heading, 512);
    }

    /// Encode a game-unit coordinate as a wire position word: signed 19-bit
    /// ×8 fixed-point in the low bits.
    fn pos_word(game_units: i32) -> [u8; 4] {
        (((game_units * 8) as u32) & 0x7FFFF).to_le_bytes()
    }

    /// Assemble a full eql zone-spawn payload matching the `fillSpawnStruct`
    /// walk. Uses the NPC / non-humanoid path (npc=1, race>12 → the 2-slot
    /// equipment branch); the 9-slot humanoid branch is exercised by the goldens.
    #[allow(clippy::too_many_arguments)]
    fn build_spawn(
        name: &str, id: u32, level: u8, cur_hp: u8, race: u32, deity: u32,
        class_: u32, z: i32, y: i32, x: i32, heading: u16, last: &str,
        title: &str, suffix: &str,
    ) -> Vec<u8> {
        let mut b = Vec::new();
        let text = |b: &mut Vec<u8>, s: &str| {
            b.extend_from_slice(s.as_bytes());
            b.push(0);
        };
        let u32le = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
        text(&mut b, name);
        u32le(&mut b, id);
        b.push(level);
        b.extend_from_slice(&[0u8; 16]);
        b.push(1); // npc
        u32le(&mut b, 0); // miscData
        b.push(0); // otherData
        b.extend_from_slice(&[0u8; 8]);
        b.push(0); // charProperties = 0 (no bodytype loop)
        b.extend_from_slice(&[0u8; 3]);
        b.push(0); // appearanceCount = 0
        b.extend_from_slice(&[0u8; 4]);
        b.push(cur_hp);
        b.extend_from_slice(&[0u8; 33]);
        u32le(&mut b, race);
        b.push(0); // holding
        u32le(&mut b, deity);
        u32le(&mut b, 0); // guildID
        u32le(&mut b, 0); // guildServerID
        u32le(&mut b, class_);
        u32le(&mut b, 0); // classMask
        b.push(0); // skip1
        b.push(0); // state
        b.push(0); // light
        b.push(0); // skip1
        text(&mut b, last); // lastName
        b.extend_from_slice(&[0u8; 2]);
        u32le(&mut b, 0); // petOwnerId
        b.extend_from_slice(&[0u8; 49]); // npc==1 extra
        b.extend_from_slice(&[0u8; 60]); // equipment (else branch: 20 + 2*5*4)
        b.extend_from_slice(&[0u8; 8]); // eql insert
        b.extend_from_slice(&pos_word(z));
        // middle word: y in low 19 bits, h2048 heading in high 13 bits
        let mid = ((((y * 8) as u32) & 0x7FFFF) | ((heading as u32) << 19)).to_le_bytes();
        b.extend_from_slice(&mid);
        b.extend_from_slice(&pos_word(x));
        u32le(&mut b, 0x6000); // delta/animation word (unmapped)
        text(&mut b, title);
        text(&mut b, suffix);
        text(&mut b, ""); // string 3
        text(&mut b, ""); // string 4
        b.extend_from_slice(&[0u8; 4]); // 4 fixed
        b.push(0); // isMercenary
        text(&mut b, "0"); // ASCII digit string
        b.extend_from_slice(&[0u8; 53]); // 53 fixed tail
        b
    }

    #[test]
    fn spawn_full_walk_reads_all_fields() {
        let b = build_spawn(
            "a guard", 4242, 55, 90, 14, 396, 3, 80, -15, 10, 1234, "", "Protector",
            "of Qeynos",
        );
        let s = parse_zone_spawn(&b).unwrap();
        assert_eq!(s.id, 4242);
        assert_eq!(s.name, "a guard");
        assert_eq!(s.level, 55);
        assert_eq!(s.cur_hp, 90);
        assert_eq!(s.max_hp, 100);
        assert_eq!(s.race, 14);
        assert_eq!(s.deity, 396);
        assert_eq!(s.class_, 3);
        assert_eq!(s.npc, 1);
        assert_eq!(s.x, 10);
        assert_eq!(s.y, -15);
        assert_eq!(s.z, 80);
        // h2048 heading out of the middle coord word's high 13 bits
        assert_eq!(s.heading, 1234);
        // titled spawn: title then suffix out of the tail-anchored string block
        assert_eq!(s.title, "Protector");
        assert_eq!(s.suffix, "of Qeynos");
    }

    #[test]
    fn spawn_last_name_and_position_past_i16_window() {
        // far spawn (|y·8| > i16::MAX) must not wrap; surname decodes; no title.
        let b = build_spawn(
            "Grarf", 7, 60, 100, 14, 0, 5, 12, -4700, 5200, 0, "Ironforge", "", "",
        );
        let s = parse_zone_spawn(&b).unwrap();
        assert_eq!(s.y, -4700);
        assert_eq!(s.x, 5200);
        assert_eq!(s.last_name, "Ironforge");
        assert_eq!(s.title, "");
        assert_eq!(s.suffix, "");
    }

    #[test]
    fn spawn_rejects_truncated() {
        let mut b = Vec::new();
        b.extend_from_slice(b"orc\0");
        b.extend_from_slice(&[0u8; 40]); // walk overruns the header
        assert!(parse_zone_spawn(&b).is_err());
    }

    #[test]
    fn consider_reads_self_target_faction() {
        let mut b = [0u8; 24];
        b[0..4].copy_from_slice(&27090u32.to_le_bytes()); // self
        b[4..8].copy_from_slice(&11626u32.to_le_bytes()); // target
        b[8..12].copy_from_slice(&4u32.to_le_bytes()); // faction (amiably)
        let c = parse_consider(&b).unwrap();
        assert_eq!(c.player_id, 27090);
        assert_eq!(c.target_id, 11626);
        assert_eq!(c.faction, 4);
        assert_eq!(c.level, 0);
        assert!(parse_consider(&[0u8; 23]).is_err());
    }

    #[test]
    fn hp_update_reads_percent_subtype() {
        // 6-byte subtype-0x02 HP feed: id@0, subtype@4=0x02, hp%@5
        let mut b = [0u8; 6];
        b[0..2].copy_from_slice(&11744u16.to_le_bytes());
        b[4] = 0x02;
        b[5] = 73;
        let h = parse_hp_update(&b).unwrap();
        assert_eq!(h.spawn_id, 11744);
        assert_eq!(h.cur_hp, 73);
        assert_eq!(h.max_hp, 100);
        // non-HP subtypes / other sizes are dropped
        let mut other = [0u8; 6];
        other[4] = 0x05;
        assert!(parse_hp_update(&other).is_err());
        assert!(parse_hp_update(&[0u8; 21]).is_err());
    }
}
