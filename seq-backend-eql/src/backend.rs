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
            "OP_Death" => death(bytes),
            "OP_HPUpdate" => hp_update(bytes),
            "OP_NewZone" => new_zone(bytes),
            "OP_PlayerProfile" => player_profile(bytes),
            "OP_ClientUpdate" => self_pos(bytes),
            "OP_SelfPos" => self_pos_breadcrumb(bytes),
            "OP_Illusion" => illusion(bytes),
            "OP_Action2" => action2(bytes),
            "OP_BeginCast" => begin_cast(bytes),
            "OP_TargetMouse" => target(bytes),
            "OP_Consider" => consider(bytes),
            "OP_CommonMessage" => chat(bytes),
            "OP_ExpUpdate" => exp(bytes),
            "OP_AAExpUpdate" => aa_exp(bytes),
            "OP_ManaChange" => mana_change(bytes),
            "OP_SkillUpdate" => skill_update(bytes),
            "OP_LootTransaction" => loot_transaction(bytes),
            "OP_MoneyUpdate" => money(bytes),
            "OP_SendAATable" => aa_table(bytes),
            "OP_BuffList" | "OP_BuffList2" | "OP_BuffList3" => buff_list(bytes),
            "OP_GroundSpawn" => ground_item(bytes),
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

// OP_Death (newCorpseStruct): the deceased becomes a corpse, not a removal. The
// caller owns the self-death case (it knows the player id) — see SpawnShell::
// killSpawn / EqlDispatch::death.
fn death(bytes: &[u8]) -> Decoded {
    match crate::death::parse_death(bytes) {
        Ok(d) => Decoded::One(Event::SpawnKilled {
            deceased_id: d.spawn_id,
            killer_id: d.killer_id,
        }),
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

// OP_SelfPos = the eql self-pos breadcrumb (a position-history trail, N×17B).
// Wired but INERT: it decodes (so the path is live and validated) yet emits
// nothing — the trail is redundant with the OP_ClientUpdate self-pos and carries
// no heading. Return `One(Event::SelfPos ...)` from the last point here if we
// ever surface the trail.
fn self_pos_breadcrumb(bytes: &[u8]) -> Decoded {
    let _ = crate::self_pos_breadcrumb::parse_self_pos_breadcrumb(bytes);
    Decoded::Ignored
}

// OP_TargetMouse = target select (byte-identical to Live's clientTargetStruct).
fn target(bytes: &[u8]) -> Decoded {
    match crate::client_target::parse_client_target(bytes) {
        Ok(t) => Decoded::One(Event::Targeted { spawn_id: t.new_target }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_Consider = con result; the considered spawn is the target (eql's own 24B).
fn consider(bytes: &[u8]) -> Decoded {
    match crate::consider::parse_consider(bytes) {
        Ok(c) => Decoded::One(Event::Considered { spawn_id: c.target_id }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_CommonMessage = player chat; keep only the player channels (drop system
// noise), matching MessageShell::channelMessage.
fn chat(bytes: &[u8]) -> Decoded {
    match crate::channel_message::parse_channel_message(bytes) {
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

// OP_ExpUpdate = the regular exp bar (0..100000). Shared expUpdateStruct.
fn exp(bytes: &[u8]) -> Decoded {
    match crate::exp_update::parse_exp_update(bytes) {
        Ok(e) => Decoded::One(Event::Exp { exp: e.exp }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_AAExpUpdate = altExpUpdateStruct {u32 altexp@0, u32 aapoints@4, u32 tail};
// the daemon reads it directly (no dedicated parser), so decode the two fields.
fn aa_exp(bytes: &[u8]) -> Decoded {
    if bytes.len() < 8 {
        return Decoded::Malformed;
    }
    let rd = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    Decoded::One(Event::AaExp {
        alt_exp: rd(0),
        aa_points: rd(4),
    })
}

// OP_MoneyUpdate (0x6414) = the authoritative carried purse (un-normalized coins).
fn money(bytes: &[u8]) -> Decoded {
    match crate::money_update::parse_money_update(bytes) {
        Ok(m) => Decoded::One(Event::Money {
            platinum: m.platinum,
            gold: m.gold,
            silver: m.silver,
            copper: m.copper,
        }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_ManaChange: the player's current mana (newMana); no max on the wire.
fn mana_change(bytes: &[u8]) -> Decoded {
    match crate::mana_change::parse_mana_change(bytes) {
        Ok(m) => Decoded::One(Event::ManaUpdate {
            mana: m.new_mana.max(0) as u32,
        }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_SkillUpdate: one skill's new value (skillIncStruct).
fn skill_update(bytes: &[u8]) -> Decoded {
    match crate::skill_update::parse_skill_update(bytes) {
        Ok(s) => Decoded::One(Event::SkillUpdate {
            skill_id: s.skill_id,
            value: s.value.max(0) as u32,
        }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_LootTransaction: only the subcode-7 server confirmation carries the sale
// coin; the other subcodes (3/5/6) ride the same id but surface nothing.
fn loot_transaction(bytes: &[u8]) -> Decoded {
    use crate::loot_transaction::LootTransactionError::NotConfirm;
    match crate::loot_transaction::parse_loot_transaction(bytes) {
        Ok(t) => Decoded::One(Event::LootTransaction {
            corpse_id: t.corpse_id,
            item_id: t.item_id,
            quantity: t.quantity,
            coin_copper: t.coin_copper,
        }),
        Err(NotConfirm(_)) => Decoded::Ignored,
        Err(_) => Decoded::Malformed,
    }
}

// eql OP_SendAATable = one AA definition (descID -> titleSID) per packet.
fn aa_table(bytes: &[u8]) -> Decoded {
    match crate::parse_aa_table_entry(bytes) {
        Ok(a) => Decoded::One(Event::AaTable {
            desc_id: a.desc_id,
            title_sid: a.title_sid,
        }),
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

// OP_BeginCast: a spawn started casting. The daemon surfaces this (a transient
// cast indicator), NOT OP_CastSpell — cast-start buff insertion was noise, buffs
// ride OP_BuffList.
fn begin_cast(bytes: &[u8]) -> Decoded {
    match crate::parse_begin_cast(bytes) {
        Ok(c) => Decoded::One(Event::SpawnCast {
            caster_id: c.caster_id,
            spell_id: c.spell_id,
            cast_time_ms: c.cast_time_ms,
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
            aa_ids: p.aa_ids,
            aa_values: p.aa_values,
            aa_spent: p.aa_spent,
            skills: p.skills,
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

// OP_Illusion: a spawn changed race/model (id + new race/gender).
fn illusion(bytes: &[u8]) -> Decoded {
    match crate::illusion::parse_illusion(bytes) {
        Ok(i) => Decoded::One(Event::SpawnIllusion {
            spawn_id: i.spawn_id,
            race: i.race,
            gender: i.gender,
        }),
        Err(_) => Decoded::Malformed,
    }
}

// OP_GroundSpawn: one ground object per packet (variable-length actorDef name).
// Coords truncate toward zero to match the daemon's float→int position cast.
fn ground_item(bytes: &[u8]) -> Decoded {
    match crate::ground_spawn::parse_ground_spawn(bytes) {
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
    fn self_pos_is_wired_but_inert() {
        // Recognized (not Unhandled) so it leaves the gap report, but emits
        // nothing — the wired-but-inert breadcrumb path.
        let d = EqlBackend.decode("OP_SelfPos", Dir::ServerToClient, &[0u8; 18]);
        assert_eq!(d, Decoded::Ignored);
    }

    #[test]
    fn empty_door_batch_is_empty_vec() {
        let d = EqlBackend.decode("OP_SpawnDoor", Dir::ServerToClient, &[]);
        assert_eq!(d, Decoded::One(Event::Doors(vec![])));
    }
}
