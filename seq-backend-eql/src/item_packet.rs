//! Parser for `OP_ItemPacket` (eql `0x05d5`) — every item the character owns,
//! with its template data.
//!
//! Sent S>C, 200-310KB, in answer to a 0-byte C>S request, once per zone-in
//! session. This is the ONLY source of an item's stats: loot events carry name
//! and icon but nothing else, and the PlayerProfile carries no item data at all
//! (proven by a paired capture — three deliberate item moves changed nothing in
//! it).
//!
//! ```text
//! header  u32                      NOT a record count — see below
//! record  char serial[16] NUL      per-INSTANCE id, unique per record
//!         106 B fixed block        constants + a 0xffff region, not yet decoded
//!         char name[] NUL          always at +123: the serial is fixed-width
//!         char lore_or_desc[] NUL  lore name (usually == name); a CONTAINER
//!                                  puts its description here instead
//!         field block              see FIELD_* below, offsets from here
//! ```
//!
//! **The header u32 is not the record count.** It read 167 against 271 serials;
//! `271 - 167 = 104`, exactly the equipment-storage count the game UI showed, so
//! it counts one tier while bag/storage contents sit beyond it. Walk the serials
//! instead — that is why this parser scans for the serial signature rather than
//! trusting a length prefix.
//!
//! **Records are variable-length** (stride 1056..3258, median 1090), so the
//! field block is located relative to the END of the two strings, never from the
//! record start.
//!
//! Confirmed against evidence outside the payload:
//! - `item_id` and `icon`, 6/6 exact against `loot.db`, which was recorded from
//!   loot events by an unrelated decode path.
//! - `slot_mask`, 21/21 — every distinct mask decodes to the semantically right
//!   slot, and the items disagree by CLASS (a Mask reads face, a Bow range, a
//!   two-handed instrument primary|secondary, a container 0).
//!
//! The stat block at `FIELD_STATS` is signed i32 on a 4-byte grid and is
//! deliberately NOT split into named stats yet: several items repeat a value
//! across columns, so the STR/STA/AGI/… order cannot be inferred from the wire,
//! and live's `ItemStatIndex` order must not be assumed to carry over. It is
//! exposed as a raw vector so nothing is silently mislabelled.

use crate::cursor::{Cursor, CursorError};
use thiserror::Error;

/// Offsets into the field block, which begins immediately after the lore string.
const FIELD_ITEM_ID: usize = 8;
const FIELD_SLOT_MASK: usize = 20;
const FIELD_ICON: usize = 28;
/// The stat block is a grid of 4-byte SLOTS whose value is an **i16 at slot+2**.
/// Reading a slot as a u32 yields `value << 16` — 655360 instead of 10 — which
/// is plausible enough to ship unnoticed, so read i16 at the +2.
const SLOT_STRIDE: usize = 4;
const SLOT_VALUE: usize = 2;

/// Slot indices, confirmed against an in-game tooltip (Loam Encrusted Cloak) and
/// cross-checked by item semantics across 270 records.
const SLOT_RESISTS: usize = 8;
const RESIST_COUNT: usize = 5;
const SLOT_STATS: usize = 14;
const STAT_COUNT: usize = 7;
const SLOT_HP: usize = 21;
const SLOT_MANA: usize = 22;
const SLOT_ENDURANCE: usize = 23;
const SLOT_AC: usize = 24;
/// Highest slot read, so a truncation check covers the whole block.
const SLOT_MAX: usize = SLOT_AC;

/// Name sits at a fixed offset because the serial ahead of it is fixed-width.
const NAME_OFFSET: usize = 123;
const SERIAL_LEN: usize = 16;
/// Container id, immediately after the serial + NUL. Record-relative, unlike
/// every FIELD_* above, which are relative to the post-strings block.
const RECORD_CONTAINER: usize = 21;
/// u32 = the item's LOCATION: low u16 slot within its container, high u16
/// parent bag slot (0xFFFF = top-level). Live's mainSlot/subSlot, packed.
const RECORD_LOCATION: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemTemplate {
    /// Per-INSTANCE serial ("iGS000e0002S4000"), unique per record — two copies
    /// of the same item type have different serials.
    pub serial: String,
    pub name: String,
    /// Usually identical to `name`. A container carries its description here
    /// ("Holds Giant items, Capacity 12"), so do not assume it is a lore name.
    pub lore_name: String,
    pub item_id: u32,
    pub icon: u32,
    /// Which container holds this item. Six values observed; three confirmed by
    /// exact match against the in-game Storage UI: 33 Exaltation, 37 activated
    /// key ring, 39 equipment key ring. Also 1 = carried inventory; 0 and 25 are
    /// unidentified.
    ///
    /// This is WHERE THE ITEM IS. `slot_mask` below is where it COULD go — they
    /// answer different questions and neither substitutes for the other.
    pub container_id: u32,
    /// Slot index WITHIN `container_id`. Unique per container. For
    /// `container_id == 0` with `parent_slot == 0xFFFF` this is the standard EQ
    /// slot enum: 0-22 worn (Charm..Ammo), 23-30 personal inventory, 35 cursor.
    pub container_slot: u16,
    /// Parent bag's slot when this item sits INSIDE a bag; `0xFFFF` = top-level.
    /// Together with `container_slot` this is Live's mainSlot/subSlot pair.
    pub parent_slot: u16,
    /// Standard EQ slot bitmask: bit2 head, bit3 face, bits1|4 ears, bit5 neck,
    /// bit6 shoulders, bit7 arms, bit8 back, bits9|10 wrists, bit11 range,
    /// bit12 hands, bit13 primary, bit14 secondary, bits15|16 fingers,
    /// bit17 chest, bit18 legs, bit19 feet, bit20 waist. 0 = not equippable.
    pub slot_mask: u32,
    /// `[STR, STA, AGI, DEX, CHA, INT, WIS]` — the proto's order.
    ///
    /// Confirmed against a tooltip: STR 2, AGI 8, INT 1 land exactly, with STA,
    /// DEX and WIS zero as displayed. CHA is the one caveat — the wire reads 3
    /// where the tooltip showed 4, and that tooltip had its "Unmodified" box
    /// UNCHECKED, so it was showing modified values. Treat these as BASE stats.
    pub stats: Vec<i32>,
    /// Five resists. Their internal ORDER is unverified: the one item with a
    /// tooltip carries 3 in all five, so nothing distinguishes them yet. Emitted
    /// in slot order; do not relabel without an item whose resists differ.
    pub resists: Vec<i32>,
    pub hp: i32,
    pub mana: i32,
    pub endurance: i32,
    pub ac: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemSet {
    /// The leading u32. Kept because it means something, but it is NOT the
    /// record count; do not size a buffer from it.
    pub header: u32,
    pub items: Vec<ItemTemplate>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ItemPacketError {
    #[error("payload too short for the header")]
    Short,
    #[error(transparent)]
    Cursor(#[from] CursorError),
}

/// The serial signature. Every record observed begins with this tag.
const SERIAL_TAG: &[u8] = b"iGS";

fn is_serial_at(b: &[u8], o: usize) -> bool {
    if o + SERIAL_LEN + 1 > b.len() || !b[o..].starts_with(SERIAL_TAG) {
        return false;
    }
    // 16 printable chars then a NUL — enough to reject a coincidental "iGS" in
    // the middle of a stat block.
    b[o..o + SERIAL_LEN]
        .iter()
        .all(|c| c.is_ascii_alphanumeric())
        && b[o + SERIAL_LEN] == 0
}

fn cstr_at(b: &[u8], o: usize) -> Option<(String, usize)> {
    let end = b.get(o..)?.iter().position(|&c| c == 0)? + o;
    Some((String::from_utf8_lossy(&b[o..end]).into_owned(), end + 1))
}

fn i16_at(b: &[u8], o: usize) -> Option<i16> {
    Some(i16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

/// Slot `j` sits at `tail + 4j`, and its value is the i16 two bytes in.
fn slot_offset(tail: usize, j: usize) -> usize {
    tail + j * SLOT_STRIDE + SLOT_VALUE
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

pub fn parse_item_packet(b: &[u8]) -> Result<ItemSet, ItemPacketError> {
    let mut c = Cursor::new(b);
    let header = c.read_u32_le().map_err(|_| ItemPacketError::Short)?;

    // Scan for record starts rather than striding: records are variable-length
    // and the header carries no usable count.
    let starts: Vec<usize> = (0..b.len()).filter(|&o| is_serial_at(b, o)).collect();

    let mut items = Vec::with_capacity(starts.len());
    for &lo in &starts {
        let Some((serial, _)) = cstr_at(b, lo) else {
            continue;
        };
        let Some((name, after_name)) = cstr_at(b, lo + NAME_OFFSET) else {
            continue;
        };
        let Some((lore_name, tail)) = cstr_at(b, after_name) else {
            continue;
        };

        // A serial with no name at +123 is a REFERENCE, not a record. The
        // loadout-swap tail carries both: a short reference list ahead of the
        // full records, each entry just a serial. Without this guard those
        // parse as items with an empty name that DUPLICATE a real record's
        // serial — 236 "records" for 234 real ones on a captured tail.
        if name.is_empty() {
            continue;
        }

        // A record whose field block runs past the buffer is truncated; skip it
        // rather than emitting zeros that look like real stats.
        let container_id = u32_at(b, lo + RECORD_CONTAINER).unwrap_or(0);
        let location = u32_at(b, lo + RECORD_LOCATION).unwrap_or(0xFFFF_FFFF);
        let container_slot = (location & 0xFFFF) as u16;
        let parent_slot = (location >> 16) as u16;

        let (Some(item_id), Some(slot_mask), Some(icon)) = (
            u32_at(b, tail + FIELD_ITEM_ID),
            u32_at(b, tail + FIELD_SLOT_MASK),
            u32_at(b, tail + FIELD_ICON),
        ) else {
            continue;
        };

        // Require the WHOLE stat block. A partial read would hand the consumer
        // zeros that read as real values — the failure this parser exists to
        // avoid. Real records are >=1056B, so only a clipped payload trips this.
        if slot_offset(tail, SLOT_MAX) + 2 > b.len() {
            continue;
        }

        let slot = |j: usize| i32::from(i16_at(b, slot_offset(tail, j)).unwrap_or(0));
        let stats = (0..STAT_COUNT).map(|k| slot(SLOT_STATS + k)).collect();
        let resists = (0..RESIST_COUNT).map(|k| slot(SLOT_RESISTS + k)).collect();

        items.push(ItemTemplate {
            serial,
            name,
            lore_name,
            item_id,
            icon,
            slot_mask,
            container_id,
            container_slot,
            parent_slot,
            stats,
            resists,
            hp: slot(SLOT_HP),
            mana: slot(SLOT_MANA),
            endurance: slot(SLOT_ENDURANCE),
            ac: slot(SLOT_AC),
        });
    }

    Ok(ItemSet { header, items })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one record with the real geometry: serial, the 106-byte gap, the
    /// two strings, then the field block.
    fn record(serial: &[u8; 16], name: &str, lore: &str, id: u32, slot: u32, icon: u32) -> Vec<u8> {
        record_in(serial, name, lore, id, slot, icon, 39)
    }

    /// Same, with an explicit packed location word (high = parent, low = slot).
    fn record_at(serial: &[u8; 16], name: &str, container: u32, parent: u16, slot: u16) -> Vec<u8> {
        let mut r = record_in(serial, name, name, 1, 0, 0, container);
        let loc = ((parent as u32) << 16) | slot as u32;
        r[RECORD_LOCATION..RECORD_LOCATION + 4].copy_from_slice(&loc.to_le_bytes());
        r
    }

    fn record_in(
        serial: &[u8; 16],
        name: &str,
        lore: &str,
        id: u32,
        slot: u32,
        icon: u32,
        container: u32,
    ) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(serial);
        r.push(0);
        r.resize(NAME_OFFSET, 0); // pad to the fixed name offset
        r[RECORD_CONTAINER..RECORD_CONTAINER + 4].copy_from_slice(&container.to_le_bytes());
        r.extend_from_slice(name.as_bytes());
        r.push(0);
        r.extend_from_slice(lore.as_bytes());
        r.push(0);
        let tail = r.len();
        r.resize(tail + (SLOT_MAX + 4) * SLOT_STRIDE, 0);
        r[tail + FIELD_ITEM_ID..tail + FIELD_ITEM_ID + 4].copy_from_slice(&id.to_le_bytes());
        r[tail + FIELD_SLOT_MASK..tail + FIELD_SLOT_MASK + 4].copy_from_slice(&slot.to_le_bytes());
        r[tail + FIELD_ICON..tail + FIELD_ICON + 4].copy_from_slice(&icon.to_le_bytes());
        // A negative STR, to pin the signedness and the slot arithmetic.
        let o = slot_offset(tail, SLOT_STATS);
        r[o..o + 2].copy_from_slice(&(-5i16).to_le_bytes());
        r
    }

    #[test]
    fn parses_two_records_and_keeps_them_apart() {
        let mut b = 167u32.to_le_bytes().to_vec();
        b.extend(record(
            b"iGS000e0002S4000",
            "Lightweight Bag",
            "Holds Giant items",
            177750,
            0,
            884,
        ));
        b.extend(record(
            b"iGS000e0002i4G00",
            "Apothic Crown",
            "Apothic Crown",
            1239,
            4,
            523,
        ));

        let set = parse_item_packet(&b).unwrap();
        assert_eq!(set.header, 167);
        assert_eq!(set.items.len(), 2);

        let bag = &set.items[0];
        assert_eq!(bag.name, "Lightweight Bag");
        assert_eq!(bag.lore_name, "Holds Giant items");
        assert_eq!(bag.item_id, 177750);
        assert_eq!(bag.icon, 884);
        assert_eq!(bag.slot_mask, 0, "a container is not equippable");

        let crown = &set.items[1];
        assert_eq!(crown.item_id, 1239);
        assert_eq!(crown.slot_mask, 4, "head");
        assert_eq!(crown.serial, "iGS000e0002i4G00");
        assert_eq!(crown.container_id, 39, "equipment key ring");
        assert_eq!(crown.stats.len(), STAT_COUNT);
        assert_eq!(crown.resists.len(), RESIST_COUNT);
    }

    #[test]
    fn location_splits_into_parent_and_slot() {
        let mut b = 1u32.to_le_bytes().to_vec();
        // worn: top-level in container 0, slot 5
        b.extend(record_at(b"iGS000e0000000a0", "Worn", 0, 0xFFFF, 5));
        // inside a bag: parent slot 19, position 24
        b.extend(record_at(b"iGS000e0000000b0", "InBag", 0, 19, 24));
        // key ring: its own container, so slot 5 here does NOT collide with worn
        b.extend(record_at(b"iGS000e0000000c0", "Ring", 39, 0xFFFF, 5));

        let set = parse_item_packet(&b).unwrap();
        let g = |n: &str| set.items.iter().find(|i| i.name == n).unwrap();

        assert_eq!(
            (g("Worn").parent_slot, g("Worn").container_slot),
            (0xFFFF, 5)
        );
        assert_eq!(
            (g("InBag").parent_slot, g("InBag").container_slot),
            (19, 24)
        );
        assert_eq!(g("Ring").container_id, 39);
        // Slot indices are per-container, so the same index in two containers is
        // not a conflict — this is why an unscoped uniqueness test fails.
        assert_eq!(g("Ring").container_slot, g("Worn").container_slot);
    }

    /// The name offset is fixed, but the FIELD BLOCK is not — it follows two
    /// variable-length strings. A short name must not shift the fields.
    #[test]
    fn field_block_follows_the_strings_not_the_record() {
        let mut b = 0u32.to_le_bytes().to_vec();
        b.extend(record(b"iGS000e0002S4000", "Ax", "Ax", 42, 8192, 100));
        b.extend(record(
            b"iGS000e0002i4G00",
            "A Very Much Longer Item Name Indeed",
            "A Very Much Longer Item Name Indeed",
            43,
            8192,
            101,
        ));
        let set = parse_item_packet(&b).unwrap();
        assert_eq!(set.items.len(), 2);
        assert_eq!(set.items[0].item_id, 42);
        assert_eq!(
            set.items[1].item_id, 43,
            "long name must not shift the fields"
        );
    }

    #[test]
    fn stats_are_signed() {
        let mut b = 0u32.to_le_bytes().to_vec();
        b.extend(record(
            b"iGS000e0002S4000",
            "White Satin Gloves",
            "x",
            1,
            4096,
            1,
        ));
        let set = parse_item_packet(&b).unwrap();
        assert_eq!(set.items[0].stats[0], -5, "STR is signed");
    }

    #[test]
    fn a_truncated_record_is_skipped_not_zero_filled() {
        let mut b = 0u32.to_le_bytes().to_vec();
        let r = record(b"iGS000e0002S4000", "Cut Short", "x", 7, 4, 9);
        b.extend(&r[..r.len() - 40]); // chop the field block
        assert!(parse_item_packet(&b).unwrap().items.is_empty());
    }

    /// End-to-end against a REAL capture dump, when one is available locally.
    /// Self-skips otherwise: captures are gitignored personal data, so this
    /// cannot ship a fixture. Produce one with:
    ///   scryd --replay <cap>.vpk --config-dir conf --no-listen \
    ///       --dump-all-sessions --dump-payload 0x05d5:/tmp/pp
    ///   SEQ_ITEM_PACKET_FIXTURE=/tmp/pp.1.bin cargo test -p seq-backend-eql
    #[test]
    fn parses_a_real_capture_when_one_is_present() {
        let Ok(path) = std::env::var("SEQ_ITEM_PACKET_FIXTURE") else {
            return;
        };
        let b = std::fs::read(&path).expect("fixture unreadable");
        let set = parse_item_packet(&b).expect("real payload must parse");

        assert!(
            set.items.len() > 100,
            "expected a full item set, got {}",
            set.items.len()
        );
        // Serials are per-instance, so they must not repeat.
        let uniq: std::collections::HashSet<_> = set.items.iter().map(|i| &i.serial).collect();
        assert_eq!(
            uniq.len(),
            set.items.len(),
            "serials must be unique per record"
        );
        // Every record must carry a name and a plausible id.
        assert!(set
            .items
            .iter()
            .all(|i| !i.name.is_empty() && i.item_id > 0));
        // Stat columns are fixed-width for every record.
        assert!(set.items.iter().all(|i| i.stats.len() == STAT_COUNT));
        assert!(set.items.iter().all(|i| i.resists.len() == RESIST_COUNT));
        eprintln!("parsed {} items from {}", set.items.len(), path);
    }

    /// The mapping, pinned against a real in-game tooltip (Loam Encrusted
    /// Cloak: AC 5, Mana 5, STR 2, AGI 8, INT 1, five resists at 3).
    #[test]
    fn slots_map_to_the_named_fields() {
        let mut r = record(
            b"iGS000e0002S4000",
            "Loam Encrusted Cloak",
            "x",
            1646,
            256,
            660,
        );
        let tail = r.len() - (SLOT_MAX + 4) * SLOT_STRIDE;
        let mut put = |j: usize, v: i16| {
            let o = slot_offset(tail, j);
            r[o..o + 2].copy_from_slice(&v.to_le_bytes());
        };
        for k in 0..RESIST_COUNT {
            put(SLOT_RESISTS + k, 3);
        }
        put(SLOT_STATS, 2); // STR
        put(SLOT_STATS + 2, 8); // AGI
        put(SLOT_STATS + 5, 1); // INT
        put(SLOT_MANA, 5);
        put(SLOT_AC, 5);

        let mut b = 0u32.to_le_bytes().to_vec();
        b.extend(r);
        let it = &parse_item_packet(&b).unwrap().items[0];

        assert_eq!(
            it.stats,
            vec![2, 0, 8, 0, 0, 1, 0],
            "STR STA AGI DEX CHA INT WIS"
        );
        assert_eq!(it.resists, vec![3; RESIST_COUNT]);
        assert_eq!(it.ac, 5);
        assert_eq!(it.mana, 5);
        assert_eq!(it.hp, 0, "the cloak has none");
        assert_eq!(it.endurance, 0);
    }

    /// A slot read as u32 yields `value << 16` (655360 for 10). The value is an
    /// i16 at slot+2, and getting that wrong produces numbers plausible enough
    /// to ship.
    #[test]
    fn a_slot_value_is_an_i16_two_bytes_in() {
        let mut r = record(b"iGS000e0002S4000", "X", "x", 1, 0, 1);
        let tail = r.len() - (SLOT_MAX + 4) * SLOT_STRIDE;
        let o = slot_offset(tail, SLOT_AC);
        r[o..o + 2].copy_from_slice(&10i16.to_le_bytes());
        // The slot's first two bytes stay zero, so a u32 read would see 10<<16.
        assert_eq!(&r[o - 2..o], &[0, 0]);

        let mut b = 0u32.to_le_bytes().to_vec();
        b.extend(r);
        assert_eq!(parse_item_packet(&b).unwrap().items[0].ac, 10);
    }

    #[test]
    fn an_empty_payload_is_an_error_not_a_panic() {
        assert_eq!(parse_item_packet(&[]).unwrap_err(), ItemPacketError::Short);
    }
}
