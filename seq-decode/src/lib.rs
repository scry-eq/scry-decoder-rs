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
//! Stage A+3: small fixed-size batch — OP_RemoveSpawn, OP_HPUpdate,
//! OP_MobHealth, OP_SpawnAppearance, OP_ExpUpdate, OP_LevelUpdate,
//! OP_SkillUpdate.

pub mod cursor;
pub mod delete_spawn;
pub mod exp_update;
pub mod hp_update;
pub mod level_update;
pub mod mob_health;
pub mod mob_update;
pub mod remove_spawn;
pub mod skill_update;
pub mod spawn;
pub mod spawn_appearance;

pub use delete_spawn::{
    parse_delete_spawn, DeleteSpawn, DeleteSpawnError, PAYLOAD_LEN as DELETE_SPAWN_LEN,
};
pub use exp_update::{parse_exp_update, ExpUpdate, ExpUpdateError};
pub use hp_update::{parse_hp_update, HpUpdate, HpUpdateError};
pub use level_update::{parse_level_update, LevelUpdate, LevelUpdateError};
pub use mob_health::{parse_mob_health, MobHealth, MobHealthError};
pub use mob_update::{
    parse_mob_update, MobUpdate, ParseError, PAYLOAD_LEN as MOB_UPDATE_LEN,
};
pub use remove_spawn::{parse_remove_spawn, RemoveSpawn, RemoveSpawnError};
pub use skill_update::{parse_skill_update, SkillUpdate, SkillUpdateError};
pub use spawn::{parse_spawn, Spawn, SpawnError};
pub use spawn_appearance::{
    parse_spawn_appearance, SpawnAppearance, SpawnAppearanceError,
};
