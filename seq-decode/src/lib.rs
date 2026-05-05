//! Pure parsers for ShowEQ packet payloads.
//!
//! No I/O, no global state, no Qt. Each module exposes a `parse_*`
//! function that turns a `&[u8]` payload into a typed struct (or a
//! `ParseError`). Higher layers (FFI bridge, replay tools, the
//! eventual standalone daemon) compose these.
//!
//! Stage A: `OP_MobUpdate`. Stage A+1: `OP_DeleteSpawn`.
//! Stage A+2: spawn (variable-length payload from `OP_ZoneEntry`
//! server-direction; mirrors `SpawnShell::fillSpawnStruct`).

pub mod cursor;
pub mod delete_spawn;
pub mod mob_update;
pub mod spawn;

pub use delete_spawn::{
    parse_delete_spawn, DeleteSpawn, DeleteSpawnError, PAYLOAD_LEN as DELETE_SPAWN_LEN,
};
pub use mob_update::{
    parse_mob_update, MobUpdate, ParseError, PAYLOAD_LEN as MOB_UPDATE_LEN,
};
pub use spawn::{parse_spawn, Spawn, SpawnError};
