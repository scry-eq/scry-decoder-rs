//! Parser for `OP_GuildMemberUpdate` — a single guild member's zone / last-on
//! update (Live/Test).
//!
//! Uses the generated `GuildMemberUpdate` binding (fixed 88-byte struct). This
//! is NOT a rank change: on Live the opcode carries only the member's name and
//! current zone/last-on (legacy's `GuildMember::update` applies exactly those),
//! filling the online/offline state the full roster leaves blank (the roster
//! ships `zoneId = 0`). Rank changes arrive via a full roster re-send.
//!
//! (eql's variant diverges — it reportedly inserts a rank field — and is decoded
//! in `seq-backend-eql`; this Live parser is not used there.)

use crate::eqstructs::GuildMemberUpdate as RawUpdate;
use thiserror::Error;

pub const PAYLOAD_LEN: usize = std::mem::size_of::<RawUpdate>();

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildMemberUpdate {
    pub name: String,
    pub zone_id: u16,
    pub zone_instance: u16,
    /// Unix seconds; 0 when never / offline.
    pub last_on: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuildMemberUpdateError {
    #[error("expected {PAYLOAD_LEN} bytes, got {0}")]
    BadLength(usize),
}

fn cstr(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

pub fn parse_guild_member_update(
    bytes: &[u8],
) -> Result<GuildMemberUpdate, GuildMemberUpdateError> {
    if bytes.len() != PAYLOAD_LEN {
        return Err(GuildMemberUpdateError::BadLength(bytes.len()));
    }
    let raw: RawUpdate = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const RawUpdate) };
    let name_field = unsafe { std::ptr::addr_of!(raw.name).read_unaligned() };
    Ok(GuildMemberUpdate {
        name: cstr(&name_field),
        zone_id: unsafe { std::ptr::addr_of!(raw.zoneId).read_unaligned() },
        zone_instance: unsafe { std::ptr::addr_of!(raw.zoneInstance).read_unaligned() },
        last_on: unsafe { std::ptr::addr_of!(raw.lastOn).read_unaligned() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(name: &str, zone: u16, last_on: u32) -> Vec<u8> {
        let mut b = vec![0u8; PAYLOAD_LEN];
        b[8..8 + name.len()].copy_from_slice(name.as_bytes()); // name @8
        b[72..74].copy_from_slice(&zone.to_le_bytes()); // zoneId @72
        b[76..80].copy_from_slice(&last_on.to_le_bytes()); // lastOn @76
        b
    }

    #[test]
    fn payload_len_is_88() {
        assert_eq!(PAYLOAD_LEN, 88);
    }

    #[test]
    fn reads_name_zone_last_on() {
        let m = parse_guild_member_update(&packet("Guildmate", 50, 0x6a5d_c783)).unwrap();
        assert_eq!(m.name, "Guildmate");
        assert_eq!(m.zone_id, 50);
        assert_eq!(m.last_on, 0x6a5d_c783);
    }

    #[test]
    fn offline_zero_zone() {
        let m = parse_guild_member_update(&packet("X", 0, 0)).unwrap();
        assert_eq!(m.zone_id, 0);
    }

    #[test]
    fn wrong_size_rejected() {
        assert!(matches!(
            parse_guild_member_update(&[0u8; 80]),
            Err(GuildMemberUpdateError::BadLength(80))
        ));
    }
}
