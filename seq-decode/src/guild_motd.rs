//! Parser for `OP_GuildMOTD` — the guild message of the day (Live/Test).
//!
//! Uses the generated `guildMOTDStruct` binding for the fixed header (a
//! `char message[0]` flexible member makes `size_of` the 140-byte header); the
//! message is the variable tail after it. eql keeps a separate parser in
//! `seq-backend-eql` — each backend owns its own.

use crate::eqstructs::guildMOTDStruct;
use thiserror::Error;

/// The fixed header (`size_of` stops at the flexible `message[0]`).
pub const HEADER_LEN: usize = std::mem::size_of::<guildMOTDStruct>();

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildMotd {
    /// The MOTD text; empty when the guild has none set.
    pub message: String,
    /// Who last set it; empty when never set.
    pub sender: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuildMotdError {
    #[error("expected at least {0} bytes, got {1}")]
    BadLength(usize, usize),
}

/// A NUL-terminated string from a fixed field (EQ buffers carry uninitialised
/// bytes after the NUL).
fn cstr(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

pub fn parse_guild_motd(bytes: &[u8]) -> Result<GuildMotd, GuildMotdError> {
    // The header must be present; the message may be zero-length (unset MOTD).
    if bytes.len() < HEADER_LEN {
        return Err(GuildMotdError::BadLength(HEADER_LEN, bytes.len()));
    }
    let raw: guildMOTDStruct =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const guildMOTDStruct) };
    let sender_field = unsafe { std::ptr::addr_of!(raw.sender).read_unaligned() };
    let sender = cstr(&sender_field);
    // The flexible `message[0]` tail begins right after the header.
    let message = cstr(&bytes[HEADER_LEN..]);
    Ok(GuildMotd { message, sender })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(sender: &str, message: &str) -> Vec<u8> {
        // Full 656-byte buffer (NUL-padded), like the wire; 72 = sender, 140 = message.
        let mut b = vec![0u8; 656];
        b[72..72 + sender.len()].copy_from_slice(sender.as_bytes());
        b[140..140 + message.len()].copy_from_slice(message.as_bytes());
        b
    }

    #[test]
    fn header_len_is_140() {
        assert_eq!(HEADER_LEN, 140);
    }

    #[test]
    fn empty_motd() {
        let m = parse_guild_motd(&vec![0u8; 656]).unwrap();
        assert_eq!(m.message, "");
        assert_eq!(m.sender, "");
    }

    #[test]
    fn reads_sender_and_message() {
        let m = parse_guild_motd(&packet("Setter", "Raid at 8")).unwrap();
        assert_eq!(m.sender, "Setter");
        assert_eq!(m.message, "Raid at 8");
    }

    #[test]
    fn stops_at_nul() {
        let mut b = packet("A", "Hi");
        b[140 + 3] = b'X'; // garbage after the "Hi\0" in the fixed buffer
        assert_eq!(parse_guild_motd(&b).unwrap().message, "Hi");
    }

    #[test]
    fn short_packet_rejected() {
        assert!(matches!(
            parse_guild_motd(&[0u8; 100]),
            Err(GuildMotdError::BadLength(140, 100))
        ));
    }
}
