//! Pure parsers for ShowEQ packet payloads.
//!
//! No I/O, no global state, no Qt. Each module exposes a `parse_*`
//! function that turns a `&[u8]` payload into a typed struct (or a
//! `ParseError`). Higher layers (FFI bridge, replay tools, the
//! eventual standalone daemon) compose these.
//!
//! Stage A scope: `OP_MobUpdate` only. More opcodes land in Stage A+1.
