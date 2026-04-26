//! C++ FFI bridge — exposes `seq-decode` parsers across the cxx ABI.
//!
//! Stage A: `decode_mob_update` only. The bridge is intentionally a
//! thin shim — keep parsing logic in `seq-decode` so it stays usable
//! from pure Rust contexts (replay tools, future standalone daemon).

#[cxx::bridge(namespace = "seq::rust")]
mod ffi {
    // Stage A placeholder — replaced by `MobUpdateOut` + `decode_mob_update`
    // in the next task. Kept so the cxx-build pipeline compiles end-to-end
    // before the parser lands.
    extern "Rust" {
        fn placeholder() -> i32;
    }
}

fn placeholder() -> i32 {
    0
}
