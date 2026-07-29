//! EQ Legends loadout swap (OP_LoadoutSwap, 0x7477).
//!
//! Sent when a player switches loadouts (the Legends multiclass class/level
//! change). No OP_PlayerProfile follows, so this is the only source for the new
//! identity. Two variants share one header + record layout:
//!   * self  (~118 KB): the acting client's own refresh, with a serialized
//!            inventory tail (unparsed here);
//!   * broadcast (~490 B): the server tells nearby clients about ANY in-range
//!            player's swap — same header + record, no inventory tail.
//!
//! Header: `u32 spawnId | u8 | u16 innerLen | <record> | <inventory tail>`.
//! `innerLen` covers the header (7 bytes) + the embedded record, which is
//! byte-identical to the OP_ZoneEntry (0x4606) spawn record — so we reuse this
//! crate's `parse_spawn` (eql's ZoneEntry parser; same canonical name as Live's
//! `seq-decode` twin, different impl) on `data[7..innerLen]` and surface the
//! fields that change on a swap (level + class), plus the header spawnId for
//! matching the tracked spawn.

use crate::{parse_spawn, DecodeError};

pub struct LoadoutSwap {
    /// The whole embedded ZoneEntry-format record. Legends does
    /// delete-then-readd on a swap — a paired OP_DeleteSpawn removes the id
    /// moments before — so this record IS the re-add, and a consumer that only
    /// applies the changed fields silently drops the spawn. Surfaced so the
    /// consumer can re-create it.
    pub record: crate::ZoneSpawn,
    /// Header spawnId — the id of the player who swapped (matches the tracked
    /// spawn, or the local player's own id for a self swap).
    pub spawn_id: u32,
    pub level: u8,
    /// Resolved current class from the embedded record (single value, as the
    /// ZoneEntry record already carries for PCs — not the multiclass mask).
    pub class_: u32,
    pub race: u32,
}

#[derive(Debug)]
pub enum LoadoutSwapError {
    /// Too short to hold the fixed header.
    Short,
    /// innerLen is below the header size or runs past the packet.
    BadInnerLen,
    /// The embedded ZoneEntry-format record failed to parse.
    Record(DecodeError),
}

pub fn parse_loadout_swap(data: &[u8]) -> Result<LoadoutSwap, LoadoutSwapError> {
    if data.len() < 7 {
        return Err(LoadoutSwapError::Short);
    }
    let spawn_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let inner_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    if inner_len < 8 || inner_len > data.len() {
        return Err(LoadoutSwapError::BadInnerLen);
    }
    let record = parse_spawn(&data[7..inner_len]).map_err(LoadoutSwapError::Record)?;
    Ok(LoadoutSwap {
        spawn_id,
        level: record.level,
        record: record.clone(),
        class_: record.class_,
        race: record.race,
    })
}
