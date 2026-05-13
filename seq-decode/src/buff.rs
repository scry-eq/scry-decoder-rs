//! Parser for `OP_Buff` — payload `buffStruct`, 168 bytes.
//! Resolved 2026-04-25/28 (combat + buff captures, n=3 cumulative
//! at 168B S>C with zero competitors). The same opcode handles
//! buff application, refresh, and fade — distinguished by
//! `changetype` (1 = fading, 2 = duration update). Daemon uses
//! spawn_id, spell_id, duration, level, spell_slot, change_type.

use seq_eqstructs::buffStruct;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<buffStruct>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buff {
    pub spawn_id: u32,
    pub spell_id: u32,
    pub duration: u32,
    pub level: u8,
    pub spell_slot: u32,
    pub change_type: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuffError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

pub fn parse_buff(bytes: &[u8]) -> Result<Buff, BuffError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(BuffError::BadLength(bytes.len()));
    }
    let raw: buffStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const buffStruct) };
    Ok(Buff {
        spawn_id:    unsafe { std::ptr::addr_of!(raw.spawnid).read_unaligned() },
        spell_id:    unsafe { std::ptr::addr_of!(raw.spellid).read_unaligned() },
        duration:    unsafe { std::ptr::addr_of!(raw.duration).read_unaligned() },
        level:       unsafe { std::ptr::addr_of!(raw.level).read_unaligned() },
        spell_slot:  unsafe { std::ptr::addr_of!(raw.spellslot).read_unaligned() },
        change_type: unsafe { std::ptr::addr_of!(raw.changetype).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_buff(&[0; 167]).is_err());
        assert!(parse_buff(&[0; 169]).is_err());
    }

    #[test]
    fn parses_buff_apply() {
        // changetype=2 (duration update) at offset 164.
        let mut buf = vec![0u8; PAYLOAD_LEN];
        buf[0..4].copy_from_slice(&123u32.to_le_bytes());      // spawnid
        buf[116..120].copy_from_slice(&5024u32.to_le_bytes()); // spellid
        buf[120..124].copy_from_slice(&3600u32.to_le_bytes()); // duration
        buf[152] = 60u8;                                          // level
        buf[160..164].copy_from_slice(&3u32.to_le_bytes());    // spell_slot
        buf[164..168].copy_from_slice(&2u32.to_le_bytes());    // changetype
        let b = parse_buff(&buf).unwrap();
        assert_eq!(b.spawn_id, 123);
        assert_eq!(b.spell_id, 5024);
        assert_eq!(b.duration, 3600);
        assert_eq!(b.level, 60);
        assert_eq!(b.spell_slot, 3);
        assert_eq!(b.change_type, 2);
    }
}
