//! Parser for `OP_PlayerProfile` (live, modern). Mirrors
//! `ZoneMgr::fillProfileStruct` in `zonemgr.cpp`: a NetStream walk
//! over a long, variable-length payload that fills `charProfileStruct`
//! / `playerProfileStruct`.
//!
//! Byte-order conventions match `NetStream`:
//!   * `*_NC` (no-convert) readers are little-endian. The legacy code
//!     calls these `readUInt8` (just one byte), `readUInt16NC`, and
//!     `readUInt32NC`.
//!   * Non-`NC` readers (`readUInt16`, `readUInt32`, `readInt32`) are
//!     big-endian — used for `standState`, `anon`, and the
//!     spellbook/mem-spell/refresh-timer integers.
//!   * `readText()` reads a NUL-terminated string (advances past the
//!     NUL).
//!
//! The parser exposes only the fields the daemon's downstream
//! consumers actually read (Player::loadProfile, Player::player,
//! MessageShell::player, SpellShell::buffLoad). Everything else is
//! skipped — but the cursor still advances through every section so
//! the final `bytes_consumed` matches the C++ parser's tally.

use thiserror::Error;

const SPELL_BUFF_SIZE: usize = 110;
const BIND_STRUCT_SIZE: usize = 20;
const EQUIP_STRUCT_SIZE: usize = 20;
const TRIBUTE_STRUCT_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerProfile {
    pub bytes_consumed: u32,
    pub checksum: u32,

    // profile.*
    pub gender: u8,
    pub race: u32,
    pub class_: u32,
    pub class_mask: u32,
    // eql-only combat state (read at a fixed profile offset on eql); live/test
    // have no swappable stance/invocation, so these stay 0 — same pattern as
    // class_mask. Present here so the shared decode_player_profile FFI (which maps
    // p.stance/p.invocation) compiles for every backend.
    pub stance: u32,
    pub invocation: u32,
    pub level: u8,
    pub level1: u8,
    pub bind0_zone_id: u32,
    pub bind0_x: f32,
    pub bind0_y: f32,
    pub bind0_z: f32,
    pub bind0_heading: f32,
    pub deity: u32,
    pub intoxication: u32,
    pub points: u32,
    pub mana: u32,
    pub cur_hp: u32,
    pub str_: u32,
    pub sta: u32,
    pub cha: u32,
    pub dex: u32,
    pub int_: u32,
    pub agi: u32,
    pub wis: u32,
    pub aa_ids: Vec<u32>,
    pub aa_values: Vec<u32>,
    /// Player skill values (eql populates these via its own profile walk; the
    /// live/test walk leaves it empty). Kept field-symmetric with the eql
    /// `PlayerProfile` so the shared `seq-bridge` mapping compiles for all backends.
    pub skills: Vec<u32>,
    pub disciplines: Vec<u32>,
    pub recast_timers: Vec<u32>,
    pub spell_book: Vec<i32>,
    pub mem_spells: Vec<i32>,
    pub spell_slot_refresh: Vec<u32>,
    pub buff_spell_ids: Vec<i32>,
    pub buff_durations: Vec<i32>,
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub copper: u32,
    pub platinum_cursor: u32,
    pub gold_cursor: u32,
    pub silver_cursor: u32,
    pub copper_cursor: u32,
    pub aa_spent: u32,
    pub aa_assigned: u32,
    pub aa_unspent: u32,
    pub endurance: u32,
    pub exp_aa: u32,

    // charProfileStruct top-level
    pub name: String,
    pub last_name: String,
    pub birthday_time: u32,
    pub account_create_date: u32,
    pub last_save_time: u32,
    pub time_played_min: u32,
    pub expansions: u32,
    pub languages: Vec<u8>,
    pub zone_id: u16,
    pub zone_instance: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub stand_state: u16,
    pub anon: u16,
    pub guild_id: u32,
    pub guild_server_id: u32,
    pub platinum_inventory: u32,
    pub gold_inventory: u32,
    pub silver_inventory: u32,
    pub copper_inventory: u32,
    pub platinum_bank: u32,
    pub gold_bank: u32,
    pub silver_bank: u32,
    pub copper_bank: u32,
    pub platinum_shared: u32,
    pub career_tribute: u32,
    pub current_tribute: u32,
    pub current_rad_crystals: u32,
    pub career_rad_crystals: u32,
    pub current_ebon_crystals: u32,
    pub career_ebon_crystals: u32,
    pub autosplit: u8,
    pub ldon_guk_points: u32,
    pub ldon_mir_points: u32,
    pub ldon_mmc_points: u32,
    pub ldon_ruj_points: u32,
    pub ldon_tak_points: u32,
    pub ldon_avail_points: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlayerProfileError {
    #[error("payload truncated at byte {0}, needed {1} more")]
    Truncated(usize, usize),
    #[error("section {0} declared {1} elements, would exceed payload")]
    OverlongSection(&'static str, u32),
}

struct R<'a> {
    bytes: &'a [u8],
    p: usize,
}

impl<'a> R<'a> {
    fn need(&self, n: usize) -> Result<(), PlayerProfileError> {
        if self.bytes.len() < self.p + n {
            Err(PlayerProfileError::Truncated(self.p, n))
        } else {
            Ok(())
        }
    }
    fn skip(&mut self, n: usize) -> Result<(), PlayerProfileError> {
        self.need(n)?;
        self.p += n;
        Ok(())
    }
    fn u8(&mut self) -> Result<u8, PlayerProfileError> {
        self.need(1)?;
        let v = self.bytes[self.p];
        self.p += 1;
        Ok(v)
    }
    fn u16_le(&mut self) -> Result<u16, PlayerProfileError> {
        self.need(2)?;
        let v = u16::from_le_bytes(self.bytes[self.p..self.p + 2].try_into().unwrap());
        self.p += 2;
        Ok(v)
    }
    fn u16_be(&mut self) -> Result<u16, PlayerProfileError> {
        self.need(2)?;
        let v = u16::from_be_bytes(self.bytes[self.p..self.p + 2].try_into().unwrap());
        self.p += 2;
        Ok(v)
    }
    fn u32_le(&mut self) -> Result<u32, PlayerProfileError> {
        self.need(4)?;
        let v = u32::from_le_bytes(self.bytes[self.p..self.p + 4].try_into().unwrap());
        self.p += 4;
        Ok(v)
    }
    fn i32_be(&mut self) -> Result<i32, PlayerProfileError> {
        self.need(4)?;
        let v = i32::from_be_bytes(self.bytes[self.p..self.p + 4].try_into().unwrap());
        self.p += 4;
        Ok(v)
    }
    fn f32_le(&mut self) -> Result<f32, PlayerProfileError> {
        self.need(4)?;
        let v = f32::from_le_bytes(self.bytes[self.p..self.p + 4].try_into().unwrap());
        self.p += 4;
        Ok(v)
    }
    /// NUL-terminated text; advances past the terminator. Mirrors
    /// `NetStream::readText()`.
    fn text(&mut self) -> Result<String, PlayerProfileError> {
        if self.p >= self.bytes.len() {
            return Ok(String::new());
        }
        let start = self.p;
        while self.p < self.bytes.len() && self.bytes[self.p] != 0 {
            self.p += 1;
        }
        let s = String::from_utf8_lossy(&self.bytes[start..self.p]).into_owned();
        if self.p < self.bytes.len() {
            self.p += 1; // consume NUL
        }
        Ok(s)
    }
    /// Mirrors the daemon's name/lastName read pattern:
    ///   u32 length prefix LE, then `length` bytes follow on the wire.
    /// The C side copies a fixed-width buffer (64 / 32) regardless of
    /// `length` and lets the destination be NUL-padded; we just take
    /// the first NUL-terminated string out of the `length` bytes.
    fn length_prefixed_name(
        &mut self,
        cap: usize,
    ) -> Result<String, PlayerProfileError> {
        let len = self.u32_le()? as usize;
        self.need(len)?;
        let span = &self.bytes[self.p..self.p + len];
        let take = cap.min(len);
        let end = span[..take]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(take);
        let s = String::from_utf8_lossy(&span[..end]).into_owned();
        self.p += len;
        Ok(s)
    }
}

pub fn parse_player_profile(bytes: &[u8]) -> Result<PlayerProfile, PlayerProfileError> {
    let mut r = R { bytes, p: 0 };

    let checksum = r.u32_le()?;
    r.skip(16)?;

    // --- profile.* ---
    let gender = r.u8()?;
    let race = r.u32_le()?;
    let class_ = r.u32_le()?;
    let level = r.u8()?;
    let level1 = r.u8()?;

    // Bind points: u32 count, then count * 20-byte BindStruct.
    // BindStruct = {u32 zoneId, f32 x, f32 y, f32 z, f32 heading}.
    let bind_count = r.u32_le()?;
    let mut bind0_zone_id = 0u32;
    let (mut bind0_x, mut bind0_y, mut bind0_z, mut bind0_heading) =
        (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for i in 0..bind_count {
        if i == 0 {
            bind0_zone_id = r.u32_le()?;
            bind0_x = r.f32_le()?;
            bind0_y = r.f32_le()?;
            bind0_z = r.f32_le()?;
            bind0_heading = r.f32_le()?;
        } else {
            r.skip(BIND_STRUCT_SIZE)?;
        }
    }

    let deity = r.u32_le()?;
    let intoxication = r.u32_le()?;

    // Spell slot refresh (first pass — overwritten later)
    //
    // Every count below is read off the wire, so it is attacker/patch
    // controlled: after an opcode rotation a stale id feeds this parser a
    // FOREIGN payload, and an unbounded reservation then tries to allocate
    // gigabytes and aborts the process. An element occupies at least one byte,
    // so the payload length is always a valid ceiling. This caps the
    // reservation only — each walk still errors on truncation, so valid
    // payloads decode exactly as before.
    let refresh_count = r.u32_le()?;
    let mut spell_slot_refresh = Vec::with_capacity((refresh_count as usize).min(r.bytes.len()));
    for _ in 0..refresh_count {
        spell_slot_refresh.push(r.u32_le()?);
    }

    // Equipment (skipped: spawn opcode carries the same data)
    let equip_count = r.u32_le()?;
    r.skip(equip_count as usize * EQUIP_STRUCT_SIZE)?;

    // Three unknown sections — skip in lock-step with the C++ parser.
    let sc0 = r.u32_le()?;
    r.skip(sc0 as usize * 20)?;
    let sc1 = r.u32_le()?;
    r.skip(sc1 as usize * 4)?;
    let sc2 = r.u32_le()?;
    r.skip(sc2 as usize * 4)?;

    // Face/hair/etc.
    r.skip(51)?;

    let points = r.u32_le()?;
    let mana = r.u32_le()?;
    let cur_hp = r.u32_le()?;
    let str_ = r.u32_le()?;
    let sta = r.u32_le()?;
    let cha = r.u32_le()?;
    let dex = r.u32_le()?;
    let int_ = r.u32_le()?;
    let agi = r.u32_le()?;
    let wis = r.u32_le()?;

    r.skip(28)?;

    // AAs: u32 count, then count * {u32 AA, u32 value, u32 unknown008}.
    let aa_count = r.u32_le()?;
    let mut aa_ids = Vec::with_capacity((aa_count as usize).min(r.bytes.len()));
    let mut aa_values = Vec::with_capacity((aa_count as usize).min(r.bytes.len()));
    for _ in 0..aa_count {
        aa_ids.push(r.u32_le()?);
        aa_values.push(r.u32_le()?);
        r.skip(4)?; // unknown008
    }

    // Skills — the C++ parser explicitly skips this section. The real
    // skills array is overlaid on the C++ side from a fixed wire offset
    // (zonemgr.cpp::zonePlayer). We mirror the skip here.
    let skills_count = r.u32_le()?;
    r.skip(skills_count as usize * 4)?;

    let sc3 = r.u32_le()?;
    r.skip(sc3 as usize * 4)?;

    let discipline_count = r.u32_le()?;
    let mut disciplines = Vec::with_capacity((discipline_count as usize).min(r.bytes.len()));
    for _ in 0..discipline_count {
        disciplines.push(r.u32_le()?);
    }

    let sc4 = r.u32_le()?;
    r.skip(sc4 as usize * 4)?;

    r.skip(4)?;

    let recast_count = r.u32_le()?;
    let mut recast_timers = Vec::with_capacity((recast_count as usize).min(r.bytes.len()));
    for _ in 0..recast_count {
        recast_timers.push(r.u32_le()?);
    }

    let sc5 = r.u32_le()?;
    r.skip(sc5 as usize * 4)?;

    // Spellbook (legacy uses readInt32 = BE).
    let book_count = r.u32_le()?;
    let mut spell_book = Vec::with_capacity((book_count as usize).min(r.bytes.len()));
    for _ in 0..book_count {
        spell_book.push(r.i32_be()?);
    }

    // Memorized spell slots (BE).
    let mem_count = r.u32_le()?;
    let mut mem_spells = Vec::with_capacity((mem_count as usize).min(r.bytes.len()));
    for _ in 0..mem_count {
        mem_spells.push(r.i32_be()?);
    }

    // Spell slot refresh timers — overwrites the earlier array. Legacy
    // reads these BE (`readInt32`) and writes them into the same
    // `spellSlotRefresh` u32 slots.
    let refresh2_count = r.u32_le()?;
    let mut spell_slot_refresh2 = Vec::with_capacity((refresh2_count as usize).min(r.bytes.len()));
    for _ in 0..refresh2_count {
        spell_slot_refresh2.push(r.i32_be()? as u32);
    }
    if !spell_slot_refresh2.is_empty() {
        spell_slot_refresh = spell_slot_refresh2;
    }

    r.skip(1)?;

    // Buffs: u32 count, then count * 110-byte spellBuff. We pull out
    // just `spellid` (offset 21, i32 LE inside the buff per
    // everquest.h's `#pragma pack(1)` layout) and `duration` (offset
    // 12, i32 LE). The rest of the slot is left for the C++ side to
    // memcpy if it ever needs more.
    let buff_count = r.u32_le()?;
    let mut buff_spell_ids = Vec::with_capacity((buff_count as usize).min(r.bytes.len()));
    let mut buff_durations = Vec::with_capacity((buff_count as usize).min(r.bytes.len()));
    for _ in 0..buff_count {
        r.need(SPELL_BUFF_SIZE)?;
        let buff = &r.bytes[r.p..r.p + SPELL_BUFF_SIZE];
        let duration = i32::from_le_bytes(buff[12..16].try_into().unwrap());
        let spellid = i32::from_le_bytes(buff[21..25].try_into().unwrap());
        buff_spell_ids.push(spellid);
        buff_durations.push(duration);
        r.p += SPELL_BUFF_SIZE;
    }

    let platinum = r.u32_le()?;
    let gold = r.u32_le()?;
    let silver = r.u32_le()?;
    let copper = r.u32_le()?;

    let platinum_cursor = r.u32_le()?;
    let gold_cursor = r.u32_le()?;
    let silver_cursor = r.u32_le()?;
    let copper_cursor = r.u32_le()?;

    r.skip(20)?;
    let aa_spent = r.u32_le()?;
    r.skip(4)?;
    let aa_assigned = r.u32_le()?;
    r.skip(20)?;
    let aa_unspent = r.u32_le()?;
    r.skip(2)?;

    // Bandoliers — skipped (no downstream consumer). Each entry is
    // 5 NUL-terminated strings + 8 u32s.
    let bando_count = r.u32_le()?;
    for _ in 0..bando_count {
        // bandolierName
        let _ = r.text()?;
        // 4 slots, each: name + u32 itemId + u32 icon
        for _ in 0..4 {
            let _ = r.text()?;
            r.skip(8)?;
        }
    }

    r.skip(80)?;
    let endurance = r.u32_le()?;

    r.skip(58)?;
    let exp_aa = r.u32_le()?;
    r.skip(8)?;

    let name = r.length_prefixed_name(64)?;
    let last_name = r.length_prefixed_name(32)?;

    let birthday_time = r.u32_le()?;
    let account_create_date = r.u32_le()?;
    let last_save_time = r.u32_le()?;
    let time_played_min = r.u32_le()?;
    r.skip(4)?;
    let expansions = r.u32_le()?;
    r.skip(4)?;

    let lang_count = r.u32_le()?;
    let mut languages = Vec::with_capacity((lang_count as usize).min(r.bytes.len()));
    for _ in 0..lang_count {
        languages.push(r.u8()?);
    }

    let zone_id = r.u16_le()?;
    let zone_instance = r.u16_le()?;

    // Position fields are memcpy'd as raw f32 LE.
    let y = r.f32_le()?;
    let x = r.f32_le()?;
    let z = r.f32_le()?;
    let heading = r.f32_le()?;

    // standState and anon are read with readUInt16 (BE).
    let stand_state = r.u16_be()?;
    let anon = r.u16_be()?;

    let guild_id = r.u32_le()?;
    let guild_server_id = r.u32_le()?;

    r.skip(2)?;
    let platinum_inventory = r.u32_le()?;
    let gold_inventory = r.u32_le()?;
    let silver_inventory = r.u32_le()?;
    let copper_inventory = r.u32_le()?;
    let platinum_bank = r.u32_le()?;
    let gold_bank = r.u32_le()?;
    let silver_bank = r.u32_le()?;
    let copper_bank = r.u32_le()?;
    let platinum_shared = r.u32_le()?;

    // Unknown count*8b section.
    let sc6 = r.u32_le()?;
    r.skip(sc6 as usize * 8)?;

    r.skip(8)?;
    let career_tribute = r.u32_le()?;
    r.skip(4)?;
    let current_tribute = r.u32_le()?;
    r.skip(6)?;

    // Tributes — skipped (no downstream consumer).
    let tribute_count = r.u32_le()?;
    r.skip(tribute_count as usize * TRIBUTE_STRUCT_SIZE)?;

    let sc7 = r.u32_le()?;
    r.skip(sc7 as usize * 8)?;

    r.skip(137)?;

    let current_rad_crystals = r.u32_le()?;
    let career_rad_crystals = r.u32_le()?;
    let current_ebon_crystals = r.u32_le()?;
    let career_ebon_crystals = r.u32_le()?;

    r.skip(91)?;
    let autosplit = r.u8()?;
    r.skip(57)?;

    let ldon_guk_points = r.u32_le()?;
    let ldon_mir_points = r.u32_le()?;
    let ldon_mmc_points = r.u32_le()?;
    let ldon_ruj_points = r.u32_le()?;
    let ldon_tak_points = r.u32_le()?;
    let ldon_avail_points = r.u32_le()?;

    Ok(PlayerProfile {
        bytes_consumed: r.p as u32,
        checksum,
        gender,
        race,
        class_,
        class_mask: 0, // live isn't multiclass; eql fills its own bitmask
        stance: 0,     // eql-only; live/test have no swappable stance/invocation
        invocation: 0,
        level,
        level1,
        bind0_zone_id,
        bind0_x,
        bind0_y,
        bind0_z,
        bind0_heading,
        deity,
        intoxication,
        points,
        mana,
        cur_hp,
        str_,
        sta,
        cha,
        dex,
        int_,
        agi,
        wis,
        aa_ids,
        aa_values,
        skills: Vec::new(), // eql-only; live surfaces skills via loadProfile's own path
        disciplines,
        recast_timers,
        spell_book,
        mem_spells,
        spell_slot_refresh,
        buff_spell_ids,
        buff_durations,
        platinum,
        gold,
        silver,
        copper,
        platinum_cursor,
        gold_cursor,
        silver_cursor,
        copper_cursor,
        aa_spent,
        aa_assigned,
        aa_unspent,
        endurance,
        exp_aa,
        name,
        last_name,
        birthday_time,
        account_create_date,
        last_save_time,
        time_played_min,
        expansions,
        languages,
        zone_id,
        zone_instance,
        x,
        y,
        z,
        heading,
        stand_state,
        anon,
        guild_id,
        guild_server_id,
        platinum_inventory,
        gold_inventory,
        silver_inventory,
        copper_inventory,
        platinum_bank,
        gold_bank,
        silver_bank,
        copper_bank,
        platinum_shared,
        career_tribute,
        current_tribute,
        current_rad_crystals,
        career_rad_crystals,
        current_ebon_crystals,
        career_ebon_crystals,
        autosplit,
        ldon_guk_points,
        ldon_mir_points,
        ldon_mmc_points,
        ldon_ruj_points,
        ldon_tak_points,
        ldon_avail_points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal-but-complete payload: all variable sections set
    /// to length 0 except the ones we want to exercise. The C++ parser
    /// happily walks a zero-count section, so this validates the
    /// skeleton of the NetStream walk.
    fn skeleton() -> Vec<u8> {
        let mut b: Vec<u8> = Vec::new();
        // checksum
        b.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());
        // unknown 16
        b.extend_from_slice(&[0u8; 16]);
        // gender u8
        b.push(0);
        // race u32
        b.extend_from_slice(&1u32.to_le_bytes());
        // class_ u32
        b.extend_from_slice(&2u32.to_le_bytes());
        // level + level1
        b.push(60);
        b.push(60);
        // bind count = 1, then one BindStruct
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&123u32.to_le_bytes()); // zoneId
        b.extend_from_slice(&(-100.0f32).to_le_bytes()); // x
        b.extend_from_slice(&200.0f32.to_le_bytes()); // y
        b.extend_from_slice(&(-50.0f32).to_le_bytes()); // z
        b.extend_from_slice(&0.5f32.to_le_bytes()); // heading
        // deity, intoxication
        b.extend_from_slice(&201u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // refresh_count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // equip_count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // sc0, sc1, sc2
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // 51b skip
        b.extend_from_slice(&[0u8; 51]);
        // points, MANA, curHp, STR, STA, CHA, DEX, INT, AGI, WIS
        b.extend_from_slice(&5u32.to_le_bytes()); // points
        b.extend_from_slice(&1234u32.to_le_bytes()); // MANA
        b.extend_from_slice(&5678u32.to_le_bytes()); // curHp
        for v in [100, 110, 120, 130, 140, 150, 160u32] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // 28b skip
        b.extend_from_slice(&[0u8; 28]);
        // aa_count = 2
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&500u32.to_le_bytes()); // AA
        b.extend_from_slice(&3u32.to_le_bytes()); // value
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown008
        b.extend_from_slice(&501u32.to_le_bytes());
        b.extend_from_slice(&5u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // skills_count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // sc3
        b.extend_from_slice(&0u32.to_le_bytes());
        // discipline_count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // sc4
        b.extend_from_slice(&0u32.to_le_bytes());
        // 4b skip
        b.extend_from_slice(&[0u8; 4]);
        // recast_count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // sc5
        b.extend_from_slice(&0u32.to_le_bytes());
        // spellbook count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // mem spells count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // refresh2 count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // 1b skip
        b.push(0);
        // buff count = 1, write a buff with spellid=42, duration=600
        b.extend_from_slice(&1u32.to_le_bytes());
        let mut buff = [0u8; SPELL_BUFF_SIZE];
        buff[12..16].copy_from_slice(&600i32.to_le_bytes()); // duration
        buff[21..25].copy_from_slice(&42i32.to_le_bytes()); // spellid
        b.extend_from_slice(&buff);
        // money on player
        for v in [10u32, 20, 30, 40] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // money on cursor
        for v in [1u32, 2, 3, 4] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // 20b skip
        b.extend_from_slice(&[0u8; 20]);
        // aa_spent
        b.extend_from_slice(&111u32.to_le_bytes());
        // 4b skip
        b.extend_from_slice(&[0u8; 4]);
        // aa_assigned
        b.extend_from_slice(&222u32.to_le_bytes());
        // 20b skip
        b.extend_from_slice(&[0u8; 20]);
        // aa_unspent
        b.extend_from_slice(&333u32.to_le_bytes());
        // 2b skip
        b.extend_from_slice(&[0u8; 2]);
        // bandolier count = 0
        b.extend_from_slice(&0u32.to_le_bytes());
        // 80b skip
        b.extend_from_slice(&[0u8; 80]);
        // endurance
        b.extend_from_slice(&444u32.to_le_bytes());
        // 58b skip
        b.extend_from_slice(&[0u8; 58]);
        // expAA
        b.extend_from_slice(&22846u32.to_le_bytes());
        // 8b skip
        b.extend_from_slice(&[0u8; 8]);
        // name: u32 len=8, then 8 bytes "PlayerXX\0"
        b.extend_from_slice(&8u32.to_le_bytes());
        b.extend_from_slice(b"Player\0\0");
        // lastName: u32 len=8, then 8 bytes "Last\0\0\0\0"
        b.extend_from_slice(&8u32.to_le_bytes());
        b.extend_from_slice(b"Last\0\0\0\0");
        // birthdayTime, accountCreateDate, lastSaveTime, timePlayedMin
        b.extend_from_slice(&100u32.to_le_bytes());
        b.extend_from_slice(&200u32.to_le_bytes());
        b.extend_from_slice(&300u32.to_le_bytes());
        b.extend_from_slice(&400u32.to_le_bytes());
        // 4b skip
        b.extend_from_slice(&[0u8; 4]);
        // expansions
        b.extend_from_slice(&0xFFu32.to_le_bytes());
        // 4b skip
        b.extend_from_slice(&[0u8; 4]);
        // lang_count = 2
        b.extend_from_slice(&2u32.to_le_bytes());
        b.push(100);
        b.push(50);
        // zoneId, zoneInstance (LE)
        b.extend_from_slice(&55u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        // y, x, z, heading (f32 LE)
        b.extend_from_slice(&(-12.5f32).to_le_bytes());
        b.extend_from_slice(&34.5f32.to_le_bytes());
        b.extend_from_slice(&7.0f32.to_le_bytes());
        b.extend_from_slice(&90.0f32.to_le_bytes());
        // standState, anon (BE u16)
        b.extend_from_slice(&100u16.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        // guildID, guildServerID
        b.extend_from_slice(&999u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        // 2b skip
        b.extend_from_slice(&[0u8; 2]);
        // inventory + bank + shared
        for v in [1u32, 2, 3, 4, 5, 6, 7, 8, 9] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // sc6
        b.extend_from_slice(&0u32.to_le_bytes());
        // 8b skip
        b.extend_from_slice(&[0u8; 8]);
        // careerTribute
        b.extend_from_slice(&77u32.to_le_bytes());
        // 4b skip
        b.extend_from_slice(&[0u8; 4]);
        // currentTribute
        b.extend_from_slice(&88u32.to_le_bytes());
        // 6b skip
        b.extend_from_slice(&[0u8; 6]);
        // tribute count
        b.extend_from_slice(&0u32.to_le_bytes());
        // sc7
        b.extend_from_slice(&0u32.to_le_bytes());
        // 137b skip
        b.extend_from_slice(&[0u8; 137]);
        // crystals (4 x u32)
        for v in [11u32, 12, 13, 14] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // 91b skip
        b.extend_from_slice(&[0u8; 91]);
        // autosplit
        b.push(1);
        // 57b skip
        b.extend_from_slice(&[0u8; 57]);
        // LDoN points (6 x u32)
        for v in [21u32, 22, 23, 24, 25, 26] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }

    #[test]
    fn skeleton_payload_round_trips() {
        let buf = skeleton();
        let p = parse_player_profile(&buf).unwrap();
        assert_eq!(p.checksum, 0xDEADBEEF);
        assert_eq!(p.race, 1);
        assert_eq!(p.class_, 2);
        assert_eq!(p.level, 60);
        assert_eq!(p.level1, 60);
        assert_eq!(p.bind0_zone_id, 123);
        assert_eq!(p.bind0_x, -100.0);
        assert_eq!(p.bind0_y, 200.0);
        assert_eq!(p.bind0_z, -50.0);
        assert_eq!(p.deity, 201);
        assert_eq!(p.mana, 1234);
        assert_eq!(p.cur_hp, 5678);
        assert_eq!(p.str_, 100);
        assert_eq!(p.wis, 160);
        assert_eq!(p.aa_ids, vec![500, 501]);
        assert_eq!(p.aa_values, vec![3, 5]);
        assert_eq!(p.buff_spell_ids, vec![42]);
        assert_eq!(p.buff_durations, vec![600]);
        assert_eq!(p.platinum, 10);
        assert_eq!(p.copper, 40);
        assert_eq!(p.platinum_cursor, 1);
        assert_eq!(p.aa_spent, 111);
        assert_eq!(p.aa_assigned, 222);
        assert_eq!(p.aa_unspent, 333);
        assert_eq!(p.endurance, 444);
        assert_eq!(p.exp_aa, 22846);
        assert_eq!(p.name, "Player");
        assert_eq!(p.last_name, "Last");
        assert_eq!(p.expansions, 0xFF);
        assert_eq!(p.languages, vec![100, 50]);
        assert_eq!(p.zone_id, 55);
        assert_eq!(p.x, 34.5);
        assert_eq!(p.y, -12.5);
        assert_eq!(p.z, 7.0);
        assert_eq!(p.heading, 90.0);
        assert_eq!(p.stand_state, 100);
        assert_eq!(p.guild_id, 999);
        assert_eq!(p.guild_server_id, 1);
        assert_eq!(p.platinum_inventory, 1);
        assert_eq!(p.platinum_bank, 5);
        assert_eq!(p.platinum_shared, 9);
        assert_eq!(p.career_tribute, 77);
        assert_eq!(p.current_tribute, 88);
        assert_eq!(p.current_rad_crystals, 11);
        assert_eq!(p.career_ebon_crystals, 14);
        assert_eq!(p.autosplit, 1);
        assert_eq!(p.ldon_guk_points, 21);
        assert_eq!(p.ldon_avail_points, 26);
        assert_eq!(p.bytes_consumed as usize, buf.len());
    }

    #[test]
    fn truncated_payload_errors() {
        let buf = vec![0u8; 100];
        assert!(parse_player_profile(&buf).is_err());
    }
}
