//! Parser for `OP_SelfPosEQL` (0x4fb6, C>S) — the local player's position-history
//! breadcrumb. Distinct from the 38-byte live `OP_ClientUpdate` (0x5188): this is
//! a batched movement journal, not a per-frame update. Cracked 2026-07-17 against
//! the run-south-then-west `/loc` capture (all 3 points exact; see
//! OPCODES_LEGENDS.md and [[project_eql_two_self_pos_opcodes]]).
//!
//! Layout = `N × 17-byte record + 1 trailing byte`:
//! ```text
//!   per record (17 bytes):
//!   /*00*/ f32 y    (gameY, /loc order)
//!   /*04*/ f32 x    (gameX)
//!   /*08*/ f32 z    (gameZ)
//!   /*12*/ u8  seq  (1..2, per-batch sub-sequence — unused here)
//!   /*13*/ u32 ts   (monotonic hi-res timer, ~65543 units/sample)
//! ```
//! Sizes seen: 18B = 1 record (a single settled sample) up to 2415B = 142 records.
//! Timestamps are monotonic and the samples trace a smooth walk — the client's
//! movement journal, surfaced as a trail overlay.

/// Byte length of one position record.
pub const RECORD_LEN: usize = 17;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreadcrumbPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub ts: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelfPosBreadcrumb {
    pub points: Vec<BreadcrumbPoint>,
}

fn read_u32_le(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn read_f32_le(b: &[u8], at: usize) -> f32 {
    f32::from_bits(read_u32_le(b, at))
}

/// Parse the breadcrumb into its ordered position samples. Structural-only: the
/// payload must be a whole number of 17-byte records plus at most one trailing
/// byte, else an empty breadcrumb is returned (the caller drops empties). Order
/// is preserved (oldest → newest).
pub fn parse_self_pos_breadcrumb(b: &[u8]) -> SelfPosBreadcrumb {
    let n = b.len() / RECORD_LEN;
    let rem = b.len() - n * RECORD_LEN;
    if n == 0 || rem > 1 {
        return SelfPosBreadcrumb::default();
    }
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let o = i * RECORD_LEN;
        points.push(BreadcrumbPoint {
            y: read_f32_le(b, o),
            x: read_f32_le(b, o + 4),
            z: read_f32_le(b, o + 8),
            ts: read_u32_le(b, o + 13),
        });
    }
    SelfPosBreadcrumb { points }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(y: f32, x: f32, z: f32, seq: u8, ts: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity(RECORD_LEN);
        v.extend_from_slice(&y.to_le_bytes());
        v.extend_from_slice(&x.to_le_bytes());
        v.extend_from_slice(&z.to_le_bytes());
        v.push(seq);
        v.extend_from_slice(&ts.to_le_bytes());
        v
    }

    #[test]
    fn single_record_plus_trailing() {
        // 18B = 1 record + 1 trailing byte, the settled-sample form.
        let mut buf = rec(941.5, 654.25, 190.0, 2, 33211);
        buf.push(0x00); // trailing
        assert_eq!(buf.len(), 18);
        let out = parse_self_pos_breadcrumb(&buf);
        assert_eq!(out.points.len(), 1);
        assert_eq!(out.points[0].x, 654.25);
        assert_eq!(out.points[0].y, 941.5);
        assert_eq!(out.points[0].z, 190.0);
        assert_eq!(out.points[0].ts, 33211);
    }

    #[test]
    fn multi_record_preserves_order() {
        let mut buf = rec(1.0, 2.0, 3.0, 1, 100);
        buf.extend(rec(4.0, 5.0, 6.0, 1, 200));
        buf.extend(rec(7.0, 8.0, 9.0, 2, 300));
        buf.push(0x00);
        let out = parse_self_pos_breadcrumb(&buf);
        assert_eq!(out.points.len(), 3);
        assert_eq!((out.points[0].x, out.points[0].ts), (2.0, 100));
        assert_eq!((out.points[2].x, out.points[2].ts), (8.0, 300));
    }

    #[test]
    fn rejects_ragged_length() {
        // remainder > 1 byte is not a valid record boundary.
        assert!(parse_self_pos_breadcrumb(&[0u8; RECORD_LEN + 5])
            .points
            .is_empty());
        assert!(parse_self_pos_breadcrumb(&[0u8; 3]).points.is_empty());
        assert!(parse_self_pos_breadcrumb(&[]).points.is_empty());
    }
}
