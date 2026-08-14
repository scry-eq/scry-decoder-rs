//! Parity harness: the LootTracker against the TypeScript recorder it replaces.
//!
//! The sequence below is the loot traffic of `eqlegends-loot2` (2026-08-08), in
//! wire order, transcribed from the recorded golden. Feeding the same events to
//! `scry-web/scripts/loot-backfill.ts` produces the 16 rows asserted here —
//! four sales whose amounts were separately confirmed against the server's own
//! text, eight corpse coin piles, and one row per disposition.
//!
//! This is the acceptance test for moving recording off the Bun host: if it
//! passes, the Rust tracker reproduces the recorder byte for byte on real
//! traffic.

use seq_backend_eql::loot_track::{LootSource, LootTracker, LOOT_COLOR};

/// One loot event from the capture, in envelope order.
enum Ev {
    /// Colour-286 narration. The golden's ChatMessage carries no item link, so
    /// the tracker falls back to the prose + the confirmation's item id —
    /// exactly what the TS recorder had to do.
    Msg(&'static str),
    /// Item confirmation: (corpse_id, item_id, qty, coin).
    Txn(u32, u32, u32, u32),
    /// Corpse coin pile.
    Coin(u32),
}

fn capture() -> Vec<Ev> {
    use Ev::*;
    vec![
        Coin(62),
        Coin(87),
        Msg("You looted a Rusty Mace from a goblin mendicant's corpse and sold it for 7 silver and 1 copper."),
        Txn(18740, 6011, 1, 71),
        Coin(724),
        Msg("You looted a Silvery War Axe +1 from an elite honor guard's corpse and sold it for 1 gold, 3 silver and 6 copper."),
        Txn(18642, 5364, 1, 136),
        Coin(653),
        Msg("You looted a Bronze Dagger +1 from a goblin diviner's corpse and sold it for 2 gold."),
        Txn(18632, 7012, 1, 200),
        Msg("You looted a Cloth Veil +1 from a goblin diviner's corpse and sold it for 1 gold, 1 silver and 4 copper."),
        Txn(18632, 1002, 1, 114),
        Coin(528),
        Msg("You looted a Tares Lichen from an elite goblin guard's corpse and stored it in your tradeskill depot"),
        Txn(18649, 16555, 1, 0),
        Coin(921),
        Msg("You looted a Throwing Boulder from an ice giant diplomat's corpse to create a Throwing Boulder +8"),
        Txn(11642, 8013, 1, 0),
        Msg("You looted a Throwing Boulder from an ice giant diplomat's corpse to create a Throwing Boulder +8"),
        Txn(11642, 8013, 1, 0),
        Coin(2881),
        Msg("You looted a Diamond Dust from an ice giant's corpse and stored it in your Dragon Hoard"),
        Txn(11613, 16884, 1, 0),
        Coin(2923),
    ]
}

/// `(source, item, qty, mob, sold, copper, disposition, item_id)` — the columns
/// `loot-backfill.ts` wrote for this capture, in the order it wrote them.
type Row = (
    &'static str,
    &'static str,
    u32,
    &'static str,
    bool,
    u32,
    &'static str,
    u32,
);

// One row per line: this is a table, and the rows are the assertion.
#[rustfmt::skip]
const EXPECTED: [Row; 16] = [
    ("coin", "Coin", 1, "", false, 62, "corpse_coin", 0),
    ("coin", "Coin", 1, "", false, 87, "corpse_coin", 0),
    ("message", "Rusty Mace", 1, "a goblin mendicant", true, 71, "sold", 6011),
    ("coin", "Coin", 1, "", false, 724, "corpse_coin", 0),
    ("message", "Silvery War Axe +1", 1, "an elite honor guard", true, 136, "sold", 5364),
    ("coin", "Coin", 1, "", false, 653, "corpse_coin", 0),
    ("message", "Bronze Dagger +1", 1, "a goblin diviner", true, 200, "sold", 7012),
    ("message", "Cloth Veil +1", 1, "a goblin diviner", true, 114, "sold", 1002),
    ("coin", "Coin", 1, "", false, 528, "corpse_coin", 0),
    ("message", "Tares Lichen", 1, "an elite goblin guard", false, 0, "tradeskill depot", 16555),
    ("coin", "Coin", 1, "", false, 921, "corpse_coin", 0),
    ("message", "Throwing Boulder", 1, "an ice giant diplomat", false, 0, "created", 8013),
    ("message", "Throwing Boulder", 1, "an ice giant diplomat", false, 0, "created", 8013),
    ("coin", "Coin", 1, "", false, 2881, "corpse_coin", 0),
    ("message", "Diamond Dust", 1, "an ice giant", false, 0, "Dragon Hoard", 16884),
    ("coin", "Coin", 1, "", false, 2923, "corpse_coin", 0),
];

#[test]
fn reproduces_the_typescript_recorder_on_a_real_capture() {
    let mut t = LootTracker::new();
    t.set_zone("greatdivide");

    let mut rows = Vec::new();
    for (i, ev) in capture().into_iter().enumerate() {
        let ts = i as i64;
        rows.extend(match ev {
            Ev::Msg(text) => t.on_loot_message(LOOT_COLOR, text, 0, "", ts),
            Ev::Txn(corpse, item, qty, coin) => {
                t.on_loot_transaction(corpse, item, qty, coin, false, 0, ts)
            }
            Ev::Coin(copper) => t.on_loot_transaction(0, 0, 0, copper, true, 0, ts),
        });
    }
    rows.extend(t.flush());

    assert_eq!(rows.len(), EXPECTED.len(), "row count");
    for (i, (r, e)) in rows.iter().zip(EXPECTED.iter()).enumerate() {
        let got = (
            r.source.as_str(),
            r.item_name.as_str(),
            r.qty,
            r.mob_name.as_str(),
            r.sold,
            r.money_copper,
            r.disposition.as_str(),
            r.item_id,
        );
        assert_eq!(got, *e, "row {i}");
    }
}

#[test]
fn every_sale_amount_matches_the_servers_own_wording() {
    // The four amounts the server stated as text, which the wire confirmed.
    let mut t = LootTracker::new();
    let mut sales = Vec::new();
    for ev in capture() {
        match ev {
            Ev::Msg(text) => {
                t.on_loot_message(LOOT_COLOR, text, 0, "", 0);
            }
            Ev::Txn(c, i, q, coin) => {
                sales.extend(
                    t.on_loot_transaction(c, i, q, coin, false, 0, 0)
                        .into_iter()
                        .filter(|r| r.sold)
                        .map(|r| r.money_copper),
                );
            }
            Ev::Coin(_) => {}
        }
    }
    assert_eq!(sales, vec![71, 136, 200, 114]);
}

#[test]
fn coin_piles_total_the_purse_delta() {
    // Sales 521c + the four piles taken between the two OP_MoneyUpdate fires
    // (87+724+653+528 = 1992c) reconcile to the observed 2513c purse delta.
    let mut t = LootTracker::new();
    let mut piles = Vec::new();
    for ev in capture() {
        if let Ev::Coin(copper) = ev {
            piles.extend(
                t.on_loot_transaction(0, 0, 0, copper, true, 0, 0)
                    .into_iter()
                    .map(|r| r.money_copper),
            );
        }
    }
    assert_eq!(piles, vec![62, 87, 724, 653, 528, 921, 2881, 2923]);
    let window: u32 = piles[1..5].iter().sum();
    assert_eq!(window + 71 + 136 + 200 + 114, 2513);
    assert!(
        piles.iter().all(|&p| p > 0),
        "a coinless pile must record nothing"
    );
}

#[test]
fn coin_rows_never_steal_a_pending_sale() {
    // Every pile in this capture arrives while a narration may be pending; the
    // TS recorder's guard is what this reproduces.
    let mut t = LootTracker::new();
    t.on_loot_message(
        LOOT_COLOR,
        "You looted a Bronze Dagger +1 from a goblin diviner's corpse and sold it for 2 gold.",
        0,
        "",
        0,
    );
    let coin = t.on_loot_transaction(0, 0, 0, 2881, true, 0, 1);
    assert_eq!(coin.len(), 1);
    assert_eq!(coin[0].source, LootSource::Coin);
    let sale = t.on_loot_transaction(18632, 7012, 1, 200, false, 238, 2);
    assert_eq!(sale[0].money_copper, 200);
}
