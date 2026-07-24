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
    /// Guild ids are only unique within a guild server, so the pair is the key
    /// into the guild map built from [`Event::GuildsInZone`]. 0 on backends that
    /// don't send it.
    pub guild_server_id: u32,
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
    /// Total AA points spent (the profile's `aa_spent`).
    pub aa_spent: u32,
    /// Learned-skill values, indexed by skill id (eql fills this; Live surfaces
    /// skills by another path, so it's empty there). `0xFFFFFFFF` = the skill is
    /// unavailable to this class; the consumer filters those (and 0) out.
    pub skills: Vec<u32>,
    /// On-hand carried coins (the base the OP_MoneyUpdate purse resyncs).
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub copper: u32,
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

/// One lootable item on a corpse (OP_LootDrops). `item_id` is parsed from the
/// item-link header; `icon` is the dragitem-atlas id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootItemInfo {
    pub name: String,
    pub icon: u32,
    pub item_id: u32,
}

/// One guild present in the zone, from the guild-in-zone opcodes. A spawn's
/// guild is on the wire only as the (guild_id, server_id) pair — these records
/// are the sole source of the NAME, so a consumer keys its guild map on the
/// pair. `server_id` is part of the key, not decoration: ids are only unique
/// within a guild server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInZone {
    pub guild_id: u32,
    pub server_id: u32,
    pub name: String,
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
    /// A spawn died (OP_Death). Unlike SpawnRemoved the body stays as a corpse;
    /// the consumer keeps it in its spawn map. `killer_id` 0 = no killer / self.
    /// The consumer owns the self-death special case (it knows the player id).
    SpawnKilled { deceased_id: u32, killer_id: u32 },
    /// A spawn's health changed (OP_HPUpdate). `max` is real HP for the self and
    /// a percentage base (100) for other spawns, mirroring the wire.
    SpawnHp { id: u32, cur: i32, max: i32 },
    /// One packet of the multiplexed stat-sync channel (eql OP_HPUpdate), which
    /// carries spawn HP plus the local player's mana/endurance together. Kept as
    /// ONE event per packet on purpose: splitting it into per-stat events makes a
    /// consumer emit several near-identical player snapshots for a single packet.
    ///
    /// The consumer owns the self/other split — it knows the player id and this
    /// crate is stateless. Routing rules, mirroring the daemon:
    ///   * HP is meaningful only when `has_hp && hp_max > 0`. For the self it is
    ///     real cur/max; for other spawns the narrow form is a percentage.
    ///   * mana/endurance are the local player's only, and only when `wide` —
    ///     the narrow form is a u8 percent with a synthesized max of 100, which
    ///     is useless as a max.
    /// eql has no standalone endurance opcode, so this is its sole endurance feed.
    StatSync {
        spawn_id: u32,
        wide: bool,
        has_hp: bool,
        hp_cur: i32,
        hp_max: i32,
        has_mana: bool,
        mana_cur: i32,
        mana_max: i32,
        has_end: bool,
        end_cur: i32,
        end_max: i32,
    },
    /// The local player moved (OP_ClientUpdate self position).
    SelfPos(Pos),
    /// A spawn changed race/model via an illusion (OP_Illusion). The consumer
    /// merges the new race into the tracked spawn and re-renders it; the daemon
    /// ignores it for an unknown spawn (the spawn arrives already illusioned).
    SpawnIllusion { spawn_id: u32, race: u32, gender: u8 },
    /// Guilds present in the current zone, resolving guild ids to names
    /// (OP_GuildsInZoneList on zone-in, OP_NewGuildInZone as guilded players
    /// arrive — the latter is just a one-element list, so both map here).
    ///
    /// The consumer accumulates these into a guild map and back-fills spawns:
    /// a spawn can arrive before its guild is named, so tagging only on receipt
    /// would permanently miss those.
    GuildsInZone { guilds: Vec<GuildInZone> },
    /// Zone changed (OP_NewZone).
    ZoneChanged(ZoneInfo),
    /// The local player's profile (OP_PlayerProfile).
    PlayerProfile(ProfileInfo),
    /// A player switched multiclass loadouts (eql OP_LoadoutSwap), changing
    /// their class + level. eql sends no OP_PlayerProfile on a swap, so this is
    /// the sole source of the new identity. The consumer owns the self/other
    /// split (it knows the player id): the self → refresh identity + its player
    /// snapshot; another spawn → update that spawn's class/level in place.
    /// `class` is the single resolved class, not the multiclass mask.
    LoadoutSwap {
        spawn_id: u32,
        level: u32,
        class: u32,
        race: u32,
    },
    /// A batch of doors / static objects (OP_SpawnDoor).
    Doors(Vec<DoorInfo>),
    /// A ground item / static placeable (OP_GroundSpawn). The daemon renders it
    /// as a DROP-type spawn (it keeps a separate drop map; a single-map consumer
    /// offsets the id like doors). `id_file` is the actorDef model string —
    /// resolving it to a real item name needs the item DB (deferred).
    GroundItem {
        drop_id: u32,
        id_file: String,
        x: i32,
        y: i32,
        z: i32,
    },
    /// A damage event (OP_Action2). Ids only; the consumer resolves names from
    /// its spawn map. `kind` is the wire damage type; `spell_id` 0 = melee.
    Combat {
        source: u32,
        target: u32,
        kind: u32,
        damage: i32,
        spell_id: u32,
    },
    /// A spawn started casting a spell (OP_BeginCast). Ids only; the consumer
    /// resolves the caster name from its spawn map and the spell name from its
    /// spell DB. `cast_time_ms` is the wire cast time (0 = instant).
    SpawnCast {
        caster_id: u32,
        spell_id: u32,
        cast_time_ms: u32,
    },
    /// The player selected a target (OP_TargetMouse). `spawn_id` 0 = cleared.
    Targeted { spawn_id: u32 },
    /// The player considered a spawn (OP_Consider) — `spawn_id` is the target.
    Considered { spawn_id: u32 },
    /// One AA definition from the OP_SendAATable burst: maps a purchased AA's
    /// `desc_id` to a `title_sid` (a dbstring type-1 id → the AA's display name).
    AaTable { desc_id: u32, title_sid: u32 },
    /// The regular experience bar (OP_ExpUpdate), 0..100000 within a level. On
    /// eql there is no discrete level packet — a wrap (decrease) is a ding.
    Exp { exp: u32 },
    /// AA experience (OP_AAExpUpdate): `alt_exp` 0..100000 toward the next point,
    /// `aa_points` = unspent points.
    AaExp { alt_exp: u32, aa_points: u32 },
    /// The player's current mana (OP_ManaChange). eql sends no max on the wire —
    /// the consumer tracks the observed high-water mark, like the daemon.
    ManaUpdate { mana: u32 },
    /// A single skill's new value (OP_SkillUpdate) — the consumer updates that
    /// skill id in the player's skill map.
    SkillUpdate { skill_id: u32, value: u32 },
    /// A corpse-loot confirmation (OP_LootTransaction subcode 7). `coin_copper`
    /// is the auto-sale proceeds (0 when the loot produced none) — the consumer
    /// adds it to the running money total, like the daemon's adjustMoney.
    LootTransaction {
        corpse_id: u32,
        item_id: u32,
        quantity: u32,
        coin_copper: u32,
    },
    /// A corpse's loot window (OP_LootDrops) — the lootable items on a corpse.
    LootDrops {
        corpse_id: u32,
        corpse_name: String,
        items: Vec<LootItemInfo>,
    },
    /// The carried purse (OP_MoneyUpdate, 0x6414). Denominations are NOT
    /// normalized on the wire — the consumer sums to total copper.
    Money {
        platinum: u32,
        gold: u32,
        silver: u32,
        copper: u32,
    },
    /// A string-id server message (OP_SimpleMessage): `format_id` resolves to
    /// text via the eqstr DB (no args); `color` is the wire ChatColor.
    SimpleMessage { format_id: u32, color: u32 },
    /// A formatted server message (OP_FormattedMessage): `format_id` + `args`
    /// interpolate through the eqstr template; `color` is the wire ChatColor.
    FormattedMessage {
        format_id: u32,
        color: u32,
        args: Vec<String>,
    },
    /// A special server message (OP_SpecialMesg): carries `message` text
    /// directly + a `source` sender and a `target` spawn id (0 = none).
    SpecialMessage {
        color: u32,
        target: u32,
        source: String,
        message: String,
    },
    /// Auto-loot / sell narration (OP_LootMessage), e.g. "You looted a …" —
    /// `text` is already link-cleaned; the consumer shows it as general chat.
    LootMessage { color: u32, text: String },
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
    /// A member joined the group (OP_GroupFollow): `name` (the invitee) is added
    /// to the roster. `level` is the member's wire level (0 if absent).
    GroupFollow { name: String, level: u32 },
    /// A group departure (OP_GroupDisband / OP_GroupDisband2): `membername`
    /// leaves; `membername == yourname` means the whole group disbanded.
    GroupDisband { yourname: String, membername: String },
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
