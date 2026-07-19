//! `OP_MoneyUpdate` (0x6414): the authoritative carried purse, 20B
//! `{u32 platinum@0, u32 gold@4, u32 silver@8, u32 copper@12, u32=0@16}`.
//!
//! Denominations are NOT normalized on the wire — 101 silver / 281 copper have
//! been observed — so callers must sum rather than assume each is < 10.
//!
//! Verified against OP_PlayerProfile on two characters: every value this opcode
//! reports matches the profile's own coin block exactly. It broadcasts at
//! zone-in (and occasionally between), but NOT per coin-earning event, so it is
//! a resync rather than a live feed.
//!
//! 0x6414 is the post-2026-07-14 id. The pre-patch id (0x4d77) now carries an
//! unrelated 12B 60s heartbeat — see OP_Unknown4 in the opcode table. Layout
//! credit: Xerxes (legacy showeq moneyUpdateEQL).

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoneyUpdate {
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub copper: u32,
}

impl MoneyUpdate {
    /// Total in copper. Sums rather than assuming normalized denominations.
    pub fn total_copper(&self) -> u64 {
        self.platinum as u64 * 1000
            + self.gold as u64 * 100
            + self.silver as u64 * 10
            + self.copper as u64
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyUpdateError {
    #[error("expected at least 16 bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_money_update(bytes: &[u8]) -> Result<MoneyUpdate, MoneyUpdateError> {
    if bytes.len() < 16 {
        return Err(MoneyUpdateError::BadLength(bytes.len()));
    }
    let rd = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    Ok(MoneyUpdate {
        platinum: rd(0),
        gold: rd(4),
        silver: rd(8),
        copper: rd(12),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verbatim 0x6414 payload from a live capture; the same character's profile
    // reported 9275p 10g 25s 47c at that moment.
    const CAPTURED: [u8; 20] = [
        0x3b, 0x24, 0x00, 0x00, // 9275 platinum
        0x0a, 0x00, 0x00, 0x00, // 10 gold
        0x19, 0x00, 0x00, 0x00, // 25 silver
        0x2f, 0x00, 0x00, 0x00, // 47 copper
        0x00, 0x00, 0x00, 0x00, // trailing u32, 0 in every capture so far
    ];

    #[test]
    fn parses_the_four_denominations() {
        let m = parse_money_update(&CAPTURED).unwrap();
        assert_eq!(m.platinum, 9275);
        assert_eq!(m.gold, 10);
        assert_eq!(m.silver, 25);
        assert_eq!(m.copper, 47);
        assert_eq!(m.total_copper(), 9_276_297);
    }

    #[test]
    fn sums_unnormalized_denominations() {
        // 9276p 33g 101s 281c — captured verbatim, denominations well past 10.
        let mut b = [0u8; 20];
        for (i, v) in [9276u32, 33, 101, 281].iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert_eq!(parse_money_update(&b).unwrap().total_copper(), 9_280_591);
    }

    #[test]
    fn rejects_the_12b_heartbeat_payload() {
        // 0x4d77's payload, which this parser previously mistook for money.
        assert_eq!(
            parse_money_update(&[0u8; 12]),
            Err(MoneyUpdateError::BadLength(12))
        );
    }
}
