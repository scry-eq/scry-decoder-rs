//! Parser for `OP_GuildMOTD` — the guild message of the day.
//!
//! Fixed layout, matching the stock struct (target@8, sender@72, message@140);
//! eql keeps its own copy per the isolation rule rather than casting the shared
//! C++ struct. Verified against a 656-byte capture: two leading u32 placeholders,
//! a 64-byte recipient name, a 64-byte "sender" (who set it), a u32 placeholder,
//! then the message in a fixed 516-byte tail buffer.
//!
//! ```text
//! u32, u32, char target[64], char sender[64], u32, char message[516]
//! ```
//!
//! All three char fields are NUL-terminated within their fixed width. The
//! packet carries no guild id — the MOTD is implicitly the local player's own
//! guild, so the consumer associates it with the guild it already tracks.

use thiserror::Error;

const SENDER_OFF: usize = 72;
const SENDER_LEN: usize = 64;
const MESSAGE_OFF: usize = 140;

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

/// Read a NUL-terminated string from a fixed-width field, stopping at the first
/// NUL (fixed EQ buffers carry uninitialised bytes after it).
fn cstr(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

pub fn parse_guild_motd(b: &[u8]) -> Result<GuildMotd, GuildMotdError> {
    // Everything up to the message field must be present; the message itself may
    // be a zero-length string (an unset MOTD, which is what every capture shows).
    if b.len() < MESSAGE_OFF {
        return Err(GuildMotdError::BadLength(MESSAGE_OFF, b.len()));
    }
    let sender = cstr(&b[SENDER_OFF..SENDER_OFF + SENDER_LEN]);
    let message = cstr(&b[MESSAGE_OFF..]);
    Ok(GuildMotd { message, sender })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 656-byte packet with a given sender + message, mirroring the
    /// captured shape (fixed 516-byte message tail).
    fn packet(sender: &str, message: &str) -> Vec<u8> {
        let mut b = vec![0u8; 656];
        let s = sender.as_bytes();
        b[SENDER_OFF..SENDER_OFF + s.len()].copy_from_slice(s);
        let m = message.as_bytes();
        b[MESSAGE_OFF..MESSAGE_OFF + m.len()].copy_from_slice(m);
        b
    }

    #[test]
    fn empty_motd_is_the_captured_case() {
        // All fixtures carry an unset MOTD: 656 bytes, sender + message empty.
        let b = vec![0u8; 656];
        let m = parse_guild_motd(&b).unwrap();
        assert_eq!(m.message, "");
        assert_eq!(m.sender, "");
    }

    #[test]
    fn reads_sender_and_message() {
        let b = packet("Setter", "Raid at 8, be on time");
        let m = parse_guild_motd(&b).unwrap();
        assert_eq!(m.sender, "Setter");
        assert_eq!(m.message, "Raid at 8, be on time");
    }

    #[test]
    fn stops_at_the_nul_not_the_buffer_end() {
        // Trailing bytes after the NUL (uninitialised fixed buffer) are ignored.
        let mut b = packet("A", "Hi");
        b[MESSAGE_OFF + 3] = b'X'; // garbage past the "Hi\0"
        assert_eq!(parse_guild_motd(&b).unwrap().message, "Hi");
    }

    #[test]
    fn a_short_packet_is_rejected() {
        assert!(matches!(
            parse_guild_motd(&[0u8; 100]),
            Err(GuildMotdError::BadLength(140, 100))
        ));
    }
}
