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
/// Signed i32 grid. Not 4-byte aligned to the block start — read on a block
/// aligned grid the values land in the HIGH half of each u32 and decode as
/// garbage (655360 rather than 10).
const FIELD_STATS: usize = 46;
const STAT_COLUMNS: usize = 14;

/// Name sits at a fixed offset because the serial ahead of it is fixed-width.
const NAME_OFFSET: usize = 123;
const SERIAL_LEN: usize = 16;
/// Container id, immediately after the serial + NUL. Record-relative, unlike
/// every FIELD_* above, which are relative to the post-strings block.
const RECORD_CONTAINER: usize = 21;

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
    /// Standard EQ slot bitmask: bit2 head, bit3 face, bits1|4 ears, bit5 neck,
    /// bit6 shoulders, bit7 arms, bit8 back, bits9|10 wrists, bit11 range,
    /// bit12 hands, bit13 primary, bit14 secondary, bits15|16 fingers,
    /// bit17 chest, bit18 legs, bit19 feet, bit20 waist. 0 = not equippable.
    pub slot_mask: u32,
    /// Raw stat columns, signed. UNLABELLED on purpose — see the module docs.
    pub stats: Vec<i32>,
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

fn i32_at(b: &[u8], o: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
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

        // A record whose field block runs past the buffer is truncated; skip it
        // rather than emitting zeros that look like real stats.
        let container_id = u32_at(b, lo + RECORD_CONTAINER).unwrap_or(0);

        let (Some(item_id), Some(slot_mask), Some(icon)) = (
            u32_at(b, tail + FIELD_ITEM_ID),
            u32_at(b, tail + FIELD_SLOT_MASK),
            u32_at(b, tail + FIELD_ICON),
        ) else {
            continue;
        };

        // Require the WHOLE stat block. A partial read would hand the consumer a
        // short vector that reads as "these stats are zero" — the same
        // plausible-zeros failure this parser exists to avoid. Real records are
        // >=1056B, so only a clipped payload ever trips this.
        let Some(stats) = (0..STAT_COLUMNS)
            .map(|j| i32_at(b, tail + FIELD_STATS + j * 4))
            .collect::<Option<Vec<i32>>>()
        else {
            continue;
        };

        items.push(ItemTemplate {
            serial,
            name,
            lore_name,
            item_id,
            icon,
            slot_mask,
            container_id,
            stats,
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
        r.resize(tail + FIELD_STATS + STAT_COLUMNS * 4 + 8, 0);
        r[tail + FIELD_ITEM_ID..tail + FIELD_ITEM_ID + 4].copy_from_slice(&id.to_le_bytes());
        r[tail + FIELD_SLOT_MASK..tail + FIELD_SLOT_MASK + 4].copy_from_slice(&slot.to_le_bytes());
        r[tail + FIELD_ICON..tail + FIELD_ICON + 4].copy_from_slice(&icon.to_le_bytes());
        // one negative stat, to pin the signedness
        r[tail + FIELD_STATS..tail + FIELD_STATS + 4].copy_from_slice(&(-5i32).to_le_bytes());
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
        assert_eq!(set.items[0].stats[0], -5, "stat columns carry negatives");
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
    ///   showeq-daemon --replay <cap>.vpk --config-dir conf --no-listen \
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
        assert!(set.items.iter().all(|i| i.stats.len() == STAT_COLUMNS));
        eprintln!("parsed {} items from {}", set.items.len(), path);
    }

    #[test]
    fn an_empty_payload_is_an_error_not_a_panic() {
        assert_eq!(parse_item_packet(&[]).unwrap_err(), ItemPacketError::Short);
    }
}
