# showeq-rust-decoder

Parallel Rust implementation of ShowEQ's packet decoder. Lives alongside
the C++ daemon (`showeq-daemon`); migrated into the daemon per-opcode via
the `--rust-opcodes` flag once each parser is verified against the C++
reference and the tier-2 byte-cmp regression harness.

See `MODERNIZATION_PLAN.md` (sibling repo `showeq-daemon`'s parent
directory) for the staged migration path. Stage A scope: `OP_MobUpdate`.

## Workspace

| Crate         | Purpose                                              |
|---------------|------------------------------------------------------|
| `seq-decode`  | Pure parsers — `&[u8]` payload → typed struct.       |
| `seq-bridge`  | `cxx` FFI shim — exposes `seq-decode` to C++ via Corrosion. |

Future stages add `seq-opcodes` (XML opcode-table loader), `seq-replay`
(`.vpk` reader), and `seq-cli` (standalone Rust binary that reads pcap or
`.vpk` and emits `.pbstream`). Out of scope for Stage A.

## Build

```sh
cargo build         # builds both crates
cargo test          # runs seq-decode unit tests
```

## License

Licensed under either of

- Apache License, Version 2.0
- MIT License

at your option. (LICENSE files to be added before any external
distribution.)
