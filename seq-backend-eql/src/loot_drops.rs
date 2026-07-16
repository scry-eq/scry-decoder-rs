//! OP_LootDrops (0x6768): corpse loot window.
//! `{u16, u32=900, u32 corpseId@6, u32 count@10, NUL-term corpse name@14, then
//! `count` item entries, each carrying a 0x12-wrapped item link}`.

use crate::links::ITEM_LINK_HEX;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootDrops {
    pub corpse_id: u32,
    pub corpse_name: String,
    pub item_names: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LootDropsError {
    #[error("payload too short: {0}")]
    BadLength(usize),
    #[error("corpse name not NUL-terminated")]
    UnterminatedName,
}

pub fn parse_loot_drops(bytes: &[u8]) -> Result<LootDrops, LootDropsError> {
    if bytes.len() < 15 {
        return Err(LootDropsError::BadLength(bytes.len()));
    }
    let corpse_id = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let count = u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize;
    let nul = bytes[14..]
        .iter()
        .position(|&c| c == 0)
        .ok_or(LootDropsError::UnterminatedName)?
        + 14;
    let corpse_name = String::from_utf8_lossy(&bytes[14..nul]).into_owned();

    // one 0x12-wrapped item link per lootable slot; name = link past the hex header.
    let mut item_names = Vec::with_capacity(count);
    let mut i = nul + 1;
    while item_names.len() < count {
        let Some(open) = bytes[i..].iter().position(|&c| c == 0x12) else { break };
        let open = i + open;
        let Some(close) = bytes[open + 1..].iter().position(|&c| c == 0x12) else { break };
        let close = open + 1 + close;
        let link = &bytes[open + 1..close];
        item_names.push(String::from_utf8_lossy(link.get(ITEM_LINK_HEX..).unwrap_or(&[])).into_owned());
        i = close + 1;
    }
    Ok(LootDrops { corpse_id, corpse_name, item_names })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_corpse_and_items() {
        let mut b = vec![1, 0];                       // u16
        b.extend_from_slice(&900u32.to_le_bytes());   // @2
        b.extend_from_slice(&11613u32.to_le_bytes()); // corpse_id @6
        b.extend_from_slice(&2u32.to_le_bytes());     // count @10
        b.extend_from_slice(b"Lady Vox\0");           // corpse name @14
        for n in ["Fine Steel", "Diamond Dust"] {
            b.push(0x12);
            b.extend_from_slice(&vec![b'0'; ITEM_LINK_HEX]);
            b.extend_from_slice(n.as_bytes());
            b.push(0x12);
        }
        let l = parse_loot_drops(&b).unwrap();
        assert_eq!(l.corpse_id, 11613);
        assert_eq!(l.corpse_name, "Lady Vox");
        assert_eq!(l.item_names, vec!["Fine Steel", "Diamond Dust"]);
    }
}
