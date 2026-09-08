//! Parser for `OP_FormattedMessage` — a length-prefixed sender name, then the
//! stock `formatId`/`msgType` header and a length-prefixed arg blob.
//!
//! ```text
//!   u32 @0    nameLen    — sender-name length, 0 on nearly every format
//!   @4        name       — nameLen bytes, not NUL-terminated
//!   u8        pad        — one byte after the name
//!   u32 @5+n  formatId   — eqstr_us.txt format-string id
//!   u32 @9+n  msgType    — message type / chat colour
//!   @13+n     args       — length-prefixed [u32 len][len bytes] slots; unused
//!                          trailing slots carry len=0 (packet size lands exact)
//! ```
//!
//! The leading name is `formattedMessageHeaderStruct` from upstream 47a4992,
//! unverified on our wire; `nameLen == 0` leaves every offset as it was. `%N`
//! interpolation stays daemon-side, where `EQStr` owns the string DB.

use thiserror::Error;

/// Fixed header length past the sender name; the arg blob starts here.
pub const HEADER_LEN: usize = 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedMessage {
    /// eqstr format-string id at @5.
    pub format_id: u32,
    /// Message type / chat colour at @9.
    pub msg_color: u32,
    /// Positional substitution args (%1..%N), in order, empty slots dropped and
    /// EQ `\x12`-wrapped links reduced to their readable name.
    pub args: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormattedMessageError {
    #[error("expected at least {HEADER_LEN} bytes, got {0}")]
    BadLength(usize),
    #[error("sender name length {0} does not fit in {1} bytes")]
    BadNameLength(usize, usize),
}

pub fn parse_formatted_message(bytes: &[u8]) -> Result<FormattedMessage, FormattedMessageError> {
    if bytes.len() < HEADER_LEN {
        return Err(FormattedMessageError::BadLength(bytes.len()));
    }
    // The header sits AFTER the sender name, whose length is the leading u32.
    let name_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    if name_len + HEADER_LEN > bytes.len() {
        return Err(FormattedMessageError::BadNameLength(name_len, bytes.len()));
    }
    let head = &bytes[name_len..];
    let format_id = u32::from_le_bytes(head[5..9].try_into().unwrap());
    let msg_color = u32::from_le_bytes(head[9..13].try_into().unwrap());
    let args = split_args(&head[HEADER_LEN..]);
    Ok(FormattedMessage {
        format_id,
        msg_color,
        args,
    })
}

/// Split the arg blob into positional args, dropping empty slots exactly as
/// `EQStr::formatMessage` does so `%N` alignment matches the client.
fn split_args(blob: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= blob.len() {
        let len = u32::from_le_bytes(blob[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if len == 0 {
            continue; // unused trailing slot
        }
        if pos + len > blob.len() {
            break; // truncated / corrupt
        }
        let text: String = blob[pos..pos + len]
            .iter()
            .map(|&byte| char::from(byte))
            .collect();
        out.push(crate::links::clean_links(&text));
        pos += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 13-byte header (fmt@5, color@9) + a length-prefixed arg blob.
    fn pkt(fmt: u32, color: u32, args: &[&[u8]]) -> Vec<u8> {
        let mut b = vec![0u8; HEADER_LEN];
        b[5..9].copy_from_slice(&fmt.to_le_bytes());
        b[9..13].copy_from_slice(&color.to_le_bytes());
        for a in args {
            b.extend_from_slice(&(a.len() as u32).to_le_bytes());
            b.extend_from_slice(a);
        }
        b
    }

    #[test]
    fn skips_a_leading_sender_name() {
        // Same packet with a 6-byte sender name prepended must decode the same.
        let plain = pkt(9072, 376, &[b"Lady Vox", b"197"]);
        let mut named = (6u32).to_le_bytes().to_vec();
        named.extend_from_slice(b"Sender");
        named.extend_from_slice(&plain[4..]);
        assert_eq!(
            parse_formatted_message(&named).unwrap(),
            parse_formatted_message(&plain).unwrap()
        );
    }

    #[test]
    fn rejects_a_name_length_past_the_payload() {
        let mut b = pkt(1, 2, &[b"a"]);
        b[0..4].copy_from_slice(&9999u32.to_le_bytes());
        assert!(matches!(
            parse_formatted_message(&b),
            Err(FormattedMessageError::BadNameLength(9999, _))
        ));
    }

    #[test]
    fn rejects_short_payload() {
        assert_eq!(
            parse_formatted_message(&[0u8; 12]),
            Err(FormattedMessageError::BadLength(12))
        );
    }

    #[test]
    fn header_only_no_args() {
        // fmt 15603 "You receive no experience…" — all-empty arg slots.
        let m = parse_formatted_message(&pkt(15603, 334, &[b"", b"", b""])).unwrap();
        assert_eq!(m.format_id, 15603);
        assert_eq!(m.msg_color, 334);
        assert!(m.args.is_empty());
    }

    #[test]
    fn combat_damage_args() {
        // fmt 9072 "%1 has taken %2 damage from your %3.%4"; str[2] is a spell link.
        let link = b"\x1263^3686^0^1^'Blood of Pain\x12";
        let m = parse_formatted_message(&pkt(9072, 376, &[b"Lady Vox", b"197", link])).unwrap();
        assert_eq!(m.format_id, 9072);
        assert_eq!(m.args, vec!["Lady Vox", "197", "Blood of Pain"]);
    }

    #[test]
    fn drops_trailing_empty_slots() {
        let m = parse_formatted_message(&pkt(1, 2, &[b"a", b"b", b"", b"", b""])).unwrap();
        assert_eq!(m.args, vec!["a", "b"]);
    }
}
