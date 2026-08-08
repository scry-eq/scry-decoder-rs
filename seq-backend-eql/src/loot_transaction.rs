//! `OP_LootTransaction`: a subcode-multiplexed corpse-loot channel. The client requests a loot action
//! and the server confirms it.
//!
//! Observed subcodes (u16 @0):
//!
//! ```text
//!   3  C->S  2B    /  S->C 6B    request + ack, purpose unknown
//!   6  C->S  25B                 the client's loot request
//!   5  S->C  16B                 coin on the corpse — decoded here
//!   7  S->C  36B                 item confirmation — decoded here
//! ```
//!
//! The 36B confirmation echoes the client's 25B request body and appends the
//! sale proceeds:
//!
//! ```text
//!   @0  u16 subcode = 7      @16 u32 quantity (1 or 2 observed)
//!   @2  u16 = 1              @20 u32 request sequence (monotonic)
//!   @4  u32 item id          @24 u16 (3 when sold, else 4/5/6)
//!   @8  u32 slot             @26 u32 SALE PROCEEDS in copper
//!   @12 u32 corpse spawn id  @30 u8[6] zero
//! ```
//!
//! The 16B subcode-5 is the corpse's coin pile, announced once per
//! `OP_LootDrops`. EQL auto-takes it, so it is always acquired:
//!
//! ```text
//!   @0  u16 subcode = 5      @7  u32 zero
//!   @2  u8  = 1              @11 u8  = 1
//!   @3  u32 COIN in copper   @12 u32 zero
//! ```
//!
//! Both coin fields are wire-verified against server text and reconciled
//! against the purse delta; evidence in OPCODES_LEGENDS.md.
//!
//! Legacy showeq reads the 36B record as a combat/death record and discards it;
//! that reading does not survive the capture (client-initiated, every target
//! looted, trailing field is coin), and upstream deleted the mapping in 08/26.

use thiserror::Error;

/// Coin-on-the-corpse subcode (16B, announced with the loot window).
pub const SUBCODE_CORPSE_COIN: u16 = 5;
/// Item confirmation subcode (36B, carries the auto-sale proceeds).
pub const SUBCODE_CONFIRM: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LootTransaction {
    pub item_id: u32,
    pub slot: u32,
    pub corpse_id: u32,
    pub quantity: u32,
    pub sequence: u32,
    /// Sale proceeds (subcode 7, 0 if unsold) or the corpse pile (subcode 5).
    pub coin_copper: u32,
    /// Subcode-5 corpse pile; the item fields are 0.
    pub from_corpse: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LootTransactionError {
    #[error("subcode {subcode} needs {want}+ bytes, got {got}")]
    Short {
        subcode: u16,
        want: usize,
        got: usize,
    },
    #[error("truncated before the subcode ({0} bytes)")]
    NoSubcode(usize),
    #[error("subcode {0} carries no decoded payload")]
    Unhandled(u16),
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse_loot_transaction(bytes: &[u8]) -> Result<LootTransaction, LootTransactionError> {
    if bytes.len() < 2 {
        return Err(LootTransactionError::NoSubcode(bytes.len()));
    }
    let subcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    match subcode {
        SUBCODE_CORPSE_COIN => {
            // Coin is a u32 at @3 — genuinely unaligned, not a mis-read offset.
            if bytes.len() < 7 {
                return Err(LootTransactionError::Short {
                    subcode,
                    want: 7,
                    got: bytes.len(),
                });
            }
            Ok(LootTransaction {
                coin_copper: rd_u32(bytes, 3),
                from_corpse: true,
                ..LootTransaction::default()
            })
        }
        SUBCODE_CONFIRM => {
            if bytes.len() < 30 {
                return Err(LootTransactionError::Short {
                    subcode,
                    want: 30,
                    got: bytes.len(),
                });
            }
            Ok(LootTransaction {
                item_id: rd_u32(bytes, 4),
                slot: rd_u32(bytes, 8),
                corpse_id: rd_u32(bytes, 12),
                quantity: rd_u32(bytes, 16),
                sequence: rd_u32(bytes, 20),
                coin_copper: rd_u32(bytes, 26),
                from_corpse: false,
            })
        }
        other => Err(LootTransactionError::Unhandled(other)),
    }
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
    fn rejects_the_undecoded_subcodes() {
        // 3 (2B/6B request+ack) and 6 (25B, the client request) ride this id
        // but surface nothing.
        for sc in [3u16, 6] {
            let mut b = CONFIRM_357;
            b[0..2].copy_from_slice(&sc.to_le_bytes());
            assert_eq!(
                parse_loot_transaction(&b),
                Err(LootTransactionError::Unhandled(sc))
            );
        }
    }

    #[test]
    fn rejects_a_short_payload() {
        assert_eq!(
            parse_loot_transaction(&[7, 0, 1, 0]),
            Err(LootTransactionError::Short {
                subcode: 7,
                want: 30,
                got: 4
            })
        );
    }

    // Captured verbatim (eqlegends-loot2, 12:06:28). The same loot window
    // produced "You receive 2 platinum, 8 gold, 8 silver and 1 copper from the
    // corpse." = 2881 copper.
    const CORPSE_COIN_2881: [u8; 16] = [
        0x05, 0x00, // subcode 5
        0x01, // u8 = 1
        0x41, 0x0b, 0x00, 0x00, // 2881 copper @3 (unaligned)
        0x00, 0x00, 0x00, 0x00, // zero
        0x01, // u8 = 1
        0x00, 0x00, 0x00, 0x00, // zero
    ];

    #[test]
    fn parses_the_corpse_coin() {
        let t = parse_loot_transaction(&CORPSE_COIN_2881).unwrap();
        assert_eq!(t.coin_copper, 2881);
        assert!(t.from_corpse);
        // The record names no item — only the coin is meaningful.
        assert_eq!(t.item_id, 0);
        assert_eq!(t.corpse_id, 0);
        assert_eq!(t.quantity, 0);
    }

    #[test]
    fn parses_a_coinless_corpse() {
        let mut b = CORPSE_COIN_2881;
        b[3..7].copy_from_slice(&0u32.to_le_bytes());
        let t = parse_loot_transaction(&b).unwrap();
        assert_eq!(t.coin_copper, 0);
        assert!(t.from_corpse);
    }

    #[test]
    fn an_item_confirmation_is_not_from_a_corpse_pile() {
        assert!(!parse_loot_transaction(&CONFIRM_357).unwrap().from_corpse);
    }

    #[test]
    fn rejects_a_short_corpse_coin() {
        assert_eq!(
            parse_loot_transaction(&[5, 0, 1, 0x41]),
            Err(LootTransactionError::Short {
                subcode: 5,
                want: 7,
                got: 4
            })
        );
    }

    #[test]
    fn rejects_a_payload_with_no_subcode() {
        assert_eq!(
            parse_loot_transaction(&[5]),
            Err(LootTransactionError::NoSubcode(1))
        );
    }
}
