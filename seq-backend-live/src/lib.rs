//! Live EQ backend — maps the shared `seq-decode` parsers into the neutral
//! [`seq_events`] vocabulary. Also serves Test today (Test's wire is
//! byte-identical to Live; a `seq-backend-test` sibling forks it when it diverges).
//!
//! Field/heading math mirrors the daemon and the scry NIF exactly so decoded
//! output stays byte-for-byte identical across the migration.

use seq_events::{
    heading_deg, Backend, Decoded, Dir, DoorInfo, Event, Pos, ProfileInfo, SpawnInfo, ZoneInfo,
};

/// The Live/Test backend (shared `seq-decode` parsers).
pub struct LiveBackend;

impl Backend for LiveBackend {
    fn name(&self) -> &'static str {
        "live"
    }

    fn decode(&self, opcode: &str, _dir: Dir, bytes: &[u8]) -> Decoded {
        match opcode {
            "OP_ZoneEntry" => spawn(bytes),
            "OP_MobUpdate" => mob_update(bytes),
            "OP_NpcMoveUpdate" => npc_move_update(bytes),
            "OP_RemoveSpawn" => remove_spawn(bytes),
            "OP_DeleteSpawn" => delete_spawn(bytes),
            "OP_HPUpdate" => hp_update(bytes),
            "OP_Death" => death(bytes),
            "OP_NewZone" => new_zone(bytes),
            "OP_PlayerProfile" => player_profile(bytes),
            "OP_ClientUpdate" => self_pos(bytes),
            "OP_Action2" => action2(bytes),
            "OP_TargetMouse" => target(bytes),
            "OP_Consider" => consider(bytes),
            "OP_CommonMessage" => chat(bytes),
            "OP_GroundSpawn" => ground_item(bytes),
            "OP_SpawnDoor" => doors(bytes),
            "OP_EnterWorld" => Decoded::One(Event::EnterWorld),
            _ => Decoded::Unhandled,
        }
    }
}

fn spawn(bytes: &[u8]) -> Decoded {
    match seq_decode::spawn::parse_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnAdded(SpawnInfo {
            id: s.spawn_id,
            name: s.name,
            last_name: s.last_name,
            race: s.race,
            class_: s.class_,
            deity: s.deity,
            level: s.level,
            npc: s.npc,
            cur_hp: u32::from(s.cur_hp),
            max_hp: None, // Live spawn carries no max HP; arrives via HP opcodes.
            guild_id: s.guild_id,
            pos: None, // Live position arrives via OP_MobUpdate.
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn mob_update(bytes: &[u8]) -> Decoded {
    match seq_decode::mob_update::parse_mob_update(bytes) {
        Ok(s) => Decoded::One(Event::SpawnMoved {
            id: u32::from(s.spawn_id),
            pos: Pos {
                x: s.x,
                y: s.y,
                z: s.z,
                heading_deg: heading_deg(s.heading, 12),
            },
        }),
        Err(_) => Decoded::Malformed,
    }
}

fn npc_move_update(bytes: &[u8]) -> Decoded {
    match seq_decode::npc_move_update::parse_npc_move_update(bytes) {
        Ok(s) => Decoded::One(Event::SpawnMoved {
            id: u32::from(s.spawn_id),
            pos: Pos {
                x: i32::from(s.x),
                y: i32::from(s.y),
                z: i32::from(s.z),
                heading_deg: heading_deg(s.heading as u16, 12),
            },
        }),
        Err(_) => Decoded::Malformed,
    }
}

fn remove_spawn(bytes: &[u8]) -> Decoded {
    match seq_decode::remove_spawn::parse_remove_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnRemoved { id: s.spawn_id }),
        Err(_) => Decoded::Malformed,
    }
}

fn delete_spawn(bytes: &[u8]) -> Decoded {
    match seq_decode::delete_spawn::parse_delete_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnRemoved { id: s.spawn_id }),
        Err(_) => Decoded::Malformed,
    }
}

fn hp_update(bytes: &[u8]) -> Decoded {
    match seq_decode::hp_update::parse_hp_update(bytes) {
        Ok(s) => Decoded::One(Event::SpawnHp {
            id: u32::from(s.spawn_id),
            cur: s.cur_hp,
            max: s.max_hp,
        }),
        Err(_) => Decoded::Malformed,
    }
}

fn self_pos(bytes: &[u8]) -> Decoded {
    match seq_decode::player_self_pos::parse_player_self_pos(bytes) {
        Ok(s) => Decoded::One(Event::SelfPos(Pos {
            x: s.x.round() as i32,
            y: s.y.round() as i32,
            z: s.z.round() as i32,
            heading_deg: heading_deg(s.heading, 12),
        })),
        Err(_) => Decoded::Malformed,
    }
}

// OP_TargetMouse = the player's target selection (0 = cleared).
fn target(bytes: &[u8]) -> Decoded {
    match seq_decode::client_target::parse_client_target(bytes) {
        Ok(t) => Decoded::One(Event::Targeted { spawn_id: t.new_target }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_Consider = the player conned a spawn; the target is the considered spawn.
fn consider(bytes: &[u8]) -> Decoded {
    match seq_decode::consider::parse_consider(bytes) {
        Ok(c) => Decoded::One(Event::Considered { spawn_id: c.target_id }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_CommonMessage = player chat; keep only the player channels (drop system
// noise), matching MessageShell::channelMessage.
fn chat(bytes: &[u8]) -> Decoded {
    match seq_decode::channel_message::parse_channel_message(bytes) {
        Ok(c) if is_player_channel(c.chan_num) => Decoded::One(Event::Chat {
            channel: c.chan_num,
            from: c.sender,
            target: c.target,
            text: c.message,
            chat_color: 0,
            channel_name: String::new(),
        }),
        Ok(_) => Decoded::Ignored,
        Err(_) => Decoded::Malformed,
    }
}

// Guild/Group/Shout/Auction/OOC/Tell/Say/Raid (MessageType enum).
fn is_player_channel(c: u32) -> bool {
    matches!(c, 0 | 2 | 3 | 4 | 5 | 7 | 8 | 15)
}

// OP_Action2 = a damage event; matches the daemon's CombatRouter::action2.
fn action2(bytes: &[u8]) -> Decoded {
    match seq_decode::action2::parse_action2(bytes) {
        Ok(a) => Decoded::One(Event::Combat {
            source: u32::from(a.source),
            target: u32::from(a.target),
            kind: u32::from(a.kind),
            damage: a.damage,
            spell_id: a.spell as u32,
        }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_Death (newCorpseStruct): a death leaves a corpse, not a removal; the caller
// owns the self-death case (SpawnShell::killSpawn).
fn death(bytes: &[u8]) -> Decoded {
    match seq_decode::death::parse_death(bytes) {
        Ok(d) => Decoded::One(Event::SpawnKilled {
            deceased_id: d.spawn_id,
            killer_id: d.killer_id,
        }),
        Err(_) => Decoded::Malformed,
    }
}

fn new_zone(bytes: &[u8]) -> Decoded {
    match seq_decode::new_zone::parse_new_zone(bytes) {
        Ok(z) => Decoded::One(Event::ZoneChanged(ZoneInfo {
            short_name: z.short_name,
            long_name: z.long_name,
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn player_profile(bytes: &[u8]) -> Decoded {
    match seq_decode::player_profile::parse_player_profile(bytes) {
        Ok(p) => Decoded::One(Event::PlayerProfile(ProfileInfo {
            name: p.name,
            last_name: p.last_name,
            class_: p.class_,
            level: p.level,
            race: p.race,
            deity: p.deity,
            cur_hp: p.cur_hp,
            mana: p.mana,
            aa_ids: p.aa_ids,
            aa_values: p.aa_values,
            aa_spent: p.aa_spent,
            platinum: p.platinum,
            gold: p.gold,
            silver: p.silver,
            copper: p.copper,
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn doors(bytes: &[u8]) -> Decoded {
    let doors: Vec<DoorInfo> = bytes
        .chunks(seq_decode::spawn_door::PAYLOAD_LEN)
        .filter(|c| c.len() == seq_decode::spawn_door::PAYLOAD_LEN)
        .filter_map(|c| seq_decode::spawn_door::parse_door(c).ok())
        .map(|d| DoorInfo {
            id: u32::from(d.door_id),
            name: d.name,
            x: d.x.round() as i32,
            y: d.y.round() as i32,
            z: d.z.round() as i32,
        })
        .collect();
    Decoded::One(Event::Doors(doors))
}

// OP_GroundSpawn: one ground object per packet. Coords truncate toward zero to
// match the daemon's float→int position cast.
fn ground_item(bytes: &[u8]) -> Decoded {
    match seq_decode::ground_spawn::parse_ground_spawn(bytes) {
        Ok(g) => Decoded::One(Event::GroundItem {
            drop_id: g.drop_id,
            id_file: g.id_file,
            x: g.x as i32,
            y: g.y as i32,
            z: g.z as i32,
        }),
        Err(_) => Decoded::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_opcode_is_unhandled() {
        let d = LiveBackend.decode("OP_DoesNotExist", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::Unhandled);
    }

    #[test]
    fn truncated_spawn_is_malformed_not_panic() {
        let d = LiveBackend.decode("OP_ZoneEntry", Dir::ServerToClient, &[0u8; 2]);
        assert_eq!(d, Decoded::Malformed);
    }

    #[test]
    fn enter_world_has_no_payload() {
        let d = LiveBackend.decode("OP_EnterWorld", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::One(Event::EnterWorld));
    }

    #[test]
    fn empty_door_batch_is_empty_vec() {
        let d = LiveBackend.decode("OP_SpawnDoor", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::One(Event::Doors(vec![])));
    }
}
