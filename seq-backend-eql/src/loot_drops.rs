//! OP_LootDrops (0x6768): corpse loot window.
//! `{u16, u32=900, u32 corpseId@6, u32 count@10, NUL-term corpse name@14, then
//! `count` item entries}`. Per entry: field[2]@+8 = item icon id, name = the
//! 0x12-wrapped item link past the shared 197-hex header.

use crate::links::ITEM_LINK_HEX;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootItem {
    pub name: String,
    pub icon: u32,
    pub item_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootDrops {
    pub corpse_id: u32,
    pub corpse_name: String,
    pub items: Vec<LootItem>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LootDropsError {
    #[error("payload too short: {0}")]
    BadLength(usize),
    #[error("corpse name not NUL-terminated")]
    UnterminatedName,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

pub fn parse_loot_drops(bytes: &[u8]) -> Result<LootDrops, LootDropsError> {
    if bytes.len() < 15 {
        return Err(LootDropsError::BadLength(bytes.len()));
    }
    let corpse_id = u32le(bytes, 6);
    let count = u32le(bytes, 10) as usize;
    let nul = bytes[14..]
        .iter()
        .position(|&c| c == 0)
        .ok_or(LootDropsError::UnterminatedName)?
        + 14;
    let corpse_name = String::from_utf8_lossy(&bytes[14..nul]).into_owned();

    let mut items = Vec::with_capacity(count.min(bytes.len() / 12 + 1));
    let mut pos = nul + 1;
    while items.len() < count && pos + 12 <= bytes.len() {
        let icon = u32le(bytes, pos + 8); // item entry field[2]
        let Some(open) = bytes[pos..].iter().position(|&c| c == 0x12) else {
            break;
        };
        let open = pos + open;
        let Some(close) = bytes[open + 1..].iter().position(|&c| c == 0x12) else {
            break;
        };
        let close = open + 1 + close;
        let body = &bytes[open + 1..close];
        let item_id = crate::links::item_id_from_link(body);
        let name = String::from_utf8_lossy(body.get(ITEM_LINK_HEX..).unwrap_or(&[])).into_owned();
        items.push(LootItem {
            name,
            icon,
            item_id,
        });
        pos = close + 1;
    }
    Ok(LootDrops {
        corpse_id,
        corpse_name,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(entry_fields: &[u32; 3], name: &str, item_id: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for f in entry_fields {
            b.extend_from_slice(&f.to_le_bytes()); // field[0..2]; [2]=icon
        }
        b.extend_from_slice(name.as_bytes());
        b.push(0); // plain name terminator
        b.push(0x12);
        // item id = leading 6 hex of the link body, then zero-padding to length.
        let mut hdr = format!("{item_id:06X}").into_bytes();
        hdr.resize(ITEM_LINK_HEX, b'0');
        b.extend_from_slice(&hdr);
        b.extend_from_slice(name.as_bytes());
        b.push(0x12);
        b
    }

    #[test]
    fn parses_items_with_icons() {
        let mut b = vec![1, 0];
        b.extend_from_slice(&900u32.to_le_bytes());
        b.extend_from_slice(&11613u32.to_le_bytes()); // corpse_id
        b.extend_from_slice(&2u32.to_le_bytes()); // count
        b.extend_from_slice(b"Lady Vox\0");
        b.extend_from_slice(&item(&[11607, 1, 593], "McVaxius` Horn of War", 11607));
        b.extend_from_slice(&item(&[9240, 0, 552], "White Dragon Hide", 9240));
        let l = parse_loot_drops(&b).unwrap();
        assert_eq!(l.corpse_id, 11613);
        assert_eq!(l.corpse_name, "Lady Vox");
        assert_eq!(l.items.len(), 2);
        assert_eq!(
            l.items[0],
            LootItem {
                name: "McVaxius` Horn of War".into(),
                icon: 593,
                item_id: 11607
            }
        );
        assert_eq!(
            l.items[1],
            LootItem {
                name: "White Dragon Hide".into(),
                icon: 552,
                item_id: 9240
            }
        );
    }
}

#[cfg(test)]
mod alloc_bounds_tests {
    use super::*;

    #[test]
    fn an_absurd_item_count_does_not_allocate() {
        let mut b = vec![0u8; 14];
        b[6..10].copy_from_slice(&42u32.to_le_bytes()); // corpse id
        b[10..14].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // count
        b.extend_from_slice(b"A corpse\0");
        // Returns what the payload actually holds (no items) without trying to
        // reserve 4e9 of them.
        let d = parse_loot_drops(&b).unwrap();
        assert!(d.items.is_empty());
    }
}
