//! eql implementation of the neutral [`seq_events::Backend`] contract.
//!
//! Maps this crate's self-contained EQ Legends parsers into the neutral event
//! vocabulary. Depends only on `seq-events` (pure vocabulary, no Live decode
//! code), so it does not breach eql's isolation — no Live wire parser reaches
//! eql through it.
//!
//! Field/heading math mirrors the scry NIF exactly (eql spawn heading is 11-bit
//! h2048, self-pos is 13-bit, mob/npc updates are 12-bit like Live) so decoded
//! output stays byte-for-byte identical across the migration.

use seq_events::{
    heading_deg, Backend, BuffEntry, Decoded, Dir, DoorInfo, Event, Pos, ProfileInfo, SpawnInfo,
    ZoneInfo,
};

/// The EverQuest Legends backend (this crate's own parsers).
pub struct EqlBackend;

impl Backend for EqlBackend {
    fn name(&self) -> &'static str {
        "eql"
    }

    fn decode(&self, opcode: &str, _dir: Dir, bytes: &[u8]) -> Decoded {
        match opcode {
            "OP_ZoneEntry" => spawn(bytes),
            "OP_MobUpdate" => mob_update(bytes),
            "OP_NpcMoveUpdate" => npc_move_update(bytes),
            "OP_RemoveSpawn" => remove_spawn(bytes),
            "OP_DeleteSpawn" => delete_spawn(bytes),
            "OP_HPUpdate" => hp_update(bytes),
            "OP_NewZone" => new_zone(bytes),
            "OP_PlayerProfile" => player_profile(bytes),
            "OP_ClientUpdate" => self_pos(bytes),
            "OP_Action2" => action2(bytes),
            "OP_BuffList" | "OP_BuffList2" | "OP_BuffList3" => buff_list(bytes),
            "OP_SpawnDoor" => doors(bytes),
            "OP_EnterWorld" => Decoded::One(Event::EnterWorld),
            _ => Decoded::Unhandled,
        }
    }
}

fn spawn(bytes: &[u8]) -> Decoded {
    match crate::parse_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnAdded(SpawnInfo {
            id: u32::from(s.id),
            name: s.name,
            last_name: s.last_name,
            race: s.race,
            class_: s.class_,
            deity: s.deity,
            level: s.level,
            npc: s.npc,
            cur_hp: u32::from(s.cur_hp),
            max_hp: Some(u32::from(s.max_hp)),
            guild_id: s.guild_id,
            // eql spawn carries position inline; heading is h2048 (11-bit).
            pos: Some(Pos {
                x: i32::from(s.x),
                y: i32::from(s.y),
                z: i32::from(s.z),
                heading_deg: heading_deg(s.heading, 11),
            }),
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn mob_update(bytes: &[u8]) -> Decoded {
    match crate::mob_update::parse_mob_update(bytes) {
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
    match crate::npc_move_update::parse_npc_move_update(bytes) {
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
    match crate::remove_spawn::parse_remove_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnRemoved { id: s.spawn_id }),
        Err(_) => Decoded::Malformed,
    }
}

fn delete_spawn(bytes: &[u8]) -> Decoded {
    match crate::delete_spawn::parse_delete_spawn(bytes) {
        Ok(s) => Decoded::One(Event::SpawnRemoved { id: s.spawn_id }),
        Err(_) => Decoded::Malformed,
    }
}

// eql OP_HPUpdate is the multiplexed stat-sync channel: spawn HP (real for the
// self, percent for others) plus player mana/endurance. We surface spawn HP as
// SpawnHp — the caller's World treats the self as a spawn, so it needs no
// player/spawn split. Player mana/endurance are deferred (Ignored for now).
fn hp_update(bytes: &[u8]) -> Decoded {
    match crate::parse_stat_sync(bytes) {
        Ok(s) if s.has_hp && s.hp_max > 0 => Decoded::One(Event::SpawnHp {
            id: s.spawn_id,
            cur: s.hp_cur as i32,
            max: s.hp_max as i32,
        }),
        Ok(_) => Decoded::Ignored,
        Err(_) => Decoded::Malformed,
    }
}

fn self_pos(bytes: &[u8]) -> Decoded {
    match crate::player_self_pos::parse_player_self_pos(bytes) {
        // eql self heading is 13-bit (8192 per circle).
        Ok(s) => Decoded::One(Event::SelfPos(Pos {
            x: s.x.round() as i32,
            y: s.y.round() as i32,
            z: s.z.round() as i32,
            heading_deg: heading_deg(s.heading, 13),
        })),
        Err(_) => Decoded::Malformed,
    }
}

// eql OP_BuffList = the authoritative per-spawn active-buff snapshot.
fn buff_list(bytes: &[u8]) -> Decoded {
    match crate::parse_buff_list(bytes) {
        Ok(bl) => Decoded::One(Event::BuffList {
            owner: bl.spawn_id,
            entries: bl
                .entries
                .into_iter()
                .map(|e| BuffEntry {
                    spell_id: e.spell_id,
                    remaining_ticks: e.remaining_ticks,
                    slot: e.slot,
                })
                .collect(),
        }),
        Err(_) => Decoded::Malformed,
    }
}

// eql reuses Live's action2Struct byte-identically (OP_Action2 = damage).
fn action2(bytes: &[u8]) -> Decoded {
    match crate::action2::parse_action2(bytes) {
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

fn new_zone(bytes: &[u8]) -> Decoded {
    match crate::parse_new_zone(bytes) {
        Ok(z) => Decoded::One(Event::ZoneChanged(ZoneInfo {
            short_name: z.short_name,
            long_name: z.long_name,
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn player_profile(bytes: &[u8]) -> Decoded {
    match crate::parse_player_profile(bytes) {
        Ok(p) => Decoded::One(Event::PlayerProfile(ProfileInfo {
            name: p.name,
            last_name: p.last_name,
            class_: p.class_,
            level: p.level,
            race: p.race,
            deity: p.deity,
            cur_hp: p.cur_hp,
            mana: p.mana,
        })),
        Err(_) => Decoded::Malformed,
    }
}

fn doors(bytes: &[u8]) -> Decoded {
    let doors: Vec<DoorInfo> = bytes
        .chunks(crate::spawn_door::PAYLOAD_LEN)
        .filter(|c| c.len() == crate::spawn_door::PAYLOAD_LEN)
        .filter_map(|c| crate::spawn_door::parse_door(c).ok())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_opcode_is_unhandled() {
        let d = EqlBackend.decode("OP_DoesNotExist", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::Unhandled);
    }

    #[test]
    fn truncated_spawn_is_malformed_not_panic() {
        let d = EqlBackend.decode("OP_ZoneEntry", Dir::ServerToClient, &[0u8; 2]);
        assert_eq!(d, Decoded::Malformed);
    }

    #[test]
    fn enter_world_has_no_payload() {
        let d = EqlBackend.decode("OP_EnterWorld", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::One(Event::EnterWorld));
    }

    #[test]
    fn empty_door_batch_is_empty_vec() {
        let d = EqlBackend.decode("OP_SpawnDoor", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::One(Event::Doors(vec![])));
    }
}
