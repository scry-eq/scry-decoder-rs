# showeq-decoder-rs

Parallel Rust implementation of ShowEQ's packet decoder. Lives alongside
the C++ daemon (`showeq-daemon`); migrated into the daemon per-opcode via
the `--rust-opcodes` flag once each parser is verified against the C++
reference and the tier-2 byte-cmp regression harness.

See `MODERNIZATION_PLAN.md` (sibling repo `showeq-daemon`'s parent
directory) for the staged migration path. Stage A scope: `OP_MobUpdate`.

## Workspace

| Crate           | Purpose                                              |
|-----------------|------------------------------------------------------|
| `seq-eqstructs` | `bindgen`-generated Rust mirrors of `everquest.h`. Allowlist grows per ported opcode. |
| `seq-decode`    | Pure parsers — `&[u8]` payload → typed struct, built on `seq-eqstructs`. |
| `seq-bridge`    | `cxx` FFI shim — exposes `seq-decode` to C++ via Corrosion. |

Future stages add `seq-opcodes` (XML opcode-table loader), `seq-replay`
(`.vpk` reader), and `seq-cli` (standalone Rust binary that reads pcap or
`.vpk` and emits `.pbstream`). Out of scope for Stage A.

## Build

```sh
cargo build         # builds all crates
cargo test          # runs unit + golden tests
```

`seq-eqstructs/build.rs` runs `bindgen` against
`../../showeq-daemon/src/everquest.h` (sibling-relative). Override with
`EVERQUEST_H=/path/to/everquest.h` for out-of-tree builds. Requires
`libclang-dev` (Debian/Ubuntu) or equivalent on the build host.

## License

GPL-2.0 — see [`LICENSE`](LICENSE). Matches `showeq` and `showeq-daemon`,
which permits direct consumption of `everquest.h` via `bindgen`.
