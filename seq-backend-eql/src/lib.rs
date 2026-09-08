//! Self-contained EverQuest Legends decode surface: eql vendors its own copy of
//! every parser and reads them through its own pinned `eqstructs`, so there is
//! no eql -> `seq-decode` edge and a Live wire patch cannot reach it. Modules
//! keep their Live counterparts' names, so the bridge routes with no per-opcode
//! cfg. Cite opcodes by NAME — EQL rotates ids nearly every patch.

use thiserror::Error;

/// eql's OWN pinned struct layouts, a hand-maintained fork of the live
/// bindings; nothing here tracks Live's generated ones.
pub(crate) mod eqstructs;

// Vendored parser + output-struct modules (forked from seq-decode; eql-owned).
pub mod action;
pub mod action2;
pub mod action_alt;
pub mod backend;
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
pub mod guild_expanded_info;
pub mod guild_in_zone;
pub mod guild_motd;
pub mod guild_roster;
pub mod hp_update;
pub mod illusion;
pub mod item_packet;
pub mod level_update;
pub mod links;
pub mod loadout_swap;
pub mod loot_drops;
pub mod loot_message;
pub mod loot_track;
pub mod loot_transaction;
pub mod mana_change;
pub mod mob_health;
pub mod mob_update;
pub mod money_update;
pub mod new_zone;
pub mod npc_move_update;
pub mod player_profile;
pub mod player_self_pos;
pub mod player_spawn_pos;
pub mod remove_spawn;
pub mod self_pos_breadcrumb;
pub mod self_track;
pub mod simple_message;
pub mod skill_update;
pub mod spawn_appearance;
pub mod spawn_door;
pub mod spawn_rename;
pub mod special_message;
pub mod stamina;
pub mod start_cast;
pub mod ucs_chat;
pub mod wear_change;
pub mod zone_change;
pub mod zone_point;
pub mod zone_server_info;

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
pub use formatted_message::{parse_formatted_message, FormattedMessage, FormattedMessageError};
pub use ground_spawn::{parse_ground_spawn, GroundSpawn, GroundSpawnError};
pub use group_disband::{parse_group_disband, GroupDisband, GroupDisbandError};
pub use group_follow::{parse_group_follow, GroupFollow, GroupFollowError};
pub use loot_drops::{parse_loot_drops, LootDrops, LootDropsError};
pub use loot_message::{parse_loot_message, LootMessage, LootMessageError};
pub use loot_track::{LootRow, LootSource, LootTracker};
pub use loot_transaction::{parse_loot_transaction, LootTransaction, LootTransactionError};
pub use money_update::{parse_money_update, MoneyUpdate, MoneyUpdateError};
// eql never sees Live's fixed HP struct (`parse_stat_sync` owns that channel),
// so this is retained only to stay byte-identical to the Live fork.
pub use hp_update::{parse_hp_update, HpUpdate, HpUpdateError};
pub use illusion::{parse_illusion, Illusion, IllusionError};
pub use level_update::{parse_level_update, LevelUpdate, LevelUpdateError};
pub use loadout_swap::{parse_loadout_swap, LoadoutSwap, LoadoutSwapError};
pub use mana_change::{parse_mana_change, ManaChange, ManaChangeError};
pub use mob_health::{parse_mob_health, MobHealth, MobHealthError};
pub use mob_update::{parse_mob_update, MobUpdate, ParseError, PAYLOAD_LEN as MOB_UPDATE_LEN};
pub use new_zone::{NewZone, NewZoneError}; // eql owns canonical `parse_new_zone` (below)
pub use npc_move_update::{parse_npc_move_update, NpcMoveUpdate, NpcMoveUpdateError};
pub use player_profile::{PlayerProfile, PlayerProfileError}; // eql owns canonical `parse_player_profile` (below)
pub use player_self_pos::{parse_player_self_pos, PlayerSelfPos, PlayerSelfPosError}; // module owns the canonical parser (validates against its own PAYLOAD_LEN, same const the size override reads)
pub use player_spawn_pos::{parse_player_spawn_pos, PlayerSpawnPos, PlayerSpawnPosError};
pub use remove_spawn::{parse_remove_spawn, RemoveSpawn, RemoveSpawnError};
pub use self_pos_breadcrumb::{parse_self_pos_breadcrumb, BreadcrumbPoint, SelfPosBreadcrumb};
pub use self_track::{SelfPosRouting, SelfStat, SelfTracker, SpawnRouting, SAME_BATCH};
pub use simple_message::{parse_simple_message, SimpleMessage, SimpleMessageError};
pub use skill_update::{parse_skill_update, SkillUpdate, SkillUpdateError};
pub use spawn_appearance::{parse_spawn_appearance, SpawnAppearance, SpawnAppearanceError};
pub use spawn_door::{parse_door, Door, DoorError};
pub use spawn_rename::{parse_spawn_rename, SpawnRename, SpawnRenameError};
pub use special_message::{parse_special_message, SpecialMessage, SpecialMessageError};
pub use stamina::{parse_stamina, Stamina, StaminaError};
pub use start_cast::{parse_start_cast, StartCast, StartCastError};
pub use ucs_chat::{parse_ucs_channels, parse_ucs_chat, UcsRecord};
pub use wear_change::{parse_wear_change, WearChange, WearChangeError};
pub use zone_change::{parse_zone_change, ZoneChange, ZoneChangeError};
pub use zone_point::{parse_zone_point, ZonePoint, ZonePointError};

/// Decode a NUL-padded byte buffer into an owned `String`. The vendored parser
/// modules call this as `crate::cstr_field` (copied from seq-decode's helper).
pub(crate) fn cstr_field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// eql's own diverged parsers below return the vendored output structs
// re-exported above: Consider, NewZone, PlayerProfile, PlayerSelfPos.

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("payload too short: {0} bytes")]
    Short(usize),
    #[error("unexpected payload length: {0} bytes")]
    BadLength(usize),
    #[error("{0} is not plausible text")]
    Implausible(&'static str),
}

// Little-endian scalar reads at a byte offset. Callers length-guard first, so
// the fixed indexing below stays in bounds.
#[inline]
fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
#[inline]
fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
#[inline]
fn rd_i64(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ])
}

/// Latin-1 → `String` (each byte is a codepoint), matching the daemon's
/// `QString::fromLatin1`.
fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The eql-relevant subset of a spawn; Live's `Spawn` can't derive `Default`,
/// so the `decode_spawn` bridge maps this small struct into `ffi::Spawn`.
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
    /// h2048 heading (0..2047). The 09/01 `posData` carries no facing, so this
    /// reads 0 until a capture re-locates it.
    pub heading: u16,
    pub level: u8,
    pub cur_hp: u8,
    pub max_hp: u8,
    pub race: u32,
    pub class_: u32,
    pub class_mask: u32,
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

// Bounds-checked reads for the variable-length tail walk: `None` past the end,
// so a truncated tail degrades to "identity + name only" instead of panicking.
#[inline]
fn opt_u8(b: &[u8], o: usize) -> Option<u8> {
    b.get(o).copied()
}
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

/// NUL-terminated latin-1 out of a fixed-width name buffer — profile names are
/// latin-1, unlike the crate-root utf8-lossy `cstr_field`.
fn cstr_latin1(buf: &[u8]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    latin1(&buf[..end])
}

/// Offset of the `u32 64` + name + `u32 32` + lastname signature, or `None`.
/// Scanning survives the inventory drift that moves the block per character.
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

/// Spellbook slot count (upstream's `MAX_SPELLBOOK_SLOTS`).
const SPELLBOOK_SLOTS: usize = 800;

/// The spellbook is `SPELLBOOK_SLOTS` × {u32 spellId, u32 small word} with
/// `0xffffffff` in empty slots; the region is unmapped, so scan for it.
fn find_profile_spellbook(b: &[u8]) -> Option<usize> {
    let span = SPELLBOOK_SLOTS * 8;
    if b.len() < span {
        return None;
    }
    'candidate: for o in (0..=(b.len() - span)).step_by(2) {
        let mut populated = 0usize;
        for k in 0..SPELLBOOK_SLOTS {
            let id = rd_u32(b, o + k * 8);
            let word = rd_u32(b, o + k * 8 + 4);
            if word > 4096 || (id != 0xffff_ffff && !(1..=100_000).contains(&id)) {
                continue 'candidate;
            }
            if id != 0xffff_ffff {
                populated += 1;
            }
        }
        if populated >= 64 {
            return Some(o);
        }
    }
    None
}

/// Parse name, lastname and the positional tail from the block located by
/// `find_profile_name_block`; byte orders match the Live tail.
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

    // Money is NOT in this walk: carried is followed by CURSOR, not bank, so
    // `parse_player_profile` reads it from fixed offsets instead.
    Some(())
}

/// `OP_PlayerProfile` (Legends) S>C: a fixed identity/coin prefix, a
/// count-prefixed walk to the AA and skill arrays, then a signature-anchored
/// name/lastname + tail. Every read is bounds-guarded, so a truncated profile
/// keeps whatever it decoded, and coin denominations are NOT normalized on the
/// wire — always sum to copper rather than assuming each is below 10.
///
/// ```text
///   /*0020*/ u8  gender
///   /*0021*/ u32 race
///   /*0025*/ u32 class      primary of the three the mask carries
///   /*0029*/ u32 classMask  multiclass bitmask (why level is @33, not @29)
///   /*0033*/ u8  level
///   /*0962*/ u32 STR STA CHA DEX INT AGI WIS   base stats, not gear totals
///   /*33687*/ u32 carried  P/G/S/C      /*33703*/ u32 cursor P/G/S/C
///   /*33777*/ u32 stance   /*33781*/ u32 invocation
///   bank      signature-located past the inventory mirror of the carried purse
/// ```
pub fn parse_player_profile(b: &[u8]) -> Result<PlayerProfile, DecodeError> {
    if b.len() < 34 {
        return Err(DecodeError::Short(b.len()));
    }
    let mut prof = PlayerProfile {
        gender: b[20],
        race: rd_u32(b, 21),
        class_: rd_u32(b, 25),
        class_mask: rd_u32(b, 29), // EQL multiclass bitmask (bit N = class N)
        level: b[33],
        ..Default::default()
    };
    if b.len() >= 990 {
        prof.str_ = rd_u32(b, 962);
        prof.sta = rd_u32(b, 966);
        prof.cha = rd_u32(b, 970);
        prof.dex = rd_u32(b, 974);
        prof.int_ = rd_u32(b, 978);
        prof.agi = rd_u32(b, 982);
        prof.wis = rd_u32(b, 986);
    }
    walk_profile_skills(b, &mut prof);

    if let Some(o) = find_profile_spellbook(b) {
        prof.spell_book = (0..SPELLBOOK_SLOTS)
            .map(|k| rd_u32(b, o + k * 8) as i32)
            .collect();
    }

    if let Some(p) = find_profile_name_block(b) {
        read_profile_name_and_tail(b, p, &mut prof);
    }
    if b.len() >= 33785 {
        prof.stance = rd_u32(b, 33777);
        prof.invocation = rd_u32(b, 33781);
    }
    if b.len() >= 33719 {
        prof.platinum = rd_u32(b, 33687);
        prof.gold = rd_u32(b, 33691);
        prof.silver = rd_u32(b, 33695);
        prof.copper = rd_u32(b, 33699);
        prof.platinum_cursor = rd_u32(b, 33703);
        prof.gold_cursor = rd_u32(b, 33707);
        prof.silver_cursor = rd_u32(b, 33711);
        prof.copper_cursor = rd_u32(b, 33715);
    }
    if b.len() >= 36277 {
        let carried = [prof.platinum, prof.gold, prof.silver, prof.copper];
        if carried != [0; 4] {
            let mut sig = [0u8; 16];
            for (i, v) in carried.iter().enumerate() {
                sig[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            // Search starts past the carried block so we find the mirror, not it.
            if let Some(off) = b[33703..]
                .windows(16)
                .position(|w| w == sig)
                .map(|o| o + 33703)
                .filter(|o| o + 32 <= b.len())
            {
                prof.platinum_inventory = rd_u32(b, off);
                prof.gold_inventory = rd_u32(b, off + 4);
                prof.silver_inventory = rd_u32(b, off + 8);
                prof.copper_inventory = rd_u32(b, off + 12);
                prof.platinum_bank = rd_u32(b, off + 16);
                prof.gold_bank = rd_u32(b, off + 20);
                prof.silver_bank = rd_u32(b, off + 24);
                prof.copper_bank = rd_u32(b, off + 28);
            }
        }
    }
    Ok(prof)
}

/// Walk the eql `OP_PlayerProfile` from @35 through every count-prefixed section
/// to the AA and SKILL arrays, mirroring the daemon's `fillProfileStructEQL`.
/// Section counts vary per character, so the cursor must be walked; any
/// underflow returns early and leaves the fields filled so far.
///
/// ```text
///   /*0000*/ checksum … /*0033*/ level, /*0034*/ level1 -> resume at 35
///   u32 count + count*20   bind points
///   u32 deity, u32 intoxication
///   u32 count + count*4    spell-slot refresh
///   u32 count + count*20   equipment
///   u32 count + count*20, u32 count + count*4, u32 count + count*4  unknown
///   51 bytes               face / hair / beard / eyes / …
///   40 bytes               points, mana, curHp, STR STA CHA DEX INT AGI WIS
///   28 bytes               unknown padding
///   u32 count + count*{u32 descId, u32 points, u32 flags}   AA array
///   u32 count + count*u32  SKILLS
/// ```
fn walk_profile_skills(b: &[u8], prof: &mut PlayerProfile) {
    // MAX_KNOWN_SKILLS in the daemon headers (src/backend/*/everquest.h).
    const MAX_KNOWN_SKILLS: usize = 100;
    let len = b.len();
    let mut p: usize = 35;

    macro_rules! next_u32 {
        () => {{
            if p + 4 > len {
                return;
            }
            let v = rd_u32(b, p);
            p += 4;
            v
        }};
    }
    macro_rules! skip {
        ($n:expr) => {{
            let n = $n;
            if p.checked_add(n).map_or(true, |e| e > len) {
                return;
            }
            p += n;
        }};
    }

    let bind_count = next_u32!() as usize;
    skip!(bind_count.saturating_mul(20));

    let _deity = next_u32!();
    let _intoxication = next_u32!();

    let refresh_count = next_u32!() as usize;
    skip!(refresh_count.saturating_mul(4));

    let equip_count = next_u32!() as usize;
    skip!(equip_count.saturating_mul(20));

    let s0 = next_u32!() as usize;
    skip!(s0.saturating_mul(20));
    let s1 = next_u32!() as usize;
    skip!(s1.saturating_mul(4));
    let s2 = next_u32!() as usize;
    skip!(s2.saturating_mul(4));

    skip!(51);
    skip!(40);
    skip!(28);

    let aa_count = next_u32!() as usize;
    let aa_bytes = aa_count.saturating_mul(12);
    if p.checked_add(aa_bytes).map_or(true, |e| e > len) {
        return;
    }
    let mut aa_spent: u32 = 0;
    prof.aa_ids.reserve(aa_count);
    prof.aa_values.reserve(aa_count);
    for _ in 0..aa_count {
        let id = rd_u32(b, p);
        let val = rd_u32(b, p + 4);
        p += 12;
        aa_spent = aa_spent.wrapping_add(val);
        prof.aa_ids.push(id);
        prof.aa_values.push(val);
    }
    prof.aa_spent = aa_spent;

    let skill_count = next_u32!() as usize;
    if p.checked_add(skill_count.saturating_mul(4))
        .map_or(true, |e| e > len)
    {
        return;
    }
    let stored = skill_count.min(MAX_KNOWN_SKILLS);
    prof.skills.reserve(stored);
    for i in 0..skill_count {
        let v = rd_u32(b, p);
        p += 4;
        if i < MAX_KNOWN_SKILLS {
            prof.skills.push(v);
        }
    }
}

// C>S self position lives in `player_self_pos.rs`, which owns both the layout
// and the `PAYLOAD_LEN` the size override reads, so the gate can't disagree.

/// `OP_NewZone` (Legends) S>C, once per zone-in: the current zone as packed
/// NUL-terminated `short_name` then `long_name`, at zone-dependent offsets.
pub fn parse_new_zone(b: &[u8]) -> Result<NewZone, DecodeError> {
    // Compatibility entry point over the module parser; the implausible-name
    // error tells an opcode rotation apart from truncation.
    new_zone::parse_new_zone(b).map_err(|error| match error {
        NewZoneError::ImplausibleName(field) => DecodeError::Implausible(field),
        _ => DecodeError::Short(b.len()),
    })
}

// `OP_MobUpdate` lives in `mob_update.rs`, reading eql's own pinned
// `spawnPositionUpdate` binding.

/// Sequential reader mirroring the daemon's `NetStream`; an overrun ends the
/// walk with `BadLength` (a dropped spawn, not a crash).
struct Walk<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Walk<'a> {
    fn new(b: &'a [u8]) -> Self {
        Walk { b, p: 0 }
    }
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
/// (low 19 bits; upper 13 carry unrelated subfields).
#[inline]
fn pos19_word(w: u32) -> i16 {
    let v = w & 0x7FFFF;
    let raw = if v & 0x4_0000 != 0 {
        (v as i32) - (1 << 19)
    } else {
        v as i32
    };
    (raw >> 3) as i16
}

/// `OP_ZoneSpawns` (Legends) S>C, one per spawn — the full front walk, mirroring
/// upstream's `SpawnShell::fillSpawnStruct`. 09/01 layout from upstream 47a4992,
/// unverified on our wire:
///
/// ```text
///   +4 pad ahead of the otherData flag byte
///   curHp read in Live's slot, then a flat 37-byte skip
///     (the appearanceCount-variable block is gone)
///   color block 36 -> 40 bytes on the equipment-bearing branch
///   posData 5 words: x:19@10, y:19@74, z:19@128, world = field >> 3, no facing
///   title/suffix gated on otherData bits 4/5, replacing the tail-anchored scan
/// ```
///
/// The flag-gated tail can't absorb a miscount the way the old tail-anchored
/// scan did, so the posData word count is load-bearing.
pub fn parse_spawn(b: &[u8]) -> Result<ZoneSpawn, DecodeError> {
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
    w.skip(4)?;
    let other_data = w.u8()?;
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

    let cur_hp = w.u8()?;
    w.skip(37)?;

    let race = w.u32()?;
    let holding = w.u8()?;
    let deity = w.u32()?;
    let guild_id = w.u32()?;
    let guild_server_id = w.u32()?;
    let class_ = w.u32()?;
    let class_mask = w.u32()?; // EQL multiclass bitmask (bit N = class N)
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
        w.skip(40 + 9 * 5 * 4)?;
    } else {
        w.skip(20 + 2 * 5 * 4)?;
    }

    // Upstream names posData in the WIRE frame and transposes at its consumer
    // (`setPos(s->y>>3, s->x>>3, s->z>>3)`); resolved here so we stay map-frame.
    let p0 = w.u32()?;
    let _p1 = w.u32()?;
    let p2 = w.u32()?;
    let _p3 = w.u32()?;
    let p4 = w.u32()?;
    if std::env::var_os("SEQ_EQL_POSDATA_DUMP").is_some() {
        eprintln!("posdata {name} {id} {p0:08x} {_p1:08x} {p2:08x} {_p3:08x} {p4:08x}");
    }
    let y = pos19_word(p0 >> 10);
    let x = pos19_word(p2 >> 10);
    let z = pos19_word(p4);
    // 08/25 had a measured facing at bit 96 — re-check on the first capture.
    let heading = 0u16;

    let mut title = String::new();
    let mut suffix = String::new();
    if other_data & 0x10 != 0 {
        title = w.text()?;
    }
    if other_data & 0x20 != 0 {
        suffix = w.text()?;
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
        class_mask,
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

/// `OP_Consider` (Legends) 24B `{u32 self, u32 target, u32 faction, u32 =7, pad,
/// pad}`; level is NOT here, so the shared `Consider` carries level=0.
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

/// eql `OP_BeginCast` (S>C, 19B): spellId u32@0, casterSpawnId u16@4,
/// castTime_ms u16@6 — spell ids exceed u16, so spellId must be read as u32.
pub struct BeginCast {
    pub caster_id: u32,
    pub spell_id: u32,
    pub cast_time_ms: u32,
}

pub fn parse_begin_cast(b: &[u8]) -> Result<BeginCast, DecodeError> {
    if b.len() < 8 {
        return Err(DecodeError::Short(b.len()));
    }
    Ok(BeginCast {
        spell_id: rd_u32(b, 0),
        caster_id: rd_u16(b, 4) as u32,
        cast_time_ms: rd_u16(b, 6) as u32,
    })
}

/// eql `OP_SendAATable` (S>C): one AA ability-rank definition per packet, of
/// which only the 37B head matters — `descID`@0 and `titleSID`@13.
pub struct AaTableEntry {
    pub desc_id: u32,
    pub title_sid: u32,
}

pub fn parse_aa_table_entry(b: &[u8]) -> Result<AaTableEntry, DecodeError> {
    // Guard the whole fixed head (37B through the record's rank field); titleSID
    // sits at 13, well before the variable tail, so a fixed read reaches it.
    if b.len() < 37 {
        return Err(DecodeError::Short(b.len()));
    }
    Ok(AaTableEntry {
        desc_id: rd_u32(b, 0),
        title_sid: rd_u32(b, 13),
    })
}

/// eql OP_Stance / OP_Invocation S>C echo: a 4B `u32` ability id @0 from a
/// stable client enum. The opcode id, not the payload, picks which one.
pub fn parse_activate_ability(b: &[u8]) -> Result<u32, DecodeError> {
    if b.len() < 4 {
        Err(DecodeError::Short(b.len()))
    } else {
        Ok(rd_u32(b, 0))
    }
}

/// Payload sizes whose eql wire diverges from Live's compiled `sizeof`. EVERY
/// mapped `SZC_Match` eql opcode belongs here, or it inherits Live's gate.
pub fn size_overrides() -> Vec<(&'static str, u32)> {
    vec![
        // eql /consider is 24B both ways; Live's considerStruct is 32B.
        (
            "considerStruct",
            core::mem::size_of::<eqstructs::considerStruct>() as u32,
        ),
        // eql OP_CastSpell is a fixed 44B, not Live's 39B.
        ("startCastStruct", start_cast::PAYLOAD_LEN as u32),
        // OP_ClientUpdate S>C other-spawn position broadcast.
        ("playerSpawnPosStruct", player_spawn_pos::PAYLOAD_LEN as u32),
        // Below: sizes equal to Live's today, declared anyway so the gate is
        // eql-owned and tracks this crate's own structs if eql later diverges.
        (
            "spawnPositionUpdate",
            core::mem::size_of::<eqstructs::spawnPositionUpdate>() as u32,
        ),
        (
            "clientTargetStruct",
            core::mem::size_of::<eqstructs::clientTargetStruct>() as u32,
        ),
        (
            "manaDecrementStruct",
            core::mem::size_of::<eqstructs::manaDecrementStruct>() as u32,
        ),
        (
            "expUpdateStruct",
            core::mem::size_of::<eqstructs::expUpdateStruct>() as u32,
        ),
        (
            "skillIncStruct",
            core::mem::size_of::<eqstructs::skillIncStruct>() as u32,
        ),
        // OP_Stamina: stock 8B staminaStruct {u32 food, u32 water}.
        (
            "staminaStruct",
            core::mem::size_of::<eqstructs::staminaStruct>() as u32,
        ),
        // OP_Illusion: 336B on the 09/01 wire, unverified here (was 332).
        (
            "spawnIllusionStruct",
            core::mem::size_of::<eqstructs::spawnIllusionStruct>() as u32,
        ),
        // OP_TimeOfDay: 8B {u8 hour/min/day/month, u16 year}, no pinned binding.
        ("timeOfDayStruct", 8),
        // OP_InspectAnswer: size taken from the struct def, not a capture.
        ("inspectDataStruct", 1956),
        (
            "deleteSpawnStruct",
            core::mem::size_of::<eqstructs::deleteSpawnStruct>() as u32,
        ),
        (
            "newCorpseStruct",
            core::mem::size_of::<eqstructs::newCorpseStruct>() as u32,
        ),
        (
            "remDropStruct",
            core::mem::size_of::<eqstructs::remDropStruct>() as u32,
        ),
        (
            "action2Struct",
            core::mem::size_of::<eqstructs::action2Struct>() as u32,
        ),
        (
            "actionStruct",
            core::mem::size_of::<eqstructs::actionStruct>() as u32,
        ),
        (
            "actionAltStruct",
            core::mem::size_of::<eqstructs::actionAltStruct>() as u32,
        ),
        // OP_SimpleMessage: stock 12B {u32 eqstrId, u32 color, u32 0}.
        (
            "simpleMessageStruct",
            core::mem::size_of::<eqstructs::simpleMessageStruct>() as u32,
        ),
        // OP_ClientUpdate C>S self-pos: 42B on the 09/01 wire (was 46B).
        ("playerSelfPosStruct", player_self_pos::PAYLOAD_LEN as u32),
        ("altExpUpdateStruct", 12), // OP_AAExpUpdate: u32 altexp, u32 aaUnspent, u32 tail
        // eql door rows are 132B, Live's 136B; OP_SpawnDoor strides on this.
        ("doorStruct", spawn_door::PAYLOAD_LEN as u32),
        ("timeOfDayStruct", 8),         // OP_TimeOfDay
        ("zoneServerInfoStruct", 130),  // OP_ZoneServerInfo (world)
        ("spawnAppearance2Struct", 24), // OP_SpawnAppearance2
        // OP_SpawnAppearance carries eql's wide 24B record, not Live's 8B — an
        // inherited Live size here drops every packet.
        (
            "spawnAppearanceStruct",
            spawn_appearance::PAYLOAD_LEN as u32,
        ),
        ("inspectDataStruct", 1956), // OP_InspectAnswer
        // Stock-struct reuse, declared so the gate is eql-owned.
        ("randomStruct", 76), // OP_RandomReply (76B, l-patch addendum 3)
        ("tradeSpellBookSlotsStruct", 8), // OP_SwapSpell
        ("buffWindowSlotStruct", 12), // OP_BuffWindow
        // eql's OP_BeginCast wire is 19B, not the stock 15B beginCastStruct.
        ("beginCastStruct", 19),        // OP_BeginCast
        ("consentResponseStruct", 193), // OP_ConsentResponse + OP_DenyResponse
        ("GuildMemberUpdate", 88),      // OP_GuildMemberUpdate
        // OP_Stance + OP_Invocation share one 4B {u32 abilityId} shape.
        ("activateAbilityStruct", 4),
    ]
}

/// eql `OP_HPUpdate`, the multiplexed stat-sync channel. It is the sole
/// endurance feed — Legends has no standalone `OP_EndUpdate`.
///
/// ```text
///   /*0000*/ u32 spawnId
///   /*0004*/ u8  flags   bit0 wide | bit1 HP | bit2 mana | bit3 endurance
///                        bits4-5 reason (ignored)
///   /*0005*/     payload  per set stat bit, in bit order:
///                           wide  {i64 cur, i64 max}
///                           narrow u8 percent (max = 100)
///                [optional trailing u32, purpose unknown]
///   flags 0x31 with no stat bits is the 6s keepalive.
/// ```
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
            1 => {
                out.has_hp = true;
                out.hp_cur = cur;
                out.hp_max = max;
            }
            2 => {
                out.has_mana = true;
                out.mana_cur = cur;
                out.mana_max = max;
            }
            _ => {
                out.has_end = true;
                out.end_cur = cur;
                out.end_max = max;
            }
        }
    }
    Ok(out)
}

/// One decoded record from `OP_BuffList`. `remaining_ticks <= 0` means
/// a permanent buff (no timer). `slot` is the buff-window slot index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffListEntry {
    pub spell_id: u32,
    pub remaining_ticks: i32,
    pub slot: u32,
    /// Who cast it. A non-self owner's list mixes its own buffs with the ones
    /// the player applied, and only the caster tells them apart.
    pub caster: String,
}

/// A decoded `OP_BuffList`: the authoritative active-buff list for one
/// spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffList {
    pub spawn_id: u32,
    pub entries: Vec<BuffListEntry>,
}

/// eql `OP_BuffList` — a per-spawn active-buff list with real server-side
/// remaining durations, sent at zone-in and on every buff change.
///
/// ```text
/// u32 spawnId | u32 (seq/timestamp) | u8 flag=1 | u8 count | u8[5] pad
/// then `count` records, each:
///   { u32 spellId, u32 =1, i32 remainingTicks, u32 =0 }   (16 fixed bytes)
///   + NUL-terminated caster name (latin1; empty for self-cast)
///   + slot: u32 BETWEEN records, u16 on the FINAL record.
/// ```
///
/// `remainingTicks <= 0` is permanent, and the cursor landing exactly on the
/// packet end is the structural canary.
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
        // NUL-terminated caster name.
        let caster = match b[pos..].iter().position(|&c| c == 0) {
            Some(off) => {
                let s = String::from_utf8_lossy(&b[pos..pos + off]).into_owned();
                pos += off + 1;
                s
            }
            None => return Err(DecodeError::BadLength(b.len())),
        };
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
        entries.push(BuffListEntry {
            spell_id,
            remaining_ticks,
            slot,
            caster,
        });
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

    #[test]
    fn profile_reads_base_stats() {
        // Seven DISTINCT values, so a permuted field mapping cannot pass.
        let mut b = vec![0u8; 990];
        for (i, v) in [11u32, 22, 33, 44, 55, 66, 77].iter().enumerate() {
            b[962 + 4 * i..966 + 4 * i].copy_from_slice(&v.to_le_bytes());
        }
        let p = parse_player_profile(&b).unwrap();
        // Wire order is STR/STA/CHA/DEX/INT/AGI/WIS.
        assert_eq!(
            (p.str_, p.sta, p.cha, p.dex, p.int_, p.agi, p.wis),
            (11, 22, 33, 44, 55, 66, 77)
        );
    }

    #[test]
    fn profile_locates_the_spellbook_by_signature() {
        // Junk whose second word is > 4096 in front, then a real 8B-record book.
        let mut b = vec![0xAAu8; 2048];
        let book = b.len();
        for k in 0..SPELLBOOK_SLOTS {
            let (id, word): (u32, u32) = if k < 100 {
                (1000 + k as u32, (k % 60) as u32)
            } else {
                (0xffff_ffff, 0)
            };
            b.extend_from_slice(&id.to_le_bytes());
            b.extend_from_slice(&word.to_le_bytes());
        }
        assert_eq!(find_profile_spellbook(&b), Some(book));
        let p = parse_player_profile(&b).unwrap();
        assert_eq!(p.spell_book.len(), SPELLBOOK_SLOTS);
        assert_eq!(p.spell_book[0], 1000);
        assert_eq!(p.spell_book[99], 1099);
        assert_eq!(p.spell_book[100], -1); // 0xffffffff = empty slot
    }

    #[test]
    fn profile_without_a_spellbook_leaves_it_empty() {
        // A zero-filled profile has no plausible book (id 0 is not a spell id).
        let p = parse_player_profile(&vec![0u8; 8000]).unwrap();
        assert!(p.spell_book.is_empty());
    }

    #[test]
    fn profile_short_buffer_leaves_base_stats_zero() {
        // One byte short of the block: read is skipped, not partial, no panic.
        let p = parse_player_profile(&vec![0u8; 989]).unwrap();
        assert_eq!(
            (p.str_, p.sta, p.cha, p.dex, p.int_, p.agi, p.wis),
            (0, 0, 0, 0, 0, 0, 0)
        );
    }

    /// Synthetic profile: identity header, zeroed junk (no 0x40, so no false
    /// name-block match), then the name block and the positional tail.
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
        // Money is read from fixed offsets now, so this short profile has none.
        assert_eq!(p.platinum, 0);
        assert_eq!(p.copper, 0);
    }

    #[test]
    fn money_at_fixed_offsets() {
        // Carried and cursor are fixed-offset; the bank is located by scanning
        // for the inventory MIRROR of the carried quadruple.
        let mut b = vec![0u8; 40000];
        let put = |b: &mut Vec<u8>, off: usize, v: u32| {
            b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        };
        for (i, v) in [9275u32, 10, 25, 47].iter().enumerate() {
            put(&mut b, 33687 + i * 4, *v); // carried
            put(&mut b, 36245 + i * 4, *v); // inventory mirror
        }
        for (i, v) in [1234u32, 5, 3, 5].iter().enumerate() {
            put(&mut b, 36261 + i * 4, *v); // bank, right after the mirror
        }
        let p = parse_player_profile(&b).unwrap();
        assert_eq!((p.platinum, p.gold, p.silver, p.copper), (9275, 10, 25, 47));
        assert_eq!(
            (p.platinum_bank, p.gold_bank, p.silver_bank, p.copper_bank),
            (1234, 5, 3, 5)
        );
        // Nothing on the cursor in this fixture.
        assert_eq!((p.platinum_cursor, p.copper_cursor), (0, 0));
    }

    #[test]
    fn empty_purse_leaves_bank_unread() {
        // An all-zero carried quadruple would match padding anywhere, so the
        // mirror scan is skipped rather than latching onto the first zeros.
        let b = vec![0u8; 40000];
        let p = parse_player_profile(&b).unwrap();
        assert_eq!((p.platinum, p.platinum_bank), (0, 0));
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
        // OP_NewZone layout: short, long, zone file, then environment.
        let mut b = Vec::new();
        b.extend_from_slice(b"guktop\0");
        b.extend_from_slice(b"The City of Guk\0");
        b.extend_from_slice(&[0u8; 3]);
        b.extend_from_slice(b"guktop.eqg\0");
        let mut tail = [0u8; 306];
        tail[5..9].copy_from_slice(&1u32.to_le_bytes());
        tail[9..13].copy_from_slice(&1.0f32.to_le_bytes());
        tail[123..127].copy_from_slice(&2.0f32.to_le_bytes());
        tail[127..131].copy_from_slice(&1.0f32.to_le_bytes());
        tail[131..135].copy_from_slice(&3.0f32.to_le_bytes());
        b.extend_from_slice(&tail);
        let z = parse_new_zone(&b).unwrap();
        assert_eq!(z.short_name, "guktop");
        assert_eq!(z.long_name, "The City of Guk");
        assert_eq!(z.zonefile, "guktop.eqg");
    }

    #[test]
    fn new_zone_rejects_unterminated() {
        assert!(parse_new_zone(b"noterminator").is_err());
        assert!(parse_new_zone(b"short\0").is_err()); // no long name
    }

    #[test]
    fn new_zone_rejects_names_of_raw_bytes() {
        // Shape World actually latched live: latin1 turns any byte into a valid
        // String, so the plausibility gate is the only thing that can reject it.
        let mut b = vec![0x5c, 0xfa, 0x27, 0x1b, 0xf2, 0xc2, 0xcd, 0x83, 0x00];
        b.extend_from_slice(b"The Plane of Hate\0");
        assert_eq!(
            parse_new_zone(&b),
            Err(DecodeError::Implausible("short_name"))
        );
    }

    #[test]
    fn new_zone_reads_an_instanced_zone() {
        let mut b = Vec::new();
        b.extend_from_slice(b"hateplane_eqlsolo\0");
        b.extend_from_slice(b"The Plane of Hate\0");
        b.extend_from_slice(&[0u8; 3]);
        b.extend_from_slice(b"hateplane.eqg\0");
        let mut tail = [0u8; 306];
        tail[5..9].copy_from_slice(&1u32.to_le_bytes());
        tail[9..13].copy_from_slice(&1.0f32.to_le_bytes());
        tail[123..127].copy_from_slice(&0.0f32.to_le_bytes());
        tail[127..131].copy_from_slice(&0.0f32.to_le_bytes());
        tail[131..135].copy_from_slice(&0.0f32.to_le_bytes());
        b.extend_from_slice(&tail);
        let z = parse_new_zone(&b).unwrap();
        assert_eq!(z.short_name, "hateplane_eqlsolo");
        assert_eq!(z.long_name, "The Plane of Hate");
    }

    // parse_player_self_pos tests live in player_self_pos.rs (the module now owns
    // the canonical parser; these lib.rs copies tested the retired 42B layout).

    /// Encode a game-unit coordinate as a wire position word: signed 19-bit
    /// ×8 fixed-point in the low bits.
    fn pos19(game_units: i32) -> u32 {
        ((game_units * 8) as u32) & 0x7FFFF
    }

    /// A full eql zone-spawn payload on the NPC / non-humanoid path (npc=1,
    /// race>12); the 9-slot humanoid branch is exercised by the goldens.
    #[allow(clippy::too_many_arguments)]
    fn build_spawn(
        name: &str,
        id: u32,
        level: u8,
        cur_hp: u8,
        race: u32,
        deity: u32,
        class_: u32,
        z: i32,
        y: i32,
        x: i32,
        last: &str,
        title: &str,
        suffix: &str,
    ) -> Vec<u8> {
        let mut b = Vec::new();
        let text = |b: &mut Vec<u8>, s: &str| {
            b.extend_from_slice(s.as_bytes());
            b.push(0);
        };
        let u32le = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
        let has_title = !title.is_empty();
        let has_suffix = !suffix.is_empty();
        text(&mut b, name);
        u32le(&mut b, id);
        b.push(level);
        b.extend_from_slice(&[0u8; 16]);
        b.push(1); // npc
        u32le(&mut b, 0); // miscData
        b.extend_from_slice(&[0u8; 4]); // 09/01 pad ahead of the flag byte
        b.push(u8::from(has_title) << 4 | u8::from(has_suffix) << 5); // otherData
        b.extend_from_slice(&[0u8; 8]);
        b.push(0); // charProperties = 0 (no bodytype loop)
        b.push(cur_hp);
        b.extend_from_slice(&[0u8; 37]);
        u32le(&mut b, race);
        b.push(0); // holding
        u32le(&mut b, deity);
        u32le(&mut b, 0); // guildID
        u32le(&mut b, 0); // guildServerID
        u32le(&mut b, class_);
        u32le(&mut b, 0); // classMask
        b.extend_from_slice(&[0u8; 4]); // skip1, state, light, skip1
        text(&mut b, last); // lastName
        b.extend_from_slice(&[0u8; 2]);
        u32le(&mut b, 0); // petOwnerId
        b.extend_from_slice(&[0u8; 49]); // npc==1 extra
        b.extend_from_slice(&[0u8; 60]); // equipment (else branch: 20 + 2*5*4)
                                         // Every bit outside the three coordinate fields is set, so a
                                         // slipped read gets a wrong number, not a plausible zero.
        u32le(&mut b, 0x3FF | (pos19(y) << 10) | (0x7u32 << 29)); // p0: pad10 | wire x | pad3
        u32le(&mut b, 0xFFFF_FFFF); // p1: pad
        u32le(&mut b, 0x3FF | (pos19(x) << 10) | (0x7u32 << 29)); // p2: pad10 | wire y | pad3
        u32le(&mut b, 0xFFFF_FFFF); // p3: pad
        u32le(&mut b, pos19(z) | (0x1FFFu32 << 19)); // p4: Z in the low 19, rest set
        if has_title {
            text(&mut b, title);
        }
        if has_suffix {
            text(&mut b, suffix);
        }
        b.extend_from_slice(&[0u8; 8]); // unknowns
        b.push(0); // isMercenary
        b.extend_from_slice(&[0u8; 70]); // unknowns
        b
    }

    #[test]
    fn spawn_full_walk_reads_all_fields() {
        let b = build_spawn(
            "a guard",
            4242,
            55,
            90,
            14,
            396,
            3,
            80,
            -15,
            10,
            "",
            "Protector",
            "of Qeynos",
        );
        let s = parse_spawn(&b).unwrap();
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
        // 09/01: posData carries no facing, so the surrounding pad bits (all set
        // here) must not leak into it.
        assert_eq!(s.heading, 0);
        assert_eq!(s.title, "Protector");
        assert_eq!(s.suffix, "of Qeynos");
    }

    #[test]
    fn spawn_last_name_and_position_past_i16_window() {
        // far spawn (|y·8| > i16::MAX) must not wrap; surname decodes; no title.
        let b = build_spawn(
            "Grarf",
            7,
            60,
            100,
            14,
            0,
            5,
            12,
            -4700,
            5200,
            "Ironforge",
            "",
            "",
        );
        let s = parse_spawn(&b).unwrap();
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
        assert!(parse_spawn(&b).is_err());
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
        assert_eq!(
            list.entries[0],
            BuffListEntry {
                spell_id: 278,
                remaining_ticks: 5,
                slot: 1,
                caster: String::new()
            }
        );
        assert_eq!(
            list.entries[1],
            BuffListEntry {
                spell_id: 515,
                remaining_ticks: -1,
                slot: 7,
                caster: "X".into()
            }
        );
        // Truncation / trailing-garbage both fail the cursor-lands-on-end canary.
        assert!(parse_buff_list(&b[..b.len() - 1]).is_err());
        let mut extra = b.clone();
        extra.push(0xAA);
        assert!(parse_buff_list(&extra).is_err());
    }
}
