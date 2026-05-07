//! C++ FFI bridge — exposes `seq-decode` parsers across the cxx ABI.
//!
//! The bridge is intentionally a thin shim. Parsing logic stays in
//! `seq-decode` so it remains usable from pure-Rust contexts (replay
//! tools, the eventual standalone daemon). This crate is the
//! `staticlib` Corrosion links into `seq-daemon-core`.

#[cxx::bridge(namespace = "seq::rust")]
mod ffi {
    /// Plain-old-data result of a `decode_mob_update` call. `ok` is the
    /// discriminator: when false, the remaining fields are zeroed and
    /// the caller should fall back to the C++ path. We use this rather
    /// than cxx's `Result` mapping because exception machinery isn't
    /// worth the ergonomic win for one call site that only fails on
    /// a length mismatch the daemon's SZC_Match dispatch already
    /// guarantees can't happen.
    struct MobUpdateOut {
        spawn_id: u16,
        x: i32,
        y: i32,
        z: i32,
        heading: u16,
        ok: bool,
    }

    /// Plain-old-data result of `decode_delete_spawn`. `ok=false` means
    /// the payload was the wrong size; SZC_Match in the daemon already
    /// guards against that so it shouldn't fire in normal operation.
    struct DeleteSpawnOut {
        spawn_id: u32,
        ok: bool,
    }

    /// Variable-length spawn payload (Stage A+2). Mirrors the fields
    /// `SpawnShell::fillSpawnStruct` populates on a `spawnStruct`. The
    /// daemon assigns each field into its own struct on the C++ side.
    /// `bytes_consumed` lets the daemon log/check expected length the
    /// same way the C++ path does.
    struct SpawnOut {
        ok: bool,
        bytes_consumed: u32,

        // Strings — NUL-padded; daemon strcpy()s up to first NUL into
        // its fixed-size char buffers (everquest.h spawnStruct).
        name: [u8; 64],
        last_name: [u8; 32],
        title: [u8; 32],
        suffix: [u8; 32],

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
        pos_data: [u32; 6],

        level: u8,
        npc: u8,
        other_data: u8,
        char_properties: u8,
        cur_hp: u8,
        holding: u8,
        state: u8,
        light: u8,
        is_mercenary: u8,
    }

    // Stage A+3 — small fixed-size opcodes. Each Out struct ends with
    // an `ok` flag mirroring DeleteSpawnOut; the daemon falls back to
    // its C++ struct cast when ok=false (SZC_Match should already
    // prevent that, but fall-back keeps the gate non-fatal).
    struct RemoveSpawnOut {
        spawn_id: u32,
        remove_spawn: u8,
        ok: bool,
    }
    struct HpUpdateOut {
        spawn_id: u16,
        cur_hp: i32,
        max_hp: i32,
        ok: bool,
    }
    struct MobHealthOut {
        spawn_id: u16,
        hp_percent: i32,
        ok: bool,
    }
    struct SpawnAppearanceOut {
        spawn_id: u16,
        kind: u16,
        parameter: u32,
        ok: bool,
    }
    struct ExpUpdateOut {
        exp: u32,
        unknown0: u32,
        kind: u32,
        unknown1: u32,
        ok: bool,
    }
    struct LevelUpdateOut {
        level: u32,
        level_old: u32,
        exp: u32,
        unknown0: u32,
        ok: bool,
    }
    struct SkillUpdateOut {
        skill_id: u32,
        value: i32,
        ok: bool,
    }

    // Stage A+4 — 7 more small fixed-size opcodes.
    struct ManaChangeOut {
        new_mana: i32,
        max_mana: i32,
        spell_id: i32,
        ok: bool,
    }
    struct StaminaOut { food: u32, water: u32, ok: bool }
    struct EndUpdateOut { spawn_id: u16, cur: u32, max: u32, ok: bool }
    struct ConsiderOut {
        player_id: u32, target_id: u32, faction: i32, level: i32, ok: bool,
    }
    struct SpawnRenameOut {
        old_name: [u8; 64],
        old_name_again: [u8; 64],
        new_name: [u8; 64],
        ok: bool,
    }
    struct ClientTargetOut { new_target: u32, ok: bool }
    struct DeathOut {
        spawn_id: u32, killer_id: u32, corpse_id: u32, kind: i32,
        spell_id: u32, zone_id: u16, zone_instance: u16, damage: u32,
        ok: bool,
    }

    // Stage A+5
    struct ClickObjectOut { drop_id: u16, spawn_id: u16, ok: bool }
    struct IllusionOut {
        spawn_id: u32, name: [u8; 64], race: u32, gender: u8,
        texture: u8, helm: u8, face: u32, ok: bool,
    }
    struct BuffOut {
        spawn_id: u32, spell_id: u32, duration: u32, level: i8,
        spell_slot: u32, change_type: u32, ok: bool,
    }
    struct Action2Out {
        target: u16, source: u16, damage: i32, spell: i32, kind: u8, ok: bool,
    }

    // Stage A+6 — second small-fixed POD batch.
    struct WearChangeOut {
        spawn_id: u16, subcommand: u16, arg1: i16, arg2: i16, arg3: u8,
        ok: bool,
    }
    struct ZoneChangeOut {
        name: [u8; 64], zone_id: u16, zone_instance: u16, ok: bool,
    }
    struct DzInfoOut {
        new_dz: u8, max_players: u32, dz_name: [u8; 128], name: [u8; 64],
        ok: bool,
    }
    struct DzSwitchOut {
        zone_id: u16, instance_id: u16, kind: u32,
        x: f32, y: f32, z: f32, ok: bool,
    }
    struct StartCastOut {
        slot: i32, spell_id: u32, target_id: u32, ok: bool,
    }
    struct ActionOut {
        target: u16, source: u16, spell: i16, level: u8, kind: u8, ok: bool,
    }
    struct GroupDisbandOut {
        yourname: [u8; 64], membername: [u8; 64], ok: bool,
    }
    struct GroupFollowOut { name: [u8; 64], ok: bool }
    struct CorpseLocOut {
        spawn_id: u32, x: f32, y: f32, z: f32, ok: bool,
    }

    // Stage A+7 — variable-length / array opcodes.
    struct DoorOut {
        name: [u8; 32],
        y: f32, x: f32, z: f32, heading: f32,
        incline: u32, size: u32,
        door_id: u8, opentype: u8, spawnstate: u8, invertstate: u8,
        zone_point: u32,
        ok: bool,
    }
    struct GroundSpawnOut {
        drop_id: u32,
        id_file: [u8; 30],
        heading: f32, y: f32, x: f32, z: f32,
        bytes_consumed: u32,
        ok: bool,
    }

    extern "Rust" {
        fn decode_mob_update(bytes: &[u8]) -> MobUpdateOut;
        fn decode_delete_spawn(bytes: &[u8]) -> DeleteSpawnOut;
        fn decode_spawn(bytes: &[u8]) -> SpawnOut;
        fn decode_remove_spawn(bytes: &[u8]) -> RemoveSpawnOut;
        fn decode_hp_update(bytes: &[u8]) -> HpUpdateOut;
        fn decode_mob_health(bytes: &[u8]) -> MobHealthOut;
        fn decode_spawn_appearance(bytes: &[u8]) -> SpawnAppearanceOut;
        fn decode_exp_update(bytes: &[u8]) -> ExpUpdateOut;
        fn decode_level_update(bytes: &[u8]) -> LevelUpdateOut;
        fn decode_skill_update(bytes: &[u8]) -> SkillUpdateOut;
        fn decode_mana_change(bytes: &[u8]) -> ManaChangeOut;
        fn decode_stamina(bytes: &[u8]) -> StaminaOut;
        fn decode_end_update(bytes: &[u8]) -> EndUpdateOut;
        fn decode_consider(bytes: &[u8]) -> ConsiderOut;
        fn decode_spawn_rename(bytes: &[u8]) -> SpawnRenameOut;
        fn decode_client_target(bytes: &[u8]) -> ClientTargetOut;
        fn decode_death(bytes: &[u8]) -> DeathOut;
        fn decode_click_object(bytes: &[u8]) -> ClickObjectOut;
        fn decode_illusion(bytes: &[u8]) -> IllusionOut;
        fn decode_buff(bytes: &[u8]) -> BuffOut;
        fn decode_action2(bytes: &[u8]) -> Action2Out;
        // Stage A+6
        fn decode_wear_change(bytes: &[u8]) -> WearChangeOut;
        fn decode_zone_change(bytes: &[u8]) -> ZoneChangeOut;
        fn decode_dz_info(bytes: &[u8]) -> DzInfoOut;
        fn decode_dz_switch_info(bytes: &[u8]) -> DzSwitchOut;
        fn decode_start_cast(bytes: &[u8]) -> StartCastOut;
        fn decode_action(bytes: &[u8]) -> ActionOut;
        fn decode_action_alt(bytes: &[u8]) -> ActionOut;
        fn decode_group_disband(bytes: &[u8]) -> GroupDisbandOut;
        fn decode_group_follow(bytes: &[u8]) -> GroupFollowOut;
        fn decode_corpse_loc(bytes: &[u8]) -> CorpseLocOut;
        // Stage A+7
        fn decode_door(bytes: &[u8]) -> DoorOut;
        fn decode_ground_spawn(bytes: &[u8]) -> GroundSpawnOut;
    }
}

fn decode_mob_update(bytes: &[u8]) -> ffi::MobUpdateOut {
    match seq_decode::parse_mob_update(bytes) {
        Ok(m) => ffi::MobUpdateOut {
            spawn_id: m.spawn_id,
            x: m.x,
            y: m.y,
            z: m.z,
            heading: m.heading,
            ok: true,
        },
        Err(_) => ffi::MobUpdateOut {
            spawn_id: 0,
            x: 0,
            y: 0,
            z: 0,
            heading: 0,
            ok: false,
        },
    }
}

fn decode_delete_spawn(bytes: &[u8]) -> ffi::DeleteSpawnOut {
    match seq_decode::parse_delete_spawn(bytes) {
        Ok(d) => ffi::DeleteSpawnOut { spawn_id: d.spawn_id, ok: true },
        Err(_) => ffi::DeleteSpawnOut { spawn_id: 0, ok: false },
    }
}

fn decode_remove_spawn(bytes: &[u8]) -> ffi::RemoveSpawnOut {
    match seq_decode::parse_remove_spawn(bytes) {
        Ok(r) => ffi::RemoveSpawnOut {
            spawn_id: r.spawn_id, remove_spawn: r.remove_spawn, ok: true,
        },
        Err(_) => ffi::RemoveSpawnOut { spawn_id: 0, remove_spawn: 0, ok: false },
    }
}

fn decode_hp_update(bytes: &[u8]) -> ffi::HpUpdateOut {
    match seq_decode::parse_hp_update(bytes) {
        Ok(h) => ffi::HpUpdateOut {
            spawn_id: h.spawn_id, cur_hp: h.cur_hp, max_hp: h.max_hp, ok: true,
        },
        Err(_) => ffi::HpUpdateOut {
            spawn_id: 0, cur_hp: 0, max_hp: 0, ok: false,
        },
    }
}

fn decode_mob_health(bytes: &[u8]) -> ffi::MobHealthOut {
    match seq_decode::parse_mob_health(bytes) {
        Ok(m) => ffi::MobHealthOut {
            spawn_id: m.spawn_id, hp_percent: m.hp_percent, ok: true,
        },
        Err(_) => ffi::MobHealthOut { spawn_id: 0, hp_percent: 0, ok: false },
    }
}

fn decode_spawn_appearance(bytes: &[u8]) -> ffi::SpawnAppearanceOut {
    match seq_decode::parse_spawn_appearance(bytes) {
        Ok(a) => ffi::SpawnAppearanceOut {
            spawn_id: a.spawn_id, kind: a.kind, parameter: a.parameter, ok: true,
        },
        Err(_) => ffi::SpawnAppearanceOut {
            spawn_id: 0, kind: 0, parameter: 0, ok: false,
        },
    }
}

fn decode_exp_update(bytes: &[u8]) -> ffi::ExpUpdateOut {
    match seq_decode::parse_exp_update(bytes) {
        Ok(e) => ffi::ExpUpdateOut {
            exp: e.exp, unknown0: e.unknown0, kind: e.kind, unknown1: e.unknown1,
            ok: true,
        },
        Err(_) => ffi::ExpUpdateOut {
            exp: 0, unknown0: 0, kind: 0, unknown1: 0, ok: false,
        },
    }
}

fn decode_level_update(bytes: &[u8]) -> ffi::LevelUpdateOut {
    match seq_decode::parse_level_update(bytes) {
        Ok(l) => ffi::LevelUpdateOut {
            level: l.level, level_old: l.level_old, exp: l.exp,
            unknown0: l.unknown0, ok: true,
        },
        Err(_) => ffi::LevelUpdateOut {
            level: 0, level_old: 0, exp: 0, unknown0: 0, ok: false,
        },
    }
}

fn decode_skill_update(bytes: &[u8]) -> ffi::SkillUpdateOut {
    match seq_decode::parse_skill_update(bytes) {
        Ok(s) => ffi::SkillUpdateOut {
            skill_id: s.skill_id, value: s.value, ok: true,
        },
        Err(_) => ffi::SkillUpdateOut { skill_id: 0, value: 0, ok: false },
    }
}

fn decode_mana_change(bytes: &[u8]) -> ffi::ManaChangeOut {
    match seq_decode::parse_mana_change(bytes) {
        Ok(m) => ffi::ManaChangeOut {
            new_mana: m.new_mana, max_mana: m.max_mana, spell_id: m.spell_id, ok: true,
        },
        Err(_) => ffi::ManaChangeOut {
            new_mana: 0, max_mana: 0, spell_id: 0, ok: false,
        },
    }
}

fn decode_stamina(bytes: &[u8]) -> ffi::StaminaOut {
    match seq_decode::parse_stamina(bytes) {
        Ok(s) => ffi::StaminaOut { food: s.food, water: s.water, ok: true },
        Err(_) => ffi::StaminaOut { food: 0, water: 0, ok: false },
    }
}

fn decode_end_update(bytes: &[u8]) -> ffi::EndUpdateOut {
    match seq_decode::parse_end_update(bytes) {
        Ok(e) => ffi::EndUpdateOut {
            spawn_id: e.spawn_id, cur: e.cur, max: e.max, ok: true,
        },
        Err(_) => ffi::EndUpdateOut { spawn_id: 0, cur: 0, max: 0, ok: false },
    }
}

fn decode_consider(bytes: &[u8]) -> ffi::ConsiderOut {
    match seq_decode::parse_consider(bytes) {
        Ok(c) => ffi::ConsiderOut {
            player_id: c.player_id, target_id: c.target_id,
            faction: c.faction, level: c.level, ok: true,
        },
        Err(_) => ffi::ConsiderOut {
            player_id: 0, target_id: 0, faction: 0, level: 0, ok: false,
        },
    }
}

fn decode_spawn_rename(bytes: &[u8]) -> ffi::SpawnRenameOut {
    match seq_decode::parse_spawn_rename(bytes) {
        Ok(r) => ffi::SpawnRenameOut {
            old_name: r.old_name,
            old_name_again: r.old_name_again,
            new_name: r.new_name,
            ok: true,
        },
        Err(_) => ffi::SpawnRenameOut {
            old_name: [0; 64], old_name_again: [0; 64], new_name: [0; 64],
            ok: false,
        },
    }
}

fn decode_client_target(bytes: &[u8]) -> ffi::ClientTargetOut {
    match seq_decode::parse_client_target(bytes) {
        Ok(t) => ffi::ClientTargetOut { new_target: t.new_target, ok: true },
        Err(_) => ffi::ClientTargetOut { new_target: 0, ok: false },
    }
}

fn decode_death(bytes: &[u8]) -> ffi::DeathOut {
    match seq_decode::parse_death(bytes) {
        Ok(d) => ffi::DeathOut {
            spawn_id: d.spawn_id, killer_id: d.killer_id, corpse_id: d.corpse_id,
            kind: d.kind, spell_id: d.spell_id, zone_id: d.zone_id,
            zone_instance: d.zone_instance, damage: d.damage, ok: true,
        },
        Err(_) => ffi::DeathOut {
            spawn_id: 0, killer_id: 0, corpse_id: 0, kind: 0, spell_id: 0,
            zone_id: 0, zone_instance: 0, damage: 0, ok: false,
        },
    }
}

fn decode_click_object(bytes: &[u8]) -> ffi::ClickObjectOut {
    match seq_decode::parse_click_object(bytes) {
        Ok(c) => ffi::ClickObjectOut {
            drop_id: c.drop_id, spawn_id: c.spawn_id, ok: true,
        },
        Err(_) => ffi::ClickObjectOut { drop_id: 0, spawn_id: 0, ok: false },
    }
}

fn decode_illusion(bytes: &[u8]) -> ffi::IllusionOut {
    match seq_decode::parse_illusion(bytes) {
        Ok(i) => ffi::IllusionOut {
            spawn_id: i.spawn_id, name: i.name, race: i.race,
            gender: i.gender, texture: i.texture, helm: i.helm, face: i.face,
            ok: true,
        },
        Err(_) => ffi::IllusionOut {
            spawn_id: 0, name: [0; 64], race: 0, gender: 0,
            texture: 0, helm: 0, face: 0, ok: false,
        },
    }
}

fn decode_buff(bytes: &[u8]) -> ffi::BuffOut {
    match seq_decode::parse_buff(bytes) {
        Ok(b) => ffi::BuffOut {
            spawn_id: b.spawn_id, spell_id: b.spell_id, duration: b.duration,
            level: b.level, spell_slot: b.spell_slot, change_type: b.change_type,
            ok: true,
        },
        Err(_) => ffi::BuffOut {
            spawn_id: 0, spell_id: 0, duration: 0, level: 0,
            spell_slot: 0, change_type: 0, ok: false,
        },
    }
}

fn decode_action2(bytes: &[u8]) -> ffi::Action2Out {
    match seq_decode::parse_action2(bytes) {
        Ok(a) => ffi::Action2Out {
            target: a.target, source: a.source, damage: a.damage,
            spell: a.spell, kind: a.kind, ok: true,
        },
        Err(_) => ffi::Action2Out {
            target: 0, source: 0, damage: 0, spell: 0, kind: 0, ok: false,
        },
    }
}

fn decode_spawn(bytes: &[u8]) -> ffi::SpawnOut {
    match seq_decode::parse_spawn(bytes) {
        Ok(s) => ffi::SpawnOut {
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
        },
        Err(_) => ffi::SpawnOut {
            ok: false,
            bytes_consumed: 0,
            name: [0; 64],
            last_name: [0; 32],
            title: [0; 32],
            suffix: [0; 32],
            spawn_id: 0, misc_data: 0, body_type: 0, race: 0,
            deity: 0, guild_id: 0, guild_server_id: 0, class_: 0,
            pet_owner_id: 0,
            equip_data: [0; 45],
            pos_data: [0; 6],
            level: 0, npc: 0, other_data: 0, char_properties: 0,
            cur_hp: 0, holding: 0, state: 0, light: 0, is_mercenary: 0,
        },
    }
}

// Stage A+6 — second small-fixed POD batch.

fn decode_wear_change(bytes: &[u8]) -> ffi::WearChangeOut {
    match seq_decode::parse_wear_change(bytes) {
        Ok(w) => ffi::WearChangeOut {
            spawn_id: w.spawn_id, subcommand: w.subcommand,
            arg1: w.arg1, arg2: w.arg2, arg3: w.arg3, ok: true,
        },
        Err(_) => ffi::WearChangeOut {
            spawn_id: 0, subcommand: 0, arg1: 0, arg2: 0, arg3: 0, ok: false,
        },
    }
}

fn decode_zone_change(bytes: &[u8]) -> ffi::ZoneChangeOut {
    match seq_decode::parse_zone_change(bytes) {
        Ok(z) => ffi::ZoneChangeOut {
            name: z.name, zone_id: z.zone_id,
            zone_instance: z.zone_instance, ok: true,
        },
        Err(_) => ffi::ZoneChangeOut {
            name: [0; 64], zone_id: 0, zone_instance: 0, ok: false,
        },
    }
}

fn decode_dz_info(bytes: &[u8]) -> ffi::DzInfoOut {
    match seq_decode::parse_dz_info(bytes) {
        Ok(d) => ffi::DzInfoOut {
            new_dz: d.new_dz, max_players: d.max_players,
            dz_name: d.dz_name, name: d.name, ok: true,
        },
        Err(_) => ffi::DzInfoOut {
            new_dz: 0, max_players: 0,
            dz_name: [0; 128], name: [0; 64], ok: false,
        },
    }
}

fn decode_dz_switch_info(bytes: &[u8]) -> ffi::DzSwitchOut {
    match seq_decode::parse_dz_switch_info(bytes) {
        Ok(s) => ffi::DzSwitchOut {
            zone_id: s.zone_id, instance_id: s.instance_id, kind: s.kind,
            x: s.x, y: s.y, z: s.z, ok: true,
        },
        Err(_) => ffi::DzSwitchOut {
            zone_id: 0, instance_id: 0, kind: 0,
            x: 0.0, y: 0.0, z: 0.0, ok: false,
        },
    }
}

fn decode_start_cast(bytes: &[u8]) -> ffi::StartCastOut {
    match seq_decode::parse_start_cast(bytes) {
        Ok(s) => ffi::StartCastOut {
            slot: s.slot, spell_id: s.spell_id, target_id: s.target_id, ok: true,
        },
        Err(_) => ffi::StartCastOut {
            slot: 0, spell_id: 0, target_id: 0, ok: false,
        },
    }
}

fn decode_action(bytes: &[u8]) -> ffi::ActionOut {
    match seq_decode::parse_action(bytes) {
        Ok(a) => ffi::ActionOut {
            target: a.target, source: a.source, spell: a.spell,
            level: a.level, kind: a.kind, ok: true,
        },
        Err(_) => ffi::ActionOut {
            target: 0, source: 0, spell: 0, level: 0, kind: 0, ok: false,
        },
    }
}

fn decode_action_alt(bytes: &[u8]) -> ffi::ActionOut {
    match seq_decode::parse_action_alt(bytes) {
        Ok(a) => ffi::ActionOut {
            target: a.target, source: a.source, spell: a.spell,
            level: a.level, kind: a.kind, ok: true,
        },
        Err(_) => ffi::ActionOut {
            target: 0, source: 0, spell: 0, level: 0, kind: 0, ok: false,
        },
    }
}

fn decode_group_disband(bytes: &[u8]) -> ffi::GroupDisbandOut {
    match seq_decode::parse_group_disband(bytes) {
        Ok(g) => ffi::GroupDisbandOut {
            yourname: g.yourname, membername: g.membername, ok: true,
        },
        Err(_) => ffi::GroupDisbandOut {
            yourname: [0; 64], membername: [0; 64], ok: false,
        },
    }
}

fn decode_group_follow(bytes: &[u8]) -> ffi::GroupFollowOut {
    match seq_decode::parse_group_follow(bytes) {
        Ok(g) => ffi::GroupFollowOut { name: g.name, ok: true },
        Err(_) => ffi::GroupFollowOut { name: [0; 64], ok: false },
    }
}

fn decode_corpse_loc(bytes: &[u8]) -> ffi::CorpseLocOut {
    match seq_decode::parse_corpse_loc(bytes) {
        Ok(c) => ffi::CorpseLocOut {
            spawn_id: c.spawn_id, x: c.x, y: c.y, z: c.z, ok: true,
        },
        Err(_) => ffi::CorpseLocOut {
            spawn_id: 0, x: 0.0, y: 0.0, z: 0.0, ok: false,
        },
    }
}

// Stage A+7

fn decode_door(bytes: &[u8]) -> ffi::DoorOut {
    match seq_decode::parse_door(bytes) {
        Ok(d) => ffi::DoorOut {
            name: d.name,
            y: d.y, x: d.x, z: d.z, heading: d.heading,
            incline: d.incline, size: d.size,
            door_id: d.door_id, opentype: d.opentype,
            spawnstate: d.spawnstate, invertstate: d.invertstate,
            zone_point: d.zone_point,
            ok: true,
        },
        Err(_) => ffi::DoorOut {
            name: [0; 32],
            y: 0.0, x: 0.0, z: 0.0, heading: 0.0,
            incline: 0, size: 0,
            door_id: 0, opentype: 0, spawnstate: 0, invertstate: 0,
            zone_point: 0,
            ok: false,
        },
    }
}

fn decode_ground_spawn(bytes: &[u8]) -> ffi::GroundSpawnOut {
    match seq_decode::parse_ground_spawn(bytes) {
        Ok(g) => ffi::GroundSpawnOut {
            drop_id: g.drop_id,
            id_file: g.id_file,
            heading: g.heading, y: g.y, x: g.x, z: g.z,
            bytes_consumed: g.bytes_consumed,
            ok: true,
        },
        Err(_) => ffi::GroundSpawnOut {
            drop_id: 0,
            id_file: [0; 30],
            heading: 0.0, y: 0.0, x: 0.0, z: 0.0,
            bytes_consumed: 0,
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
