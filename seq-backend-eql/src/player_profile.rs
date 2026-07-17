//! `PlayerProfile` output struct + error for `OP_PlayerProfile`.
//!
//! The single authoritative eql parser is `parse_player_profile` in
//! `lib.rs` (fixed identity offsets gender@20 / race@21 / class@25 /
//! classMask@29 / level@33, then an absolute name-block scan). This
//! module holds ONLY the struct/error it returns — the old vendored
//! NetStream-walk copy of the live parser was dead code (never called by
//! the bridge) and was removed to keep one authoritative parser.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerProfile {
    pub bytes_consumed: u32,
    pub checksum: u32,

    // profile.*
    pub gender: u8,
    pub race: u32,
    pub class_: u32,
    pub class_mask: u32,
    // EQL active stance / invocation ability ids, read at a FIXED offset in the
    // profile (33777 / 33781; verified identical across 2 chars of different
    // class/level/server). OP_Stance/OP_Invocation echo only on a SWAP, so this
    // seeds the initial state at zone-in. 0 = none / out of range.
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
    /// Player skill values, index = skill id (0..MAX_KNOWN_SKILLS). Populated by
    /// the eql `parse_player_profile` walk; empty on a short-read.
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
