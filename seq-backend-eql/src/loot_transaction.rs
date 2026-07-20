//! `OP_LootTransaction` (0x6c3c's neighbour, id 0x7d1c): a subcode-multiplexed
//! corpse-loot channel. The client requests a loot action and the server
//! confirms it.
//!
//! Observed subcodes (u16 @0), from one 26-minute capture:
//!
//! ```text
//!   3  C->S  2B    /  S->C 6B    request + ack, purpose unknown
//!   6  C->S  25B                 the client's loot request
//!   5  S->C  16B                 server response, layout unmapped
//!   7  S->C  36B                 server confirmation — decoded here
//! ```
//!
//! The 36B confirmation echoes the client's 25B request body and appends the
//! sale proceeds:
//!
//! ```text
//!   @0  u16 subcode = 7      @16 u32 quantity (1 or 2 observed)
//!   @2  u16 = 1              @20 u32 request sequence (monotonic)
//!   @4  u32 item id          @24 u16 (3 or 0)
//!   @8  u32 slot             @26 u32 SALE PROCEEDS in copper
//!   @12 u32 corpse spawn id  @30 u8[6] zero
//! ```
//!
//! `coin_copper` is wire-verified: across 24 confirmations the 18 non-zero
//! values matched, exactly and in full, the 18 amounts the server separately
//! stated as English text ("...and sold it for 3 gold, 5 silver and 7 copper"
//! = 357). Reading it here replaces parsing that wording.
//!
//! `corpse_id` is likewise verified: all 18 distinct values are spawns that both
//! died and appear as a `loot_drops` corpse id, with no exceptions.
//!
//! NOTE this id is named OP_Death* in legacy showeq, which reads the 36B record
//! as a combat/death record ({sourceId, skillOrDamage, targetId, combatSeq}).
//! That does not survive the capture: the record is client-initiated, every
//! target was looted, and the trailing field is coin. Its `combatSeq` is really
//! a loot request counter, and its "always 1" field @16 takes the value 2.

use thiserror::Error;

/// Server confirmation subcode — the only one decoded.
pub const SUBCODE_CONFIRM: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LootTransaction {
    pub item_id: u32,
    pub slot: u32,
    pub corpse_id: u32,
    pub quantity: u32,
    pub sequence: u32,
    /// Sale proceeds in copper; 0 when the loot produced no coin.
    pub coin_copper: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LootTransactionError {
    #[error("expected 30+ bytes, got {0}")]
    Short(usize),
    #[error("not a server confirmation (subcode {0})")]
    NotConfirm(u16),
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse_loot_transaction(bytes: &[u8]) -> Result<LootTransaction, LootTransactionError> {
    if bytes.len() < 30 {
        return Err(LootTransactionError::Short(bytes.len()));
    }
    let subcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    if subcode != SUBCODE_CONFIRM {
        return Err(LootTransactionError::NotConfirm(subcode));
    }
    Ok(LootTransaction {
        item_id: rd_u32(bytes, 4),
        slot: rd_u32(bytes, 8),
        corpse_id: rd_u32(bytes, 12),
        quantity: rd_u32(bytes, 16),
        sequence: rd_u32(bytes, 20),
        coin_copper: rd_u32(bytes, 26),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured verbatim. The same loot produced the chat line
    // "...and sold it for 3 gold, 5 silver and 7 copper" = 357 copper.
    const CONFIRM_357: [u8; 36] = [
        0x07, 0x00, 0x01, 0x00, // subcode 7, u16=1
        0xec, 0x03, 0x00, 0x00, // item 1004
        0x04, 0x00, 0x00, 0x00, // slot 4
        0xcb, 0x2e, 0x00, 0x00, // corpse 11979
        0x01, 0x00, 0x00, 0x00, // quantity 1
        0x0f, 0x00, 0x00, 0x00, // sequence 15
        0x03, 0x00, // u16
        0x65, 0x01, 0x00, 0x00, // 357 copper
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn parses_a_confirmation() {
        let t = parse_loot_transaction(&CONFIRM_357).unwrap();
        assert_eq!(t.item_id, 1004);
        assert_eq!(t.slot, 4);
        assert_eq!(t.corpse_id, 11979);
        assert_eq!(t.quantity, 1);
        assert_eq!(t.sequence, 15);
        assert_eq!(t.coin_copper, 357);
    }

    #[test]
    fn accepts_a_coinless_loot() {
        // Same shape with no proceeds — 6 of 24 confirmations carried 0.
        let mut b = CONFIRM_357;
        b[26..30].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_loot_transaction(&b).unwrap().coin_copper, 0);
    }

    #[test]
    fn rejects_the_other_subcodes() {
        // 3 (2B/6B), 5 (16B) and 6 (25B, the client request) all ride this id.
        for sc in [3u16, 5, 6] {
            let mut b = CONFIRM_357;
            b[0..2].copy_from_slice(&sc.to_le_bytes());
            assert_eq!(
                parse_loot_transaction(&b),
                Err(LootTransactionError::NotConfirm(sc))
            );
        }
    }

    #[test]
    fn rejects_a_short_payload() {
        assert_eq!(
            parse_loot_transaction(&[7, 0, 1, 0]),
            Err(LootTransactionError::Short(4))
        );
    }
}
