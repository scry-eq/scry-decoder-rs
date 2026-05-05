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

    extern "Rust" {
        fn decode_mob_update(bytes: &[u8]) -> MobUpdateOut;
        fn decode_delete_spawn(bytes: &[u8]) -> DeleteSpawnOut;
        fn decode_spawn(bytes: &[u8]) -> SpawnOut;
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
