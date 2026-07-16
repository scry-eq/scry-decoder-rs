//! C++ FFI bridge — exposes the Rust decoders across the cxx ABI.
//!
//! The bridge is intentionally a thin shim. Parsing logic stays in the decode
//! crates (`seq-decode` for the shared/Live path, `seq-backend-eql` for eql's
//! diverged opcodes) so it remains usable from pure-Rust contexts (replay
//! tools, the eventual standalone daemon). This crate is the `staticlib`
//! Corrosion links into `seq-daemon-core`.
//!
//! Backend selection via Cargo features: `backend-live` / `backend-test` pick
//! the bindings crate in `seq-decode` (Test rides the shared Live decoders).
//! `backend-eql` instead links `seq-backend-eql` — a fully self-contained eql
//! decode stack that shares NOTHING with Live, so a Live wire patch can't reach
//! eql. The uniform `decode_*` FFI surface is identical for every backend; the
//! `backend` alias below routes each call to the active backend's decoders, and
//! the few opcodes whose eql wire diverges call eql's parser explicitly.

#[cfg(not(any(feature = "backend-live", feature = "backend-test", feature = "backend-eql")))]
compile_error!(
    "seq-bridge: enable exactly one backend feature (default: backend-live)."
);

// The active backend's decoder crate. live/test share `seq-decode`; eql is its
// own self-contained stack (`seq-backend-eql`). Shared decoders call
// `backend::parse_*`; the eql-diverged opcodes cfg-select
// `seq_backend_eql::parse_legends_*` directly below.
#[cfg(not(feature = "backend-eql"))]
use seq_decode as backend;
#[cfg(feature = "backend-eql")]
use seq_backend_eql as backend;

#[cxx::bridge(namespace = "seq::rust")]
mod ffi {
    /// Decoded `OP_MobUpdate` payload. `ok` is the discriminator: when
    /// false the remaining fields are zeroed and the daemon drops the
    /// packet. The discriminator is preferred over cxx's `Result`
    /// mapping because the latter would emit C++ exceptions for what
    /// the daemon's SZC_Match dispatch already prevents.
    struct MobUpdate {
        spawn_id: u16,
        x: i32,
        y: i32,
        z: i32,
        heading: u16,
        ok: bool,
    }

    /// Decoded `OP_DeleteSpawn`. `ok=false` means the payload was the
    /// wrong size; SZC_Match in the daemon already guards against that
    /// so it shouldn't fire in normal operation.
    struct DeleteSpawn {
        spawn_id: u32,
        ok: bool,
    }

    /// Variable-length spawn payload (Stage A+2). Mirrors the fields
    /// `SpawnShell::fillSpawnStruct` populates on a `spawnStruct`. The
    /// daemon assigns each field into its own struct via `applySpawn`.
    /// `bytes_consumed` records the parser's consumed length.
    struct Spawn {
        ok: bool,
        bytes_consumed: u32,

        // Daemon strncpy's into its fixed-size char buffers
        // (everquest.h spawnStruct).
        name: String,
        last_name: String,
        title: String,
        suffix: String,

        spawn_id: u32,
        misc_data: u32,
        body_type: u32,
        race: u32,
        deity: u32,
        guild_id: u32,
        guild_server_id: u32,
        class_: u32,
        pet_owner_id: u32,

        // 9 slots × 5 u32s — same memory layout as
        // EquipStruct equipment[9] on the C++ side.
        equip_data: [u32; 45],
        pos_data: [u32; 5],

        level: u8,
        npc: u8,
        other_data: u8,
        char_properties: u8,
        cur_hp: u8,
        holding: u8,
        state: u8,
        light: u8,
        is_mercenary: u8,

        // Decoded position + hp, filled only by backends whose spawn wire is
        // already decoded (eql). Live leaves these zero and uses pos_data /
        // cur_hp; eql leaves the raw arrays zero and fills these.
        x: i16,
        y: i16,
        z: i16,
        max_hp: u8,
        // Decoded h2048 heading (0..2047). eql only; Live leaves 0 and takes
        // heading from pos_data / OP_MobUpdate.
        heading: u16,
    }

    // Stage A+3 — small fixed-size opcodes. Each struct ends with an
    // `ok` discriminator: false means the payload was malformed (length
    // mismatch / failed extraction); the daemon drops the packet.
    struct RemoveSpawn {
        spawn_id: u32,
        remove_spawn: u8,
        ok: bool,
    }
    struct HpUpdate {
        spawn_id: u16,
        cur_hp: i32,
        max_hp: i32,
        ok: bool,
    }
    // eql OP_HPUpdate (0x2735) is a multiplexed stat-sync channel, not Live's
    // fixed HP struct — decoded via decode_stat_sync. `wide` gates the real
    // cur/max (vs percent) forms; `has_*` mark which stats the packet carried.
    struct StatSync {
        spawn_id: u32,
        wide: bool,
        has_hp: bool,
        hp_cur: i64,
        hp_max: i64,
        has_mana: bool,
        mana_cur: i64,
        mana_max: i64,
        has_end: bool,
        end_cur: i64,
        end_max: i64,
        ok: bool,
    }
    // eql OP_LoadoutSwap (0x7477): a player's multiclass loadout change. Only
    // the identity fields that change on a swap are surfaced; spawn_id is the
    // header id of the player who swapped (self or a nearby tracked spawn).
    struct LoadoutSwap {
        spawn_id: u32,
        level: u8,
        class_: u32,
        race: u32,
        ok: bool,
    }
    struct MoneyUpdate {
        copper: u32,
        ok: bool,
    }
    struct MobHealth {
        spawn_id: u16,
        hp_percent: i32,
        ok: bool,
    }
    struct SpawnAppearance {
        spawn_id: u16,
        kind: u16,
        parameter: u32,
        ok: bool,
    }
    struct ExpUpdate {
        exp: u32,
        unknown0: u32,
        kind: u32,
        unknown1: u32,
        ok: bool,
    }
    struct LevelUpdate {
        level: u32,
        level_old: u32,
        exp: u32,
        unknown0: u32,
        ok: bool,
    }
    struct SkillUpdate {
        skill_id: u32,
        value: i32,
        ok: bool,
    }

    // Stage A+4 — 7 more small fixed-size opcodes.
    struct ManaChange {
        new_mana: i32,
        max_mana: i32,
        spell_id: i32,
        ok: bool,
    }
    struct Stamina { food: u32, water: u32, ok: bool }
    struct EndUpdate { spawn_id: u16, cur: u32, max: u32, ok: bool }
    struct Consider {
        player_id: u32, target_id: u32, faction: i32, level: i32, ok: bool,
    }
    /// A backend-declared payload size override: `name` (a toml `typename`) →
    /// its authoritative size on the linked backend. The daemon applies these
    /// over its C++ `sizeof` size table so `SZC_Match` validates against the
    /// backend's size, not a hardcoded Live `sizeof`. Empty for live/test.
    struct StructSize {
        name: String,
        size: u32,
    }
    // One record of eql OP_BuffList (0x77ae). Returned as a flat Vec (empty on
    // decode failure); every entry repeats spawn_id so the C++ side can filter
    // to the player without a wrapper struct. remaining_ticks <= 0 = permanent.
    struct BuffListEntry {
        spawn_id: u32,
        spell_id: u32,
        remaining_ticks: i32,
        slot: u32,
    }
    // One EQL UCS (cross-zone chat) line. `channel_first` is the still-masked
    // first byte of the channel name; `channel_rest` is the clean remainder.
    // The caller recovers the per-session mask (from the General* crib) to
    // repair `channel_first`. An empty Vec = no chat in the packet.
    struct UcsChatRecord {
        channel_first: u8,
        channel_rest: String,
        channel_run: String,
        sender: String,
        message: String,
        spam: bool,
    }
    struct SpawnRename {
        old_name: String,
        old_name_again: String,
        new_name: String,
        ok: bool,
    }
    struct ClientTarget { new_target: u32, ok: bool }
    struct Death {
        spawn_id: u32, killer_id: u32, corpse_id: u32, kind: i32,
        spell_id: u32, zone_id: u16, zone_instance: u16, damage: u32,
        ok: bool,
    }

    // Stage A+5
    struct ClickObject { drop_id: u16, spawn_id: u16, ok: bool }
    struct Illusion {
        spawn_id: u32, name: String, race: u32, gender: u8,
        texture: u8, helm: u8, face: u32, ok: bool,
    }
    struct Buff {
        spawn_id: u32, spell_id: u32,
        // form: 0=fade(13b) | 1=initial(30b) | 2=live-update(34+b).
        form: u8, slot: u8, dur_ticks: u32, ok: bool,
    }
    struct Action2 {
        target: u16, source: u16, damage: i32, spell: i32, kind: u8, ok: bool,
    }

    // Stage A+6 — second small-fixed POD batch.
    struct WearChange {
        spawn_id: u16, subcommand: u16, arg1: i16, arg2: i16, arg3: u8,
        ok: bool,
    }
    struct ZoneChange {
        name: String, zone_id: u16, zone_instance: u16, ok: bool,
    }
    struct DzInfo {
        new_dz: u8, max_players: u32, dz_name: String, name: String,
        ok: bool,
    }
    struct DzSwitch {
        zone_id: u16, instance_id: u16, kind: u32,
        x: f32, y: f32, z: f32, ok: bool,
    }
    struct StartCast {
        slot: i32, spell_id: u32, target_id: u32, ok: bool,
    }
    struct Action {
        target: u16, source: u16, spell: u16, level: u8, kind: u8, ok: bool,
    }
    struct GroupDisband {
        yourname: String, membername: String, ok: bool,
    }
    struct GroupFollow { name: String, ok: bool }
    struct GroupMemberList {
        group_id: u32,
        member_count: u32,
        // scanned member names, '\n'-separated (daemon splits + dedups + self-filters).
        names: String,
        ok: bool,
    }
    struct CorpseLoc {
        spawn_id: u32, x: f32, y: f32, z: f32, ok: bool,
    }

    // Stage A+7 — variable-length / array opcodes.
    struct Door {
        name: String,
        y: f32, x: f32, z: f32, heading: f32,
        incline: u32, size: u32,
        door_id: u8, opentype: u8, spawnstate: u8, invertstate: u8,
        zone_point: u32,
        ok: bool,
    }
    struct GroundSpawn {
        drop_id: u32,
        id_file: String,
        heading: f32, y: f32, x: f32, z: f32,
        bytes_consumed: u32,
        ok: bool,
    }

    // Per-element decode for OP_SendZonePoints. Daemon reads the 4-byte
    // count off the front, then invokes this on each 24-byte
    // zonePointStruct slice.
    struct ZonePoint {
        zone_trigger: u32,
        y: f32, x: f32, z: f32, heading: f32,
        zone_id: u16, zone_instance: u16,
        ok: bool,
    }

    // Message opcode payloads. OP_SimpleMessage is fixed 12b; OP_FormattedMessage
    // has a 13b header + variable-length text array (daemon slices the
    // tail off the raw payload); OP_SpecialMesg has two embedded
    // NUL-terminated strings the parser surfaces directly.
    struct SimpleMessage {
        message_format: u32,
        message_color: u32,
        ok: bool,
    }
    // OP_FormattedMessage. `message_format`/`message_color` are the stock
    // Live header (format id + chat colour). The remaining fields are the
    // EQL 0x3c0a enrichment and stay zero/empty on live/test: that channel
    // diverges (format id @9, not @5) and multiplexes a spell id, a
    // message-class discriminator, the actor spawn id, and a pre-split
    // NUL-delimited arg list the stock header can't represent. On eql,
    // message_format/message_color mirror format_id/spell_id so the stock
    // MessageShell::formattedMessage symbol still resolves; the eql handler
    // reads the rich fields. See seq-backend-eql/src/formatted_message.rs.
    struct FormattedMessage {
        message_format: u32,
        message_color: u32,
        spell_id: u32,
        msg_type: u8,
        spawn_id: u32,
        format_id: u32,
        args: Vec<String>,
        ok: bool,
    }
    struct SpecialMessage {
        message_color: u32,
        target: u16,
        source: String,
        message: String,
        ok: bool,
    }
    struct ChannelMessage {
        sender: String,
        target: String,
        language: u32,
        chan_num: u32,
        skill_in_language: u32,
        message: String,
        ok: bool,
    }
    struct NewZone {
        short_name: String,
        long_name: String,
        zonefile: String,
        zone_exp_multiplier: f32,
        safe_y: f32, safe_x: f32, safe_z: f32,
        zone_id: u32,
        ok: bool,
    }

    // OP_PlayerProfile — long, variable-length NetStream walk. Only
    // fields the daemon's downstream consumers actually read are
    // exposed; everything else is parsed (to advance the cursor) but
    // dropped. `bytes_consumed` matches the C++ parser's tally for the
    // length-mismatch debug print.
    struct PlayerProfile {
        ok: bool,
        bytes_consumed: u32,
        checksum: u32,

        // profile.*
        gender: u8,
        race: u32,
        class_: u32,
        level: u8,
        level1: u8,
        bind0_zone_id: u32,
        bind0_x: f32, bind0_y: f32, bind0_z: f32, bind0_heading: f32,
        deity: u32,
        intoxication: u32,
        points: u32,
        mana: u32,
        cur_hp: u32,
        str_: u32, sta: u32, cha: u32, dex: u32,
        int_: u32, agi: u32, wis: u32,
        aa_ids: Vec<u32>,
        aa_values: Vec<u32>,
        disciplines: Vec<u32>,
        recast_timers: Vec<u32>,
        spell_book: Vec<i32>,
        mem_spells: Vec<i32>,
        spell_slot_refresh: Vec<u32>,
        buff_spell_ids: Vec<i32>,
        buff_durations: Vec<i32>,
        platinum: u32, gold: u32, silver: u32, copper: u32,
        platinum_cursor: u32, gold_cursor: u32,
        silver_cursor: u32, copper_cursor: u32,
        aa_spent: u32,
        aa_assigned: u32,
        aa_unspent: u32,
        endurance: u32,
        exp_aa: u32,

        // charProfileStruct top-level
        name: String,
        last_name: String,
        birthday_time: u32,
        account_create_date: u32,
        last_save_time: u32,
        time_played_min: u32,
        expansions: u32,
        languages: Vec<u8>,
        zone_id: u16,
        zone_instance: u16,
        x: f32, y: f32, z: f32, heading: f32,
        stand_state: u16,
        anon: u16,
        guild_id: u32,
        guild_server_id: u32,
        platinum_inventory: u32, gold_inventory: u32,
        silver_inventory: u32, copper_inventory: u32,
        platinum_bank: u32, gold_bank: u32,
        silver_bank: u32, copper_bank: u32,
        platinum_shared: u32,
        career_tribute: u32, current_tribute: u32,
        current_rad_crystals: u32, career_rad_crystals: u32,
        current_ebon_crystals: u32, career_ebon_crystals: u32,
        autosplit: u8,
        ldon_guk_points: u32, ldon_mir_points: u32, ldon_mmc_points: u32,
        ldon_ruj_points: u32, ldon_tak_points: u32, ldon_avail_points: u32,
    }

    // Stage A+8 — bitfield-laden / BitStream-packed opcodes.
    struct PlayerSelfPos {
        spawn_id: u16,
        x: f32, y: f32, z: f32,
        delta_x: f32, delta_y: f32, delta_z: f32,
        heading: u16, delta_heading: i16, animation: i16,
        pitch: u16,
        ok: bool,
    }
    struct PlayerSpawnPos {
        spawn_id: u16, spawn_id2: u16,
        x: i32, y: i32, z: i32,
        delta_x: i32, delta_y: i32, delta_z: i32,
        heading: u16, delta_heading: i16, animation: i16,
        pitch: u16,
        ok: bool,
    }
    struct NpcMove {
        spawn_id: u16,
        x: i16, y: i16, z: i16, heading: i16,
        delta_x: i16, delta_y: i16, delta_z: i16,
        delta_heading: i8, animation: i16,
        ok: bool,
    }

    extern "Rust" {
        fn decode_mob_update(bytes: &[u8]) -> MobUpdate;
        fn decode_delete_spawn(bytes: &[u8]) -> DeleteSpawn;
        fn decode_spawn(bytes: &[u8]) -> Spawn;
        fn decode_remove_spawn(bytes: &[u8]) -> RemoveSpawn;
        fn decode_hp_update(bytes: &[u8]) -> HpUpdate;
        fn decode_stat_sync(bytes: &[u8]) -> StatSync;
        fn decode_loadout_swap(bytes: &[u8]) -> LoadoutSwap;
        fn decode_money_update(bytes: &[u8]) -> MoneyUpdate;
        fn decode_buff_list(bytes: &[u8]) -> Vec<BuffListEntry>;
        fn decode_ucs_chat(bytes: &[u8]) -> Vec<UcsChatRecord>;
        fn decode_ucs_channels(bytes: &[u8]) -> Vec<String>;
        fn decode_mob_health(bytes: &[u8]) -> MobHealth;
        fn decode_spawn_appearance(bytes: &[u8]) -> SpawnAppearance;
        fn decode_exp_update(bytes: &[u8]) -> ExpUpdate;
        fn decode_level_update(bytes: &[u8]) -> LevelUpdate;
        fn decode_skill_update(bytes: &[u8]) -> SkillUpdate;
        fn decode_mana_change(bytes: &[u8]) -> ManaChange;
        fn decode_stamina(bytes: &[u8]) -> Stamina;
        fn decode_end_update(bytes: &[u8]) -> EndUpdate;
        fn decode_consider(bytes: &[u8]) -> Consider;
        fn decode_spawn_rename(bytes: &[u8]) -> SpawnRename;
        fn decode_client_target(bytes: &[u8]) -> ClientTarget;
        fn decode_death(bytes: &[u8]) -> Death;
        fn decode_click_object(bytes: &[u8]) -> ClickObject;
        fn decode_illusion(bytes: &[u8]) -> Illusion;
        fn decode_buff(bytes: &[u8]) -> Buff;
        fn decode_action2(bytes: &[u8]) -> Action2;
        // Stage A+6
        fn decode_wear_change(bytes: &[u8]) -> WearChange;
        fn decode_zone_change(bytes: &[u8]) -> ZoneChange;
        fn decode_dz_info(bytes: &[u8]) -> DzInfo;
        fn decode_dz_switch_info(bytes: &[u8]) -> DzSwitch;
        fn decode_start_cast(bytes: &[u8]) -> StartCast;
        fn decode_action(bytes: &[u8]) -> Action;
        fn decode_action_alt(bytes: &[u8]) -> Action;
        fn decode_group_disband(bytes: &[u8]) -> GroupDisband;
        fn decode_group_follow(bytes: &[u8]) -> GroupFollow;
        fn decode_group_member_list(bytes: &[u8]) -> GroupMemberList;
        fn decode_corpse_loc(bytes: &[u8]) -> CorpseLoc;
        // Stage A+7
        fn decode_door(bytes: &[u8]) -> Door;
        fn decode_ground_spawn(bytes: &[u8]) -> GroundSpawn;
        fn decode_zone_point(bytes: &[u8]) -> ZonePoint;
        // Message opcodes
        fn decode_simple_message(bytes: &[u8]) -> SimpleMessage;
        fn decode_formatted_message(bytes: &[u8]) -> FormattedMessage;
        fn decode_special_message(bytes: &[u8]) -> SpecialMessage;
        fn decode_loot_message(bytes: &[u8]) -> SpecialMessage;
        fn decode_channel_message(bytes: &[u8]) -> ChannelMessage;
        fn decode_new_zone(bytes: &[u8]) -> NewZone;
        fn decode_player_profile(bytes: &[u8]) -> PlayerProfile;
        // Stage A+8
        fn decode_player_self_pos(bytes: &[u8]) -> PlayerSelfPos;
        fn decode_player_spawn_pos(bytes: &[u8]) -> PlayerSpawnPos;
        fn decode_npc_move_update(bytes: &[u8]) -> NpcMove;

        /// Backend-sourced payload size overrides (see `StructSize`). Empty on
        /// live/test; eql returns the payloads whose wire size diverges from
        /// Live's compiled `everquest.h` struct.
        fn struct_size_overrides() -> Vec<StructSize>;

        /// Per-row byte stride of an `OP_SpawnDoor` array payload for the
        /// linked backend (136 on live/test = `sizeof(doorStruct)`; 132 on
        /// eql). The daemon's `SpawnShell::newDoorSpawns` iterates with this
        /// instead of the compiled Live `sizeof`, which would mis-stride a
        /// diverged backend's rows.
        fn door_stride() -> usize;
    }
}

fn struct_size_overrides() -> Vec<ffi::StructSize> {
    // The daemon's SZC_Match size table is built from Live's C++ `sizeof`
    // (s_everquest.h); these entries let the linked backend override any name
    // whose wire size diverges. live/test diverge from nothing → empty; eql
    // sources its list from the pinned seq-backend-eql struct/parser sizes.
    #[cfg(feature = "backend-eql")]
    let raw = seq_backend_eql::size_overrides();
    #[cfg(not(feature = "backend-eql"))]
    let raw: Vec<(&'static str, u32)> = Vec::new();
    raw.into_iter()
        .map(|(name, size)| ffi::StructSize { name: name.to_string(), size })
        .collect()
}

fn door_stride() -> usize {
    backend::spawn_door::PAYLOAD_LEN
}

fn decode_mob_update(bytes: &[u8]) -> ffi::MobUpdate {
    // eql's OP_MobUpdate is byte-identical to Live's spawnPositionUpdate
    // (14B, packed y:19/z:19/u3:7/x:19/heading:12 fixed-point ×8; verified
    // 2026-07-08 over 1665 packets — 19-bit sign-fill consistent on every
    // axis), so every backend shares the Live parser here.
    match backend::parse_mob_update(bytes) {
        Ok(m) => ffi::MobUpdate {
            spawn_id: m.spawn_id,
            x: m.x,
            y: m.y,
            z: m.z,
            heading: m.heading,
            ok: true,
        },
        Err(_) => ffi::MobUpdate {
            spawn_id: 0,
            x: 0,
            y: 0,
            z: 0,
            heading: 0,
            ok: false,
        },
    }
}

fn decode_delete_spawn(bytes: &[u8]) -> ffi::DeleteSpawn {
    match backend::parse_delete_spawn(bytes) {
        Ok(d) => ffi::DeleteSpawn { spawn_id: d.spawn_id, ok: true },
        Err(_) => ffi::DeleteSpawn { spawn_id: 0, ok: false },
    }
}

fn decode_remove_spawn(bytes: &[u8]) -> ffi::RemoveSpawn {
    match backend::parse_remove_spawn(bytes) {
        Ok(r) => ffi::RemoveSpawn {
            spawn_id: r.spawn_id, remove_spawn: r.remove_spawn, ok: true,
        },
        Err(_) => ffi::RemoveSpawn { spawn_id: 0, remove_spawn: 0, ok: false },
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_hp_update(bytes: &[u8]) -> ffi::HpUpdate {
    match backend::parse_hp_update(bytes) {
        Ok(h) => ffi::HpUpdate {
            spawn_id: h.spawn_id, cur_hp: h.cur_hp, max_hp: h.max_hp, ok: true,
        },
        Err(_) => ffi::HpUpdate {
            spawn_id: 0, cur_hp: 0, max_hp: 0, ok: false,
        },
    }
}

// eql: OP_HPUpdate (0x2735) is the multiplexed stat-sync channel, decoded via
// decode_stat_sync — Live's fixed HP struct never appears, so this shared FFI
// is inert.
#[cfg(feature = "backend-eql")]
fn decode_hp_update(_bytes: &[u8]) -> ffi::HpUpdate {
    ffi::HpUpdate { spawn_id: 0, cur_hp: 0, max_hp: 0, ok: false }
}

fn stat_sync_err() -> ffi::StatSync {
    ffi::StatSync {
        spawn_id: 0, wide: false,
        has_hp: false, hp_cur: 0, hp_max: 0,
        has_mana: false, mana_cur: 0, mana_max: 0,
        has_end: false, end_cur: 0, end_max: 0,
        ok: false,
    }
}

// eql-only: the multiplexed stat-sync channel (real HP cur/max + player mana).
// live/test have no such channel, so their build gets an inert stub.
#[cfg(feature = "backend-eql")]
fn decode_stat_sync(bytes: &[u8]) -> ffi::StatSync {
    match seq_backend_eql::parse_stat_sync(bytes) {
        Ok(s) => ffi::StatSync {
            spawn_id: s.spawn_id, wide: s.wide,
            has_hp: s.has_hp, hp_cur: s.hp_cur, hp_max: s.hp_max,
            has_mana: s.has_mana, mana_cur: s.mana_cur, mana_max: s.mana_max,
            has_end: s.has_end, end_cur: s.end_cur, end_max: s.end_max,
            ok: true,
        },
        Err(_) => stat_sync_err(),
    }
}

fn loadout_swap_err() -> ffi::LoadoutSwap {
    ffi::LoadoutSwap { spawn_id: 0, level: 0, class_: 0, race: 0, ok: false }
}

// eql-only: OP_LoadoutSwap (0x7477). live/test have no such opcode, so their
// build gets an inert stub.
#[cfg(feature = "backend-eql")]
fn decode_loadout_swap(bytes: &[u8]) -> ffi::LoadoutSwap {
    match seq_backend_eql::parse_loadout_swap(bytes) {
        Ok(s) => ffi::LoadoutSwap {
            spawn_id: s.spawn_id, level: s.level, class_: s.class_, race: s.race, ok: true,
        },
        Err(_) => loadout_swap_err(),
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_loadout_swap(_bytes: &[u8]) -> ffi::LoadoutSwap {
    loadout_swap_err()
}

// eql-only: OP_MoneyUpdate (0x4d77) running total copper.
#[cfg(feature = "backend-eql")]
fn decode_money_update(bytes: &[u8]) -> ffi::MoneyUpdate {
    match seq_backend_eql::parse_money_update(bytes) {
        Ok(m) => ffi::MoneyUpdate { copper: m.copper, ok: true },
        Err(_) => ffi::MoneyUpdate { copper: 0, ok: false },
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_money_update(_bytes: &[u8]) -> ffi::MoneyUpdate {
    ffi::MoneyUpdate { copper: 0, ok: false }
}

// eql-only: OP_LootMessage (0x7d46) personal loot text, reusing SpecialMessage.
#[cfg(feature = "backend-eql")]
fn decode_loot_message(bytes: &[u8]) -> ffi::SpecialMessage {
    match seq_backend_eql::parse_loot_message(bytes) {
        Ok(m) => ffi::SpecialMessage {
            message_color: m.color, target: 0,
            source: String::new(), message: m.text, ok: true,
        },
        Err(_) => ffi::SpecialMessage {
            message_color: 0, target: 0,
            source: String::new(), message: String::new(), ok: false,
        },
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_loot_message(_bytes: &[u8]) -> ffi::SpecialMessage {
    ffi::SpecialMessage {
        message_color: 0, target: 0,
        source: String::new(), message: String::new(), ok: false,
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_stat_sync(_bytes: &[u8]) -> ffi::StatSync {
    stat_sync_err()
}

// eql-only: OP_BuffList (0x77ae) — the authoritative per-spawn active-buff list.
// Flattened to a Vec (empty = decode failed / not present). live/test stub empty.
#[cfg(feature = "backend-eql")]
fn decode_buff_list(bytes: &[u8]) -> Vec<ffi::BuffListEntry> {
    match seq_backend_eql::parse_buff_list(bytes) {
        Ok(list) => list
            .entries
            .into_iter()
            .map(|e| ffi::BuffListEntry {
                spawn_id: list.spawn_id,
                spell_id: e.spell_id,
                remaining_ticks: e.remaining_ticks,
                slot: e.slot,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_buff_list(_bytes: &[u8]) -> Vec<ffi::BuffListEntry> {
    Vec::new()
}

// EQL UCS cross-zone chat. Flattened to a Vec (empty = no chat / not present).
// live/test stub empty.
#[cfg(feature = "backend-eql")]
fn decode_ucs_chat(bytes: &[u8]) -> Vec<ffi::UcsChatRecord> {
    seq_backend_eql::parse_ucs_chat(bytes)
        .into_iter()
        .map(|r| ffi::UcsChatRecord {
            channel_first: r.channel_first,
            channel_rest: r.channel_rest,
            channel_run: r.channel_run,
            sender: r.sender,
            message: r.message,
            spam: r.spam,
        })
        .collect()
}

#[cfg(not(feature = "backend-eql"))]
fn decode_ucs_chat(_bytes: &[u8]) -> Vec<ffi::UcsChatRecord> {
    Vec::new()
}

// EQL UCS channel-name learning (/list rosters + join notices). live/test stub.
#[cfg(feature = "backend-eql")]
fn decode_ucs_channels(bytes: &[u8]) -> Vec<String> {
    seq_backend_eql::parse_ucs_channels(bytes)
}

#[cfg(not(feature = "backend-eql"))]
fn decode_ucs_channels(_bytes: &[u8]) -> Vec<String> {
    Vec::new()
}

fn decode_mob_health(bytes: &[u8]) -> ffi::MobHealth {
    match backend::parse_mob_health(bytes) {
        Ok(m) => ffi::MobHealth {
            spawn_id: m.spawn_id, hp_percent: m.hp_percent, ok: true,
        },
        Err(_) => ffi::MobHealth { spawn_id: 0, hp_percent: 0, ok: false },
    }
}

fn decode_spawn_appearance(bytes: &[u8]) -> ffi::SpawnAppearance {
    match backend::parse_spawn_appearance(bytes) {
        Ok(a) => ffi::SpawnAppearance {
            spawn_id: a.spawn_id, kind: a.kind, parameter: a.parameter, ok: true,
        },
        Err(_) => ffi::SpawnAppearance {
            spawn_id: 0, kind: 0, parameter: 0, ok: false,
        },
    }
}

fn decode_exp_update(bytes: &[u8]) -> ffi::ExpUpdate {
    match backend::parse_exp_update(bytes) {
        Ok(e) => ffi::ExpUpdate {
            exp: e.exp, unknown0: e.unknown0, kind: e.kind, unknown1: e.unknown1,
            ok: true,
        },
        Err(_) => ffi::ExpUpdate {
            exp: 0, unknown0: 0, kind: 0, unknown1: 0, ok: false,
        },
    }
}

fn decode_level_update(bytes: &[u8]) -> ffi::LevelUpdate {
    match backend::parse_level_update(bytes) {
        Ok(l) => ffi::LevelUpdate {
            level: l.level, level_old: l.level_old, exp: l.exp,
            unknown0: l.unknown0, ok: true,
        },
        Err(_) => ffi::LevelUpdate {
            level: 0, level_old: 0, exp: 0, unknown0: 0, ok: false,
        },
    }
}

fn decode_skill_update(bytes: &[u8]) -> ffi::SkillUpdate {
    match backend::parse_skill_update(bytes) {
        Ok(s) => ffi::SkillUpdate {
            skill_id: s.skill_id, value: s.value, ok: true,
        },
        Err(_) => ffi::SkillUpdate { skill_id: 0, value: 0, ok: false },
    }
}

fn decode_mana_change(bytes: &[u8]) -> ffi::ManaChange {
    match backend::parse_mana_change(bytes) {
        Ok(m) => ffi::ManaChange {
            new_mana: m.new_mana, max_mana: m.max_mana, spell_id: m.spell_id, ok: true,
        },
        Err(_) => ffi::ManaChange {
            new_mana: 0, max_mana: 0, spell_id: 0, ok: false,
        },
    }
}

fn decode_stamina(bytes: &[u8]) -> ffi::Stamina {
    match backend::parse_stamina(bytes) {
        Ok(s) => ffi::Stamina { food: s.food, water: s.water, ok: true },
        Err(_) => ffi::Stamina { food: 0, water: 0, ok: false },
    }
}

fn decode_end_update(bytes: &[u8]) -> ffi::EndUpdate {
    match backend::parse_end_update(bytes) {
        Ok(e) => ffi::EndUpdate {
            spawn_id: e.spawn_id, cur: e.cur, max: e.max, ok: true,
        },
        Err(_) => ffi::EndUpdate { spawn_id: 0, cur: 0, max: 0, ok: false },
    }
}

fn decode_consider(bytes: &[u8]) -> ffi::Consider {
    let parsed = backend::parse_consider(bytes);
    match parsed {
        Ok(c) => ffi::Consider {
            player_id: c.player_id, target_id: c.target_id,
            faction: c.faction, level: c.level, ok: true,
        },
        Err(_) => ffi::Consider {
            player_id: 0, target_id: 0, faction: 0, level: 0, ok: false,
        },
    }
}

fn decode_spawn_rename(bytes: &[u8]) -> ffi::SpawnRename {
    match backend::parse_spawn_rename(bytes) {
        Ok(r) => ffi::SpawnRename {
            old_name: r.old_name,
            old_name_again: r.old_name_again,
            new_name: r.new_name,
            ok: true,
        },
        Err(_) => ffi::SpawnRename {
            old_name: String::new(),
            old_name_again: String::new(),
            new_name: String::new(),
            ok: false,
        },
    }
}

fn decode_client_target(bytes: &[u8]) -> ffi::ClientTarget {
    match backend::parse_client_target(bytes) {
        Ok(t) => ffi::ClientTarget { new_target: t.new_target, ok: true },
        Err(_) => ffi::ClientTarget { new_target: 0, ok: false },
    }
}

fn decode_death(bytes: &[u8]) -> ffi::Death {
    match backend::parse_death(bytes) {
        Ok(d) => ffi::Death {
            spawn_id: d.spawn_id, killer_id: d.killer_id, corpse_id: d.corpse_id,
            kind: d.kind, spell_id: d.spell_id, zone_id: d.zone_id,
            zone_instance: d.zone_instance, damage: d.damage, ok: true,
        },
        Err(_) => ffi::Death {
            spawn_id: 0, killer_id: 0, corpse_id: 0, kind: 0, spell_id: 0,
            zone_id: 0, zone_instance: 0, damage: 0, ok: false,
        },
    }
}

fn decode_click_object(bytes: &[u8]) -> ffi::ClickObject {
    match backend::parse_click_object(bytes) {
        Ok(c) => ffi::ClickObject {
            drop_id: c.drop_id, spawn_id: c.spawn_id, ok: true,
        },
        Err(_) => ffi::ClickObject { drop_id: 0, spawn_id: 0, ok: false },
    }
}

fn decode_illusion(bytes: &[u8]) -> ffi::Illusion {
    match backend::parse_illusion(bytes) {
        Ok(i) => ffi::Illusion {
            spawn_id: i.spawn_id, name: i.name, race: i.race,
            gender: i.gender, texture: i.texture, helm: i.helm, face: i.face,
            ok: true,
        },
        Err(_) => ffi::Illusion {
            spawn_id: 0, name: String::new(), race: 0, gender: 0,
            texture: 0, helm: 0, face: 0, ok: false,
        },
    }
}

fn decode_buff(bytes: &[u8]) -> ffi::Buff {
    match backend::parse_buff(bytes) {
        Ok(b) => ffi::Buff {
            spawn_id: b.spawn_id, spell_id: b.spell_id,
            form: b.form, slot: b.slot, dur_ticks: b.dur_ticks,
            ok: true,
        },
        Err(_) => ffi::Buff {
            spawn_id: 0, spell_id: 0, form: 0, slot: 0, dur_ticks: 0, ok: false,
        },
    }
}

fn decode_action2(bytes: &[u8]) -> ffi::Action2 {
    match backend::parse_action2(bytes) {
        Ok(a) => ffi::Action2 {
            target: a.target, source: a.source, damage: a.damage,
            spell: a.spell, kind: a.kind, ok: true,
        },
        Err(_) => ffi::Action2 {
            target: 0, source: 0, damage: 0, spell: 0, kind: 0, ok: false,
        },
    }
}

// Zeroed sentinel for a bad/absent spawn payload. Also the field base for
// eql's partial fill (its Legends spawn only decodes id/name/pos/level/hp; the
// Live-only raw equipment/position arrays stay zero).
fn spawn_err() -> ffi::Spawn {
    ffi::Spawn {
        ok: false,
        bytes_consumed: 0,
        name: String::new(),
        last_name: String::new(),
        title: String::new(),
        suffix: String::new(),
        spawn_id: 0, misc_data: 0, body_type: 0, race: 0,
        deity: 0, guild_id: 0, guild_server_id: 0, class_: 0,
        pet_owner_id: 0,
        equip_data: [0; 45],
        pos_data: [0; 5],
        level: 0, npc: 0, other_data: 0, char_properties: 0,
        cur_hp: 0, holding: 0, state: 0, light: 0, is_mercenary: 0,
        x: 0, y: 0, z: 0, max_hp: 0, heading: 0,
    }
}

#[cfg(not(feature = "backend-eql"))]
fn decode_spawn(bytes: &[u8]) -> ffi::Spawn {
    match backend::parse_spawn(bytes) {
        Ok(s) => ffi::Spawn {
            ok: true,
            bytes_consumed: s.bytes_consumed,
            name: s.name,
            last_name: s.last_name,
            title: s.title,
            suffix: s.suffix,
            spawn_id: s.spawn_id,
            misc_data: s.misc_data,
            body_type: s.body_type,
            race: s.race,
            deity: s.deity,
            guild_id: s.guild_id,
            guild_server_id: s.guild_server_id,
            class_: s.class_,
            pet_owner_id: s.pet_owner_id,
            equip_data: s.equip_data,
            pos_data: s.pos_data,
            level: s.level,
            npc: s.npc,
            other_data: s.other_data,
            char_properties: s.char_properties,
            cur_hp: s.cur_hp,
            holding: s.holding,
            state: s.state,
            light: s.light,
            is_mercenary: s.is_mercenary,
            x: 0, y: 0, z: 0, max_hp: 0, heading: 0,
        },
        Err(_) => spawn_err(),
    }
}

// eql: zone-spawn decodes id/name/decoded-pos/level/hp; the rest stays zero.
// This one keeps a cfg-split (eql's ZoneSpawn is a different shape than Live's
// Spawn — decoded x/y/z vs raw pos arrays).
#[cfg(feature = "backend-eql")]
fn decode_spawn(bytes: &[u8]) -> ffi::Spawn {
    match seq_backend_eql::parse_zone_spawn(bytes) {
        Ok(s) => ffi::Spawn {
            ok: true,
            name: s.name,
            last_name: s.last_name,
            title: s.title,
            suffix: s.suffix,
            spawn_id: u32::from(s.id),
            race: s.race,
            class_: s.class_,
            deity: s.deity,
            guild_id: s.guild_id,
            guild_server_id: s.guild_server_id,
            pet_owner_id: s.pet_owner_id,
            body_type: s.body_type,
            level: s.level,
            npc: s.npc,
            holding: s.holding,
            state: s.state,
            light: s.light,
            cur_hp: s.cur_hp,
            max_hp: s.max_hp,
            x: s.x,
            y: s.y,
            z: s.z,
            heading: s.heading,
            ..spawn_err()
        },
        Err(_) => spawn_err(),
    }
}

// Stage A+6 — second small-fixed POD batch.

fn decode_wear_change(bytes: &[u8]) -> ffi::WearChange {
    match backend::parse_wear_change(bytes) {
        Ok(w) => ffi::WearChange {
            spawn_id: w.spawn_id, subcommand: w.subcommand,
            arg1: w.arg1, arg2: w.arg2, arg3: w.arg3, ok: true,
        },
        Err(_) => ffi::WearChange {
            spawn_id: 0, subcommand: 0, arg1: 0, arg2: 0, arg3: 0, ok: false,
        },
    }
}

fn decode_zone_change(bytes: &[u8]) -> ffi::ZoneChange {
    match backend::parse_zone_change(bytes) {
        Ok(z) => ffi::ZoneChange {
            name: z.name, zone_id: z.zone_id,
            zone_instance: z.zone_instance, ok: true,
        },
        Err(_) => ffi::ZoneChange {
            name: String::new(), zone_id: 0, zone_instance: 0, ok: false,
        },
    }
}

fn decode_dz_info(bytes: &[u8]) -> ffi::DzInfo {
    match backend::parse_dz_info(bytes) {
        Ok(d) => ffi::DzInfo {
            new_dz: d.new_dz, max_players: d.max_players,
            dz_name: d.dz_name, name: d.name, ok: true,
        },
        Err(_) => ffi::DzInfo {
            new_dz: 0, max_players: 0,
            dz_name: String::new(), name: String::new(), ok: false,
        },
    }
}

fn decode_dz_switch_info(bytes: &[u8]) -> ffi::DzSwitch {
    match backend::parse_dz_switch_info(bytes) {
        Ok(s) => ffi::DzSwitch {
            zone_id: s.zone_id, instance_id: s.instance_id, kind: s.kind,
            x: s.x, y: s.y, z: s.z, ok: true,
        },
        Err(_) => ffi::DzSwitch {
            zone_id: 0, instance_id: 0, kind: 0,
            x: 0.0, y: 0.0, z: 0.0, ok: false,
        },
    }
}

fn decode_start_cast(bytes: &[u8]) -> ffi::StartCast {
    match backend::parse_start_cast(bytes) {
        Ok(s) => ffi::StartCast {
            slot: s.slot, spell_id: s.spell_id, target_id: s.target_id, ok: true,
        },
        Err(_) => ffi::StartCast {
            slot: 0, spell_id: 0, target_id: 0, ok: false,
        },
    }
}

fn decode_action(bytes: &[u8]) -> ffi::Action {
    match backend::parse_action(bytes) {
        Ok(a) => ffi::Action {
            target: a.target, source: a.source, spell: a.spell,
            level: a.level, kind: a.kind, ok: true,
        },
        Err(_) => ffi::Action {
            target: 0, source: 0, spell: 0, level: 0, kind: 0, ok: false,
        },
    }
}

fn decode_action_alt(bytes: &[u8]) -> ffi::Action {
    match backend::parse_action_alt(bytes) {
        Ok(a) => ffi::Action {
            target: a.target, source: a.source, spell: a.spell,
            level: a.level, kind: a.kind, ok: true,
        },
        Err(_) => ffi::Action {
            target: 0, source: 0, spell: 0, level: 0, kind: 0, ok: false,
        },
    }
}

fn decode_group_disband(bytes: &[u8]) -> ffi::GroupDisband {
    match backend::parse_group_disband(bytes) {
        Ok(g) => ffi::GroupDisband {
            yourname: g.yourname, membername: g.membername, ok: true,
        },
        Err(_) => ffi::GroupDisband {
            yourname: String::new(), membername: String::new(), ok: false,
        },
    }
}

fn decode_group_follow(bytes: &[u8]) -> ffi::GroupFollow {
    match backend::parse_group_follow(bytes) {
        Ok(g) => ffi::GroupFollow { name: g.name, ok: true },
        Err(_) => ffi::GroupFollow { name: String::new(), ok: false },
    }
}

fn decode_group_member_list(bytes: &[u8]) -> ffi::GroupMemberList {
    match backend::group_member_list::parse_group_member_list(bytes) {
        Ok(g) => ffi::GroupMemberList {
            group_id: g.group_id,
            member_count: g.member_count,
            names: g.names.join("\n"),
            ok: true,
        },
        Err(_) => ffi::GroupMemberList {
            group_id: 0,
            member_count: 0,
            names: String::new(),
            ok: false,
        },
    }
}

fn decode_corpse_loc(bytes: &[u8]) -> ffi::CorpseLoc {
    match backend::parse_corpse_loc(bytes) {
        Ok(c) => ffi::CorpseLoc {
            spawn_id: c.spawn_id, x: c.x, y: c.y, z: c.z, ok: true,
        },
        Err(_) => ffi::CorpseLoc {
            spawn_id: 0, x: 0.0, y: 0.0, z: 0.0, ok: false,
        },
    }
}

// Stage A+7

fn decode_door(bytes: &[u8]) -> ffi::Door {
    match backend::parse_door(bytes) {
        Ok(d) => ffi::Door {
            name: d.name,
            y: d.y, x: d.x, z: d.z, heading: d.heading,
            incline: d.incline, size: d.size,
            door_id: d.door_id, opentype: d.opentype,
            spawnstate: d.spawnstate, invertstate: d.invertstate,
            zone_point: d.zone_point,
            ok: true,
        },
        Err(_) => ffi::Door {
            name: String::new(),
            y: 0.0, x: 0.0, z: 0.0, heading: 0.0,
            incline: 0, size: 0,
            door_id: 0, opentype: 0, spawnstate: 0, invertstate: 0,
            zone_point: 0,
            ok: false,
        },
    }
}

fn decode_ground_spawn(bytes: &[u8]) -> ffi::GroundSpawn {
    match backend::parse_ground_spawn(bytes) {
        Ok(g) => ffi::GroundSpawn {
            drop_id: g.drop_id,
            id_file: g.id_file,
            heading: g.heading, y: g.y, x: g.x, z: g.z,
            bytes_consumed: g.bytes_consumed,
            ok: true,
        },
        Err(_) => ffi::GroundSpawn {
            drop_id: 0,
            id_file: String::new(),
            heading: 0.0, y: 0.0, x: 0.0, z: 0.0,
            bytes_consumed: 0,
            ok: false,
        },
    }
}

fn decode_zone_point(bytes: &[u8]) -> ffi::ZonePoint {
    match backend::parse_zone_point(bytes) {
        Ok(p) => ffi::ZonePoint {
            zone_trigger: p.zone_trigger,
            y: p.y, x: p.x, z: p.z, heading: p.heading,
            zone_id: p.zone_id, zone_instance: p.zone_instance,
            ok: true,
        },
        Err(_) => ffi::ZonePoint {
            zone_trigger: 0,
            y: 0.0, x: 0.0, z: 0.0, heading: 0.0,
            zone_id: 0, zone_instance: 0,
            ok: false,
        },
    }
}

fn decode_simple_message(bytes: &[u8]) -> ffi::SimpleMessage {
    match backend::parse_simple_message(bytes) {
        Ok(m) => ffi::SimpleMessage {
            message_format: m.message_format,
            message_color:  m.message_color,
            ok: true,
        },
        Err(_) => ffi::SimpleMessage {
            message_format: 0, message_color: 0, ok: false,
        },
    }
}

fn formatted_message_err() -> ffi::FormattedMessage {
    ffi::FormattedMessage {
        message_format: 0, message_color: 0,
        spell_id: 0, msg_type: 0, spawn_id: 0, format_id: 0,
        args: Vec::new(), ok: false,
    }
}

// live/test: stock formattedMessageStruct (format id + chat colour); the EQL
// enrichment fields stay empty.
#[cfg(not(feature = "backend-eql"))]
fn decode_formatted_message(bytes: &[u8]) -> ffi::FormattedMessage {
    match backend::parse_formatted_message(bytes) {
        Ok(m) => ffi::FormattedMessage {
            message_format: m.message_format,
            message_color:  m.message_color,
            spell_id: 0, msg_type: 0, spawn_id: 0, format_id: 0,
            args: Vec::new(), ok: true,
        },
        Err(_) => formatted_message_err(),
    }
}

// eql: 0x15d0 (07/14) carries a stock length-prefixed FormattedMessage —
// formatId@5, msgType/colour@9, length-prefixed args@13 (see the parser). The
// args are already positional (empty slots dropped, links cleaned); the daemon
// interpolates via EQStr::formatMessage(format_id, args). message_format/
// message_color mirror format_id/msg_color for stock symbol compatibility.
#[cfg(feature = "backend-eql")]
fn decode_formatted_message(bytes: &[u8]) -> ffi::FormattedMessage {
    match backend::parse_formatted_message(bytes) {
        Ok(m) => ffi::FormattedMessage {
            message_format: m.format_id,
            message_color:  m.msg_color,
            spell_id: 0,
            msg_type: 0,
            spawn_id: 0,
            format_id: m.format_id,
            args: m.args,
            ok: true,
        },
        Err(_) => formatted_message_err(),
    }
}

fn decode_special_message(bytes: &[u8]) -> ffi::SpecialMessage {
    match backend::parse_special_message(bytes) {
        Ok(m) => ffi::SpecialMessage {
            message_color: m.message_color,
            target: m.target,
            source: m.source,
            message: m.message,
            ok: true,
        },
        Err(_) => ffi::SpecialMessage {
            message_color: 0,
            target: 0,
            source: String::new(),
            message: String::new(),
            ok: false,
        },
    }
}

fn decode_channel_message(bytes: &[u8]) -> ffi::ChannelMessage {
    match backend::parse_channel_message(bytes) {
        Ok(m) => ffi::ChannelMessage {
            sender: m.sender,
            target: m.target,
            language: m.language,
            chan_num: m.chan_num,
            skill_in_language: m.skill_in_language,
            message: m.message,
            ok: true,
        },
        Err(_) => ffi::ChannelMessage {
            sender: String::new(),
            target: String::new(),
            language: 0,
            chan_num: 0,
            skill_in_language: 0,
            message: String::new(),
            ok: false,
        },
    }
}

fn decode_player_profile(bytes: &[u8]) -> ffi::PlayerProfile {
    let parsed = backend::parse_player_profile(bytes);
    match parsed {
        Ok(p) => ffi::PlayerProfile {
            ok: true,
            bytes_consumed: p.bytes_consumed,
            checksum: p.checksum,
            gender: p.gender,
            race: p.race,
            class_: p.class_,
            level: p.level,
            level1: p.level1,
            bind0_zone_id: p.bind0_zone_id,
            bind0_x: p.bind0_x, bind0_y: p.bind0_y,
            bind0_z: p.bind0_z, bind0_heading: p.bind0_heading,
            deity: p.deity,
            intoxication: p.intoxication,
            points: p.points,
            mana: p.mana,
            cur_hp: p.cur_hp,
            str_: p.str_, sta: p.sta, cha: p.cha, dex: p.dex,
            int_: p.int_, agi: p.agi, wis: p.wis,
            aa_ids: p.aa_ids,
            aa_values: p.aa_values,
            disciplines: p.disciplines,
            recast_timers: p.recast_timers,
            spell_book: p.spell_book,
            mem_spells: p.mem_spells,
            spell_slot_refresh: p.spell_slot_refresh,
            buff_spell_ids: p.buff_spell_ids,
            buff_durations: p.buff_durations,
            platinum: p.platinum, gold: p.gold,
            silver: p.silver, copper: p.copper,
            platinum_cursor: p.platinum_cursor, gold_cursor: p.gold_cursor,
            silver_cursor: p.silver_cursor, copper_cursor: p.copper_cursor,
            aa_spent: p.aa_spent,
            aa_assigned: p.aa_assigned,
            aa_unspent: p.aa_unspent,
            endurance: p.endurance,
            exp_aa: p.exp_aa,
            name: p.name,
            last_name: p.last_name,
            birthday_time: p.birthday_time,
            account_create_date: p.account_create_date,
            last_save_time: p.last_save_time,
            time_played_min: p.time_played_min,
            expansions: p.expansions,
            languages: p.languages,
            zone_id: p.zone_id,
            zone_instance: p.zone_instance,
            x: p.x, y: p.y, z: p.z, heading: p.heading,
            stand_state: p.stand_state,
            anon: p.anon,
            guild_id: p.guild_id,
            guild_server_id: p.guild_server_id,
            platinum_inventory: p.platinum_inventory,
            gold_inventory: p.gold_inventory,
            silver_inventory: p.silver_inventory,
            copper_inventory: p.copper_inventory,
            platinum_bank: p.platinum_bank, gold_bank: p.gold_bank,
            silver_bank: p.silver_bank, copper_bank: p.copper_bank,
            platinum_shared: p.platinum_shared,
            career_tribute: p.career_tribute,
            current_tribute: p.current_tribute,
            current_rad_crystals: p.current_rad_crystals,
            career_rad_crystals: p.career_rad_crystals,
            current_ebon_crystals: p.current_ebon_crystals,
            career_ebon_crystals: p.career_ebon_crystals,
            autosplit: p.autosplit,
            ldon_guk_points: p.ldon_guk_points,
            ldon_mir_points: p.ldon_mir_points,
            ldon_mmc_points: p.ldon_mmc_points,
            ldon_ruj_points: p.ldon_ruj_points,
            ldon_tak_points: p.ldon_tak_points,
            ldon_avail_points: p.ldon_avail_points,
        },
        Err(_) => ffi::PlayerProfile {
            ok: false,
            bytes_consumed: 0,
            checksum: 0,
            gender: 0, race: 0, class_: 0, level: 0, level1: 0,
            bind0_zone_id: 0,
            bind0_x: 0.0, bind0_y: 0.0, bind0_z: 0.0, bind0_heading: 0.0,
            deity: 0, intoxication: 0, points: 0, mana: 0, cur_hp: 0,
            str_: 0, sta: 0, cha: 0, dex: 0, int_: 0, agi: 0, wis: 0,
            aa_ids: Vec::new(),
            aa_values: Vec::new(),
            disciplines: Vec::new(),
            recast_timers: Vec::new(),
            spell_book: Vec::new(),
            mem_spells: Vec::new(),
            spell_slot_refresh: Vec::new(),
            buff_spell_ids: Vec::new(),
            buff_durations: Vec::new(),
            platinum: 0, gold: 0, silver: 0, copper: 0,
            platinum_cursor: 0, gold_cursor: 0,
            silver_cursor: 0, copper_cursor: 0,
            aa_spent: 0, aa_assigned: 0, aa_unspent: 0,
            endurance: 0, exp_aa: 0,
            name: String::new(),
            last_name: String::new(),
            birthday_time: 0, account_create_date: 0,
            last_save_time: 0, time_played_min: 0, expansions: 0,
            languages: Vec::new(),
            zone_id: 0, zone_instance: 0,
            x: 0.0, y: 0.0, z: 0.0, heading: 0.0,
            stand_state: 0, anon: 0,
            guild_id: 0, guild_server_id: 0,
            platinum_inventory: 0, gold_inventory: 0,
            silver_inventory: 0, copper_inventory: 0,
            platinum_bank: 0, gold_bank: 0,
            silver_bank: 0, copper_bank: 0,
            platinum_shared: 0,
            career_tribute: 0, current_tribute: 0,
            current_rad_crystals: 0, career_rad_crystals: 0,
            current_ebon_crystals: 0, career_ebon_crystals: 0,
            autosplit: 0,
            ldon_guk_points: 0, ldon_mir_points: 0, ldon_mmc_points: 0,
            ldon_ruj_points: 0, ldon_tak_points: 0, ldon_avail_points: 0,
        },
    }
}

fn decode_new_zone(bytes: &[u8]) -> ffi::NewZone {
    let parsed = backend::parse_new_zone(bytes);
    match parsed {
        Ok(z) => ffi::NewZone {
            short_name: z.short_name,
            long_name:  z.long_name,
            zonefile:   z.zonefile,
            zone_exp_multiplier: z.zone_exp_multiplier,
            safe_y: z.safe_y, safe_x: z.safe_x, safe_z: z.safe_z,
            zone_id: z.zone_id,
            ok: true,
        },
        Err(_) => ffi::NewZone {
            short_name: String::new(),
            long_name:  String::new(),
            zonefile:   String::new(),
            zone_exp_multiplier: 0.0,
            safe_y: 0.0, safe_x: 0.0, safe_z: 0.0,
            zone_id: 0,
            ok: false,
        },
    }
}

// Stage A+8

fn decode_player_self_pos(bytes: &[u8]) -> ffi::PlayerSelfPos {
    let parsed = backend::parse_player_self_pos(bytes);
    match parsed {
        Ok(p) => ffi::PlayerSelfPos {
            spawn_id: p.spawn_id,
            x: p.x, y: p.y, z: p.z,
            delta_x: p.delta_x, delta_y: p.delta_y, delta_z: p.delta_z,
            heading: p.heading, delta_heading: p.delta_heading,
            animation: p.animation, pitch: p.pitch,
            ok: true,
        },
        Err(_) => ffi::PlayerSelfPos {
            spawn_id: 0,
            x: 0.0, y: 0.0, z: 0.0,
            delta_x: 0.0, delta_y: 0.0, delta_z: 0.0,
            heading: 0, delta_heading: 0, animation: 0, pitch: 0,
            ok: false,
        },
    }
}

fn decode_player_spawn_pos(bytes: &[u8]) -> ffi::PlayerSpawnPos {
    match backend::parse_player_spawn_pos(bytes) {
        Ok(p) => ffi::PlayerSpawnPos {
            spawn_id: p.spawn_id, spawn_id2: p.spawn_id2,
            x: p.x, y: p.y, z: p.z,
            delta_x: p.delta_x, delta_y: p.delta_y, delta_z: p.delta_z,
            heading: p.heading, delta_heading: p.delta_heading,
            animation: p.animation, pitch: p.pitch,
            ok: true,
        },
        Err(_) => ffi::PlayerSpawnPos {
            spawn_id: 0, spawn_id2: 0,
            x: 0, y: 0, z: 0,
            delta_x: 0, delta_y: 0, delta_z: 0,
            heading: 0, delta_heading: 0, animation: 0, pitch: 0,
            ok: false,
        },
    }
}

fn decode_npc_move_update(bytes: &[u8]) -> ffi::NpcMove {
    match backend::parse_npc_move_update(bytes) {
        Ok(n) => ffi::NpcMove {
            spawn_id: n.spawn_id,
            x: n.x, y: n.y, z: n.z, heading: n.heading,
            delta_x: n.delta_x, delta_y: n.delta_y, delta_z: n.delta_z,
            delta_heading: n.delta_heading, animation: n.animation,
            ok: true,
        },
        Err(_) => ffi::NpcMove {
            spawn_id: 0,
            x: 0, y: 0, z: 0, heading: 0,
            delta_x: 0, delta_y: 0, delta_z: 0,
            delta_heading: 0, animation: 0,
            ok: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_zero_payload() {
        let r = decode_mob_update(&[0u8; 14]);
        assert!(r.ok);
        assert_eq!(r.spawn_id, 0);
        assert_eq!(r.x, 0);
    }

    #[test]
    fn bad_length_returns_ok_false() {
        let r = decode_mob_update(&[0u8; 13]);
        assert!(!r.ok);
    }

    #[test]
    fn delete_spawn_roundtrip() {
        let bytes = [0xEF, 0xBE, 0xAD, 0xDE];
        let r = decode_delete_spawn(&bytes);
        assert!(r.ok);
        assert_eq!(r.spawn_id, 0xDEADBEEF);
    }

    #[test]
    fn delete_spawn_bad_length_returns_ok_false() {
        let r = decode_delete_spawn(&[0u8; 3]);
        assert!(!r.ok);
    }
}
