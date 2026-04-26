//! Golden-byte tests for `parse_mob_update` — payloads built with the
//! same bit-packing recipe as the C++ probe (`g++` against the actual
//! everquest.h struct, see PR description / Stage A plan), so any
//! divergence between Rust and C++ extraction shows up here first.

use seq_decode::{parse_mob_update, MobUpdate, MOB_UPDATE_LEN};

fn build_payload(spawn_id: u16, y_19: i32, z_19: i32, x_19: i32, heading_12: u16) -> [u8; MOB_UPDATE_LEN] {
    let mut buf = [0u8; MOB_UPDATE_LEN];
    buf[0..2].copy_from_slice(&spawn_id.to_le_bytes());
    let pack19 = |v: i32| (v as u32) & 0x7_FFFF;
    let bf64: u64 = (pack19(y_19) as u64)
        | ((pack19(z_19) as u64) << 19)
        | ((pack19(x_19) as u64) << 45);
    buf[4..12].copy_from_slice(&bf64.to_le_bytes());
    buf[12..14].copy_from_slice(&(heading_12 & 0x0FFF).to_le_bytes());
    buf
}

#[test]
fn matches_cpp_probe_pattern() {
    // Same inputs as /tmp/bitprobe.cpp during Stage A bring-up.
    // Probe output: spawnId=0x1234, y=1, z=2, x=-1, heading=0xABC,
    // y>>3=0, z>>3=0, x>>3=-1.
    let buf = build_payload(0x1234, 1, 2, -1, 0xABC);
    assert_eq!(
        parse_mob_update(&buf).unwrap(),
        MobUpdate { spawn_id: 0x1234, y: 0, z: 0, x: -1, heading: 0xABC },
    );
}

#[test]
fn coordinate_extremes() {
    // 19-bit signed range: -262_144 ..= 262_143; the >> 3 in the
    // parser maps that to the int16 game-world range.
    let buf = build_payload(1, 262_143, -262_144, 0, 0);
    let r = parse_mob_update(&buf).unwrap();
    assert_eq!(r.y, 32_767);
    assert_eq!(r.z, -32_768);
    assert_eq!(r.x, 0);
}

#[test]
fn heading_full_range() {
    for h in [0u16, 1, 2047, 2048, 4094, 4095] {
        let buf = build_payload(1, 0, 0, 0, h);
        assert_eq!(parse_mob_update(&buf).unwrap().heading, h, "heading={h}");
    }
}

#[test]
fn unused2_does_not_leak_into_heading() {
    // unused2 sits at bits 12..16 of the second u16. Set them and
    // confirm parse masks them off.
    let mut buf = build_payload(1, 0, 0, 0, 0x123);
    buf[13] |= 0xF0;
    assert_eq!(parse_mob_update(&buf).unwrap().heading, 0x123);
}

#[test]
fn unk1_bytes_ignored() {
    let mut buf = build_payload(0xBEEF, 100, -50, 200, 1234);
    let baseline = parse_mob_update(&buf).unwrap();
    buf[2] = 0xAB;
    buf[3] = 0xCD;
    assert_eq!(parse_mob_update(&buf).unwrap(), baseline);
}

#[test]
fn u3_field_ignored() {
    // u3:7 sits at bits 38..45 of the int64_t bitfield. Setting it
    // must not perturb x/y/z extraction.
    let baseline_buf = build_payload(1, 100, -50, 200, 0);
    let baseline = parse_mob_update(&baseline_buf).unwrap();
    let mut buf = baseline_buf;
    let bf64 = u64::from_le_bytes(buf[4..12].try_into().unwrap()) | (0x7Fu64 << 38);
    buf[4..12].copy_from_slice(&bf64.to_le_bytes());
    let r = parse_mob_update(&buf).unwrap();
    assert_eq!(r.x, baseline.x);
    assert_eq!(r.y, baseline.y);
    assert_eq!(r.z, baseline.z);
}

#[test]
fn spawn_id_high_bit() {
    // Spawn IDs > 0x7FFF: in the C struct the field is int16_t but
    // the daemon casts to uint16_t at the call site, so high IDs
    // must round-trip as their unsigned value.
    let buf = build_payload(0xFFFF, 0, 0, 0, 0);
    assert_eq!(parse_mob_update(&buf).unwrap().spawn_id, 0xFFFF);
}
