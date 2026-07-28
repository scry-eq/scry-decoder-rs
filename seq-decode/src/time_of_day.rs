//! Parser for `OP_TimeOfDay` — payload `timeOfDayStruct`, 8 bytes.
//! The Norrath clock sync-point: hour 1-24, minute 0-59, day 1-28,
//! month 1-12, plus the game year. The trailing u16 is a placeholder.

use crate::eqstructs::timeOfDayStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<timeOfDayStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimeOfDayError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_time_of_day(bytes: &[u8]) -> Result<TimeOfDay, TimeOfDayError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(TimeOfDayError::BadLength(bytes.len()));
    }
    let raw: timeOfDayStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const timeOfDayStruct) };
    Ok(TimeOfDay {
        hour: unsafe { std::ptr::addr_of!(raw.hour).read_unaligned() },
        minute: unsafe { std::ptr::addr_of!(raw.minute).read_unaligned() },
        day: unsafe { std::ptr::addr_of!(raw.day).read_unaligned() },
        month: unsafe { std::ptr::addr_of!(raw.month).read_unaligned() },
        year: unsafe { std::ptr::addr_of!(raw.year).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_time_of_day(&[0; 7]).is_err());
        assert!(parse_time_of_day(&[0; 9]).is_err());
    }

    #[test]
    fn parses_fields() {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0] = 13; // hour
        buf[1] = 42; // minute
        buf[2] = 27; // day
        buf[3] = 11; // month
        buf[4..6].copy_from_slice(&3789u16.to_le_bytes());
        let t = parse_time_of_day(&buf).unwrap();
        assert_eq!(
            t,
            TimeOfDay {
                hour: 13,
                minute: 42,
                day: 27,
                month: 11,
                year: 3789
            }
        );
    }
}
