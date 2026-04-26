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

    extern "Rust" {
        fn decode_mob_update(bytes: &[u8]) -> MobUpdateOut;
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
}
