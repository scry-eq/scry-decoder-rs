//! OP_MoneyUpdate (0x4d77): running total money. `{u32 copper@0, u32=1, u32=42}`.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyUpdate {
    pub copper: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyUpdateError {
    #[error("expected at least 4 bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_money_update(bytes: &[u8]) -> Result<MoneyUpdate, MoneyUpdateError> {
    if bytes.len() < 4 {
        return Err(MoneyUpdateError::BadLength(bytes.len()));
    }
    Ok(MoneyUpdate { copper: u32::from_le_bytes(bytes[0..4].try_into().unwrap()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_copper() {
        let b = [0x04u8, 0x2f, 0, 0, 1, 0, 0, 0, 42, 0, 0, 0];
        assert_eq!(parse_money_update(&b).unwrap().copper, 12036);
    }
}
