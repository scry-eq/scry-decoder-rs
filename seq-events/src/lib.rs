//! Neutral, backend-agnostic decode vocabulary + the backend contract.
//!
//! Every server backend (live/test/eql) decodes its own wire format into these
//! shared types, so the daemon consuming them never learns which server it is
//! talking to. A backend maps its per-server structs (Live `Spawn` vs eql
//! `ZoneSpawn`, different heading conventions, …) into one `Event` shape; the
//! daemon just applies events.
//!
//! This crate holds NO wire-decode logic — only the vocabulary, the trait, and
//! shared neutral math — so a backend depending on it is never coupled to
//! another server's parsers.

/// Packet direction on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Server → client (spawns, zone, profile, …).
    ServerToClient,
    /// Client → server (e.g. the player's own position updates).
    ClientToServer,
}

/// A world position in EQ coordinates; heading already normalized to degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// 0..359, converted from the backend's raw heading bits via [`heading_deg`].
    pub heading_deg: u16,
}

/// A spawn (NPC, PC, or corpse) entering the zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnInfo {
    pub id: u32,
    pub name: String,
    pub last_name: String,
    pub race: u32,
    pub class_: u32,
    pub deity: u32,
    pub level: u8,
    /// Nonzero for NPCs.
    pub npc: u8,
    pub cur_hp: u32,
    /// `None` when the spawn packet carries no max HP (Live — it arrives later
    /// via HP opcodes); `Some` for backends that ship it inline (eql).
    pub max_hp: Option<u32>,
    pub guild_id: u32,
    /// Present when the spawn packet carries position (eql); `None` when
    /// position arrives separately via movement opcodes (Live).
    pub pos: Option<Pos>,
}

/// The local player's character profile (self identity + vitals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub name: String,
    pub last_name: String,
    pub class_: u32,
    pub level: u8,
    pub race: u32,
    pub deity: u32,
    pub cur_hp: u32,
    pub mana: u32,
    /// Purchased-AA descIDs, paired index-for-index with `aa_values` (ranks).
    pub aa_ids: Vec<u32>,
    pub aa_values: Vec<u32>,
}

/// Zone identity from OP_NewZone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneInfo {
    pub short_name: String,
    pub long_name: String,
}

/// One active-buff entry from an OP_BuffList (belongs to the list's owner spawn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffEntry {
    pub spell_id: u32,
    /// Server-side remaining duration in ticks; `<= 0` = permanent.
    pub remaining_ticks: i32,
    /// Buff-window slot index.
    pub slot: u32,
}

/// A single door / static object row from OP_SpawnDoor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A decoded, backend-neutral world event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A spawn entered the zone (OP_ZoneEntry).
    SpawnAdded(SpawnInfo),
    /// A spawn moved (OP_MobUpdate / OP_NpcMoveUpdate).
    SpawnMoved { id: u32, pos: Pos },
    /// A spawn left the zone (OP_RemoveSpawn / OP_DeleteSpawn).
    SpawnRemoved { id: u32 },
    /// A spawn's health changed (OP_HPUpdate). `max` is real HP for the self and
    /// a percentage base (100) for other spawns, mirroring the wire.
    SpawnHp { id: u32, cur: i32, max: i32 },
    /// The local player moved (OP_ClientUpdate self position).
    SelfPos(Pos),
    /// Zone changed (OP_NewZone).
    ZoneChanged(ZoneInfo),
    /// The local player's profile (OP_PlayerProfile).
    PlayerProfile(ProfileInfo),
    /// A batch of doors / static objects (OP_SpawnDoor).
    Doors(Vec<DoorInfo>),
    /// A damage event (OP_Action2). Ids only; the consumer resolves names from
    /// its spawn map. `kind` is the wire damage type; `spell_id` 0 = melee.
    Combat {
        source: u32,
        target: u32,
        kind: u32,
        damage: i32,
        spell_id: u32,
    },
    /// The player selected a target (OP_TargetMouse). `spawn_id` 0 = cleared.
    Targeted { spawn_id: u32 },
    /// The player considered a spawn (OP_Consider) — `spawn_id` is the target.
    Considered { spawn_id: u32 },
    /// One AA definition from the OP_SendAATable burst: maps a purchased AA's
    /// `desc_id` to a `title_sid` (a dbstring type-1 id → the AA's display name).
    AaTable { desc_id: u32, title_sid: u32 },
    /// A player chat message (OP_CommonMessage). `channel` is the MessageType
    /// (0=Guild 2=Group 3=Shout 4=Auction 5=OOC 7=Tell 8=Say 15=Raid). `target`
    /// is meaningful only for tells; `chat_color`/`channel_name` are 0/empty for
    /// channel messages (set by the formatted/UCS paths).
    Chat {
        channel: u32,
        from: String,
        target: String,
        text: String,
        chat_color: u32,
        channel_name: String,
    },
    /// The authoritative active-buff list for one spawn (eql OP_BuffList), sent
    /// at zone-in and on every buff change. A full snapshot: the consumer
    /// REPLACES that owner's buffs. `owner` == the player → the buff panel; a
    /// mob → that mob's effects. `remaining_ticks <= 0` on an entry = permanent.
    BuffList {
        owner: u32,
        entries: Vec<BuffEntry>,
    },
    /// Zone-in boundary marker (OP_EnterWorld) — no payload; the daemon uses it
    /// to reset per-zone state.
    EnterWorld,
}

/// Outcome of decoding one app packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// One neutral event.
    One(Event),
    /// Several events from one packet.
    Many(Vec<Event>),
    /// The opcode was recognized and parsed, but carried nothing to surface
    /// (e.g. an eql stat-sync packet with only player mana/endurance).
    Ignored,
    /// This backend has no decoder for the opcode (caller may still count it).
    Unhandled,
    /// The opcode is handled but its payload failed to parse.
    Malformed,
}

/// The contract every server backend implements. The daemon holds a
/// `Box<dyn Backend>` and never branches on live/test/eql.
pub trait Backend: Send + Sync {
    /// Stable backend identifier (`"live"`, `"eql"`, …).
    fn name(&self) -> &'static str;

    /// Decode one app packet, keyed on the opcode's stable NAME. Patch-day id
    /// rotations are the caller's opcode-table concern (id→name), not the
    /// backend's — names stay stable across remaps.
    fn decode(&self, opcode: &str, dir: Dir, bytes: &[u8]) -> Decoded;
}

/// Legacy heading (`0..2^bits`, N per circle) → compass degrees `0..359`,
/// matching the daemon's `360 - ((raw * 360) >> bits)`.
pub fn heading_deg(raw: u16, bits: u32) -> u16 {
    let d = 360i32 - ((i32::from(raw) * 360) >> bits);
    (((d % 360) + 360) % 360) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_zero_is_zero() {
        assert_eq!(heading_deg(0, 12), 0);
    }

    #[test]
    fn heading_wraps_into_0_359() {
        for bits in [11u32, 12, 13] {
            let max = 1u16 << bits;
            for raw in [1u16, max / 4, max / 2, max - 1] {
                let d = heading_deg(raw, bits);
                assert!(d < 360, "raw={raw} bits={bits} -> {d}");
            }
        }
    }
}
