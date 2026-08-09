//! EQ Legends loadout swap (OP_LoadoutSwap).
//!
//! Sent when a player switches loadouts (the Legends multiclass class/level
//! change). No OP_PlayerProfile follows, so this is the only source for the new
//! identity. Two variants share one header + record layout:
//!   * self (~118 KB): the acting client's own refresh, with a serialized
//!     inventory tail (unparsed here);
//!   * broadcast (~490 B): the server tells nearby clients about ANY in-range
//!     player's swap — same header + record, no inventory tail.
//!
//! Header: `u32 spawnId | u8 | u16 innerLen | <record> | <inventory tail>`.
//! `innerLen` covers the header (7 bytes) + the embedded record, which is
//! byte-identical to the OP_ZoneEntry spawn record — so we reuse this
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
    /// Length of the serialized inventory tail — `data[innerLen..]`.
    ///
    /// **0 on a broadcast**, which is how the two variants are told apart
    /// without guessing from the payload size: verified 0 across all nine
    /// broadcast fires in `eqlegends-inventory-paired`. A self swap should show
    /// a large value (the notes put the self variant near 118 KB against a
    /// ~490 B broadcast).
    ///
    /// The tail's CONTENTS are not parsed: no capture containing a self variant
    /// exists yet, and the leading hypothesis — that it is the same
    /// serial/name/lore/field records `item_packet` decodes — is untested.
    /// Guessing here would be the one mistake this crate keeps avoiding, so the
    /// length is surfaced and the bytes are left to `tail_of`.
    pub tail_len: usize,
}

/// The serialized inventory tail, or an empty slice when there is none.
///
/// Borrowed from the caller's buffer rather than copied — a self variant's tail
/// is ~117 KB and nothing has yet earned a clone of it.
pub fn tail_of(data: &[u8]) -> &[u8] {
    if data.len() < 7 {
        return &[];
    }
    let inner_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    if inner_len < 8 || inner_len > data.len() {
        return &[];
    }
    &data[inner_len..]
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
        tail_len: data.len() - inner_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_payload_yields_no_tail_rather_than_panicking() {
        assert!(tail_of(&[]).is_empty());
        assert!(tail_of(&[0u8; 6]).is_empty());
    }

    /// innerLen covers the header + record, so a payload whose length EQUALS
    /// innerLen is a broadcast and has no tail. Every captured broadcast reads
    /// this way.
    #[test]
    fn a_broadcast_has_an_empty_tail() {
        let mut p = vec![0u8; 64];
        p[5..7].copy_from_slice(&64u16.to_le_bytes());
        assert!(tail_of(&p).is_empty());
    }

    #[test]
    fn a_self_variant_exposes_the_bytes_past_inner_len() {
        let mut p = vec![0u8; 100];
        p[5..7].copy_from_slice(&64u16.to_le_bytes());
        p[64] = 0xAB;
        let t = tail_of(&p);
        assert_eq!(t.len(), 36);
        assert_eq!(t[0], 0xAB);
    }

    /// A bogus innerLen must not slice past the buffer.
    #[test]
    fn an_out_of_range_inner_len_is_refused() {
        let mut p = vec![0u8; 32];
        p[5..7].copy_from_slice(&9999u16.to_le_bytes());
        assert!(tail_of(&p).is_empty());
    }
}
