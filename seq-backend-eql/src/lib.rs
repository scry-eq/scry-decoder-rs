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
pub mod loadout_swap;
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
pub mod spawn_appearance;
pub mod spawn_door;
pub mod spawn_rename;
pub mod special_message;
pub mod ucs_chat;
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
// Vendored 18B `hpNpcUpdateStruct` parser: eql never sees Live's fixed HP
// struct (0x2735 is the multiplexed stat channel decoded by `parse_stat_sync`
// below), so the shared `decode_hp_update` FFI is stubbed inert for eql in the
// bridge. Retained only so the module stays byte-identical to the Live fork.
pub use hp_update::{parse_hp_update, HpUpdate, HpUpdateError};
pub use illusion::{parse_illusion, Illusion, IllusionError};
pub use level_update::{parse_level_update, LevelUpdate, LevelUpdateError};
pub use loadout_swap::{parse_loadout_swap, LoadoutSwap, LoadoutSwapError};
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
pub use spawn_appearance::{
    parse_spawn_appearance, SpawnAppearance, SpawnAppearanceError,
};
pub use spawn_door::{parse_door, Door, DoorError};
pub use spawn_rename::{parse_spawn_rename, SpawnRename, SpawnRenameError};
pub use special_message::{parse_special_message, SpecialMessage, SpecialMessageError};
pub use ucs_chat::{parse_ucs_channels, parse_ucs_chat, UcsRecord};
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
#[inline]
fn rd_i64(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes([
        b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7],
    ])
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
        // eql OP_CastSpell (0x10b5) is a fixed 40B — Live's startCastStruct is 39B
        // (packed) + 1 trailing byte; the shared decoder reads slot@0/spellId@4/
        // targetId@18, all within the first 39B, so only the size gate needs 40.
        ("startCastStruct", 40),
        // eql OP_ClientUpdate S>C other-spawn position broadcast is 28B (19-bit ×8
        // packed, coord in the low bits) — Live's playerSpawnPosStruct is 24B.
        // Decoded by this crate's own parse_player_spawn_pos (28B); position
        // cracked 2026-07-10, see OPCODES_LEGENDS.md.
        ("playerSpawnPosStruct", player_spawn_pos::PAYLOAD_LEN as u32),

        // --- De-piggyback (2026-07-10): eql OWNS every mapped SZC_Match gate size ---
        // The daemon's compiled size table is Live's `everquest.h` sizeof. Before
        // this block, any mapped eql SZC_Match opcode NOT listed above silently
        // inherited that Live sizeof as its packet gate — fragile (a Live struct
        // change moves the eql gate) and the source of the WearChange/SpawnUpdateStruct
        // 32B collision (a wrong-handler binding that size-matched by coincidence).
        // So EVERY mapped SZC_Match eql opcode declares its gate size HERE, making the
        // size eql-owned and severing the Live dependency. `packetinfo.cpp` warns at
        // load if a mapped SZC_Match eql opcode is missing from this list, so the next
        // opcode-map can't reintroduce the footgun silently. Sizes equal Live's TODAY
        // (eql goldens verify byte-for-byte); each is sourced from THIS crate's own
        // pinned `eqstructs` where a binding exists, so if eql's copy later diverges the
        // gate tracks it — never Live's. The handful with no pinned binding (eql reuses
        // the shared decode) carry the capture-confirmed eql size as a literal.
        ("spawnPositionUpdate", core::mem::size_of::<eqstructs::spawnPositionUpdate>() as u32),
        ("clientTargetStruct", core::mem::size_of::<eqstructs::clientTargetStruct>() as u32),
        ("manaDecrementStruct", core::mem::size_of::<eqstructs::manaDecrementStruct>() as u32),
        ("expUpdateStruct", core::mem::size_of::<eqstructs::expUpdateStruct>() as u32),
        ("skillIncStruct", core::mem::size_of::<eqstructs::skillIncStruct>() as u32),
        ("deleteSpawnStruct", core::mem::size_of::<eqstructs::deleteSpawnStruct>() as u32),
        ("newCorpseStruct", core::mem::size_of::<eqstructs::newCorpseStruct>() as u32),
        ("remDropStruct", core::mem::size_of::<eqstructs::remDropStruct>() as u32),
        ("action2Struct", core::mem::size_of::<eqstructs::action2Struct>() as u32),
        ("actionStruct", core::mem::size_of::<eqstructs::actionStruct>() as u32),
        ("actionAltStruct", core::mem::size_of::<eqstructs::actionAltStruct>() as u32),
        // OP_SimpleMessage (0x50a7, 07/12 rotation): stock 12B {u32 eqstrId, u32
        // color, u32 0}; pinned binding so the gate tracks eql's own copy.
        ("simpleMessageStruct", core::mem::size_of::<eqstructs::simpleMessageStruct>() as u32),
        // No pinned eql binding (eql reuses the shared decode) — capture-confirmed size:
        ("playerSelfPosStruct", 42),   // OP_ClientUpdate C>S self-position (float)
        ("altExpUpdateStruct", 12),    // OP_AAExpUpdate (0x42d1): u32 altexp, u32 aaUnspent, u32 tail
        // eql door rows are 132B (Live doorStruct is 136B); OP_SpawnDoor gates
        // SZC_Modulus on this and newDoorSpawns strides via door_stride().
        ("doorStruct", spawn_door::PAYLOAD_LEN as u32),
        ("timeOfDayStruct", 8),        // OP_TimeOfDay
        ("zoneServerInfoStruct", 130), // OP_ZoneServerInfo (world)
        ("spawnAppearance2Struct", 24),// OP_SpawnAppearance2
        ("inspectDataStruct", 1956),   // OP_InspectAnswer
        // Stock-struct reuse (eql size == Live's today, stock layout): declared
        // so the gate is eql-owned, not silently inherited from everquest.h.
        ("randomStruct", 76),               // OP_RandomReply (76B, l-patch addendum 3)
        ("tradeSpellBookSlotsStruct", 8),   // OP_SwapSpell
        ("buffWindowSlotStruct", 12),       // OP_BuffWindow
    ]
}

/// eql `OP_HPUpdate` (0x2735) — the multiplexed stat-sync channel, fully
/// decoded from the community f-patch's `SpawnShell::spawnStatEQL` (6378 wide
/// packets across the pcap library, zero layout exceptions).
///
/// Layout: `u32 spawnId | u8 flags | per-stat payload | [optional u32 tail]`.
/// `flags`: bit0 = wide form; bit1 = HP; bit2 = mana; bit3 = stamina/endurance;
/// bits4-5 = reason (event/refresh/tick, ignored). The per-stat payload appears
/// in bit order (HP, then mana, then endurance): the wide form carries
/// `{i64 cur, i64 max}` per set stat bit; the narrow form carries one `u8
/// percent` per stat (`max` = 100). Some packets append a trailing `u32`
/// (purpose unknown). `flags 0x31` with no stat bits is the 6s keepalive.
///
/// Consumers (see `EqlDispatch::statSync`): HP → spawn cur/max (the wide form
/// supersedes the old percent-only feed); the player's mana → `Player::setMana`
/// and endurance → `Player::setEndurance` (real cur/max, player-only, wide form
/// only — Legends has no standalone OP_EndUpdate, so this channel is the sole
/// endurance feed). Food/water never ride this channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatSync {
    pub spawn_id: u32,
    pub wide: bool,
    pub has_hp: bool,
    pub hp_cur: i64,
    pub hp_max: i64,
    pub has_mana: bool,
    pub mana_cur: i64,
    pub mana_max: i64,
    pub has_end: bool,
    pub end_cur: i64,
    pub end_max: i64,
}

pub fn parse_stat_sync(b: &[u8]) -> Result<StatSync, DecodeError> {
    if b.len() < 5 {
        return Err(DecodeError::Short(b.len()));
    }
    let flags = b[4];
    let wide = flags & 0x01 != 0;
    let nstats = ((flags & 0x02) != 0) as usize
        + ((flags & 0x04) != 0) as usize
        + ((flags & 0x08) != 0) as usize;
    let stat_sz = if wide { 16 } else { 1 };
    let expect = 5 + stat_sz * nstats;
    // Structural canary: exact size, optionally +4 (trailing u32). A mismatch
    // means the wire layout shifted — reject rather than misread the stats.
    if b.len() != expect && b.len() != expect + 4 {
        return Err(DecodeError::BadLength(b.len()));
    }

    let mut out = StatSync {
        spawn_id: rd_u32(b, 0),
        wide,
        ..StatSync::default()
    };

    let mut pos = 5;
    // bit1 = HP, bit2 = mana, bit3 = endurance, read in that fixed order.
    for bit in 1..=3u8 {
        if flags & (1 << bit) == 0 {
            continue;
        }
        let (cur, max) = if wide {
            let pair = (rd_i64(b, pos), rd_i64(b, pos + 8));
            pos += 16;
            pair
        } else {
            let c = b[pos] as i64;
            pos += 1;
            // A narrow percent above 100 signals a layout change; drop that stat
            // (matching the f-patch's warn+skip) but keep parsing the rest.
            if c > 100 {
                continue;
            }
            (c, 100)
        };
        match bit {
            1 => { out.has_hp = true; out.hp_cur = cur; out.hp_max = max; }
            2 => { out.has_mana = true; out.mana_cur = cur; out.mana_max = max; }
            _ => { out.has_end = true; out.end_cur = cur; out.end_max = max; }
        }
    }
    Ok(out)
}

/// One decoded record from `OP_BuffList` (0x77ae). `remaining_ticks <= 0` means
/// a permanent buff (no timer). `slot` is the buff-window slot index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffListEntry {
    pub spell_id: u32,
    pub remaining_ticks: i32,
    pub slot: u32,
}

/// A decoded `OP_BuffList` (0x77ae): the authoritative active-buff list for one
/// spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffList {
    pub spawn_id: u32,
    pub entries: Vec<BuffListEntry>,
}

/// eql `OP_BuffList` (0x77ae) — a per-spawn active-buff list, sent at zone-in
/// and on every buff change, with real server-side remaining durations (the
/// patch calls it "the one that makes it work"). Layout (validated 26/26 across
/// the upperguk capture, cursor lands exactly on the packet end):
///   `u32 spawnId | u32 (seq/timestamp) | u8 flag=1 | u8 count | u8[5] pad`
/// then `count` records, each:
///   `{ u32 spellId, u32 =1, i32 remainingTicks, u32 =0 }`  (16 fixed bytes)
///   + NUL-terminated caster name (latin1; empty for self-cast)
///   + slot: `u32` BETWEEN records, `u16` on the FINAL record.
/// `remainingTicks <= 0` is permanent. The cursor-lands-on-end check is the
/// structural canary (a layout drift rejects the whole packet).
pub fn parse_buff_list(b: &[u8]) -> Result<BuffList, DecodeError> {
    if b.len() < 15 {
        return Err(DecodeError::Short(b.len()));
    }
    let spawn_id = rd_u32(b, 0);
    let count = b[9] as usize;
    let mut pos = 15;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        if pos + 16 > b.len() {
            return Err(DecodeError::BadLength(b.len()));
        }
        let spell_id = rd_u32(b, pos);
        let remaining_ticks = rd_u32(b, pos + 8) as i32;
        pos += 16;
        // NUL-terminated caster name (skip past it; the name isn't surfaced).
        match b[pos..].iter().position(|&c| c == 0) {
            Some(off) => pos += off + 1,
            None => return Err(DecodeError::BadLength(b.len())),
        }
        // slot: u32 between records, u16 on the final record.
        let slot = if i + 1 == count {
            if pos + 2 > b.len() {
                return Err(DecodeError::BadLength(b.len()));
            }
            let s = rd_u16(b, pos) as u32;
            pos += 2;
            s
        } else {
            if pos + 4 > b.len() {
                return Err(DecodeError::BadLength(b.len()));
            }
            let s = rd_u32(b, pos);
            pos += 4;
            s
        };
        entries.push(BuffListEntry { spell_id, remaining_ticks, slot });
    }
    // Structural canary: the parse must consume the packet exactly.
    if pos != b.len() {
        return Err(DecodeError::BadLength(b.len()));
    }
    Ok(BuffList { spawn_id, entries })
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
    fn stat_sync_narrow_hp_percent() {
        // 6-byte narrow HP feed: u32 id@0, flags@4=0x02 (HP, narrow), percent@5.
        let mut b = [0u8; 6];
        b[0..4].copy_from_slice(&11744u32.to_le_bytes());
        b[4] = 0x02;
        b[5] = 73;
        let s = parse_stat_sync(&b).unwrap();
        assert_eq!(s.spawn_id, 11744);
        assert!(!s.wide);
        assert!(s.has_hp);
        assert_eq!(s.hp_cur, 73);
        assert_eq!(s.hp_max, 100);
        assert!(!s.has_mana && !s.has_end);
    }

    #[test]
    fn stat_sync_wide_hp_mana() {
        // 37-byte wide HP+mana: flags 0x07 (wide|HP|mana), two {i64 cur,max} pairs.
        let mut b = [0u8; 37];
        b[0..4].copy_from_slice(&42u32.to_le_bytes());
        b[4] = 0x07;
        b[5..13].copy_from_slice(&1500i64.to_le_bytes()); // hp cur
        b[13..21].copy_from_slice(&2000i64.to_le_bytes()); // hp max
        b[21..29].copy_from_slice(&300i64.to_le_bytes()); // mana cur
        b[29..37].copy_from_slice(&450i64.to_le_bytes()); // mana max
        let s = parse_stat_sync(&b).unwrap();
        assert!(s.wide);
        assert_eq!(s.spawn_id, 42);
        assert!(s.has_hp && s.hp_cur == 1500 && s.hp_max == 2000);
        assert!(s.has_mana && s.mana_cur == 300 && s.mana_max == 450);
        assert!(!s.has_end);
    }

    #[test]
    fn stat_sync_wide_three_stats_with_tail() {
        // 53+4-byte wide HP+mana+endurance with the optional trailing u32.
        let mut b = [0u8; 57];
        b[0..4].copy_from_slice(&7u32.to_le_bytes());
        b[4] = 0x0f; // wide | HP | mana | endurance
        b[5..13].copy_from_slice(&10i64.to_le_bytes()); // hp cur
        b[13..21].copy_from_slice(&20i64.to_le_bytes()); // hp max
        b[21..29].copy_from_slice(&30i64.to_le_bytes()); // mana cur
        b[29..37].copy_from_slice(&40i64.to_le_bytes()); // mana max
        b[37..45].copy_from_slice(&50i64.to_le_bytes()); // end cur
        b[45..53].copy_from_slice(&60i64.to_le_bytes()); // end max
        // bytes 53..57 = trailing u32, ignored.
        let s = parse_stat_sync(&b).unwrap();
        assert!(s.has_hp && s.hp_cur == 10 && s.hp_max == 20);
        assert!(s.has_mana && s.mana_cur == 30 && s.mana_max == 40);
        assert!(s.has_end && s.end_cur == 50 && s.end_max == 60);
    }

    #[test]
    fn stat_sync_keepalive_narrow_over_100_and_canary() {
        // flags 0x31 = wide|reason, no stat bits → 5-byte keepalive, decodes empty.
        let mut ka = [0u8; 5];
        ka[4] = 0x31;
        let s = parse_stat_sync(&ka).unwrap();
        assert!(!s.has_hp && !s.has_mana && !s.has_end);
        // A narrow percent above 100 drops that stat.
        let mut hot = [0u8; 6];
        hot[4] = 0x02;
        hot[5] = 200;
        assert!(!parse_stat_sync(&hot).unwrap().has_hp);
        // Wrong size for the declared flags is rejected (structural canary).
        let mut bad = [0u8; 8];
        bad[4] = 0x02; // narrow HP → expect 6 or 10 bytes, not 8
        assert!(parse_stat_sync(&bad).is_err());
        // Runt.
        assert!(parse_stat_sync(&[0u8; 4]).is_err());
    }

    #[test]
    fn buff_list_parses_records_and_permanents() {
        let mut b = Vec::new();
        b.extend_from_slice(&100u32.to_le_bytes()); // spawnId
        b.extend_from_slice(&0u32.to_le_bytes()); // @4
        b.push(1); // flag
        b.push(2); // count
        b.extend_from_slice(&[0u8; 5]); // pad
        // record 1: spell 278, ticks 5, empty caster, slot 1 (u32 — not final)
        b.extend_from_slice(&278u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&5i32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0); // empty name
        b.extend_from_slice(&1u32.to_le_bytes()); // slot
        // record 2 (final): spell 515, ticks -1 (permanent), caster "X", slot 7 (u16)
        b.extend_from_slice(&515u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&(-1i32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(b"X\0");
        b.extend_from_slice(&7u16.to_le_bytes()); // final slot is u16

        let list = parse_buff_list(&b).unwrap();
        assert_eq!(list.spawn_id, 100);
        assert_eq!(list.entries.len(), 2);
        assert_eq!(list.entries[0], BuffListEntry { spell_id: 278, remaining_ticks: 5, slot: 1 });
        assert_eq!(list.entries[1], BuffListEntry { spell_id: 515, remaining_ticks: -1, slot: 7 });
        // Truncation / trailing-garbage both fail the cursor-lands-on-end canary.
        assert!(parse_buff_list(&b[..b.len() - 1]).is_err());
        let mut extra = b.clone();
        extra.push(0xAA);
        assert!(parse_buff_list(&extra).is_err());
    }
}
