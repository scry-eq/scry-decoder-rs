//! Diff harness: bindgen-generated parser vs the original hand-rolled
//! parser on the same arbitrary 14-byte payloads. Failure here pinpoints
//! exactly where the bindgen port diverges from the hand-rolled reference.

use seq_decode::{parse_mob_update, MobUpdate};

/// Hand-rolled reference, copy-pasted verbatim from the pre-bindgen
/// version of seq-decode/src/mob_update.rs.
fn parse_reference(bytes: &[u8]) -> Option<MobUpdate> {
    if bytes.len() != 14 {
        return None;
    }
    let spawn_id = u16::from_le_bytes([bytes[0], bytes[1]]);
    let bf64 = u64::from_le_bytes([
        bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
    ]);
    let y = sign_extend_19((bf64 & 0x7_FFFF) as u32) >> 3;
    let z = sign_extend_19(((bf64 >> 19) & 0x7_FFFF) as u32) >> 3;
    let x = sign_extend_19(((bf64 >> 45) & 0x7_FFFF) as u32) >> 3;
    let bf16 = u16::from_le_bytes([bytes[12], bytes[13]]);
    let heading = bf16 & 0x0FFF;
    Some(MobUpdate {
        spawn_id,
        x,
        y,
        z,
        heading,
    })
}

fn sign_extend_19(v: u32) -> i32 {
    let m = v & 0x7_FFFF;
    if m & 0x4_0000 != 0 {
        (m | 0xFFF8_0000) as i32
    } else {
        m as i32
    }
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed
}

#[test]
fn bindgen_matches_hand_rolled_on_random_inputs() {
    let mut seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let mut diffs = Vec::new();
    for _ in 0..10_000 {
        let mut bytes = [0u8; 14];
        for chunk in bytes.chunks_mut(8) {
            let r = lcg(&mut seed);
            for (i, b) in chunk.iter_mut().enumerate() {
                *b = (r >> (i * 8)) as u8;
            }
        }
        let bindgen = parse_mob_update(&bytes).unwrap();
        let reference = parse_reference(&bytes).unwrap();
        if bindgen != reference {
            diffs.push((bytes, reference, bindgen));
            if diffs.len() >= 5 {
                break;
            }
        }
    }
    if !diffs.is_empty() {
        for (bytes, r, b) in &diffs {
            eprintln!("payload: {:02x?}", bytes);
            eprintln!("  reference: {:?}", r);
            eprintln!("  bindgen:   {:?}", b);
            eprintln!();
        }
        panic!("{} divergent samples", diffs.len());
    }
}
