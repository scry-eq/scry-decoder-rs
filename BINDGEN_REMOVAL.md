# Bindgen Removal + TOML Opcode Migration

Branch: `feat/eq-codegen` (worktrees: `worktrees/decoder-rs-codegen`, `worktrees/daemon-codegen`)

This document tracks the replacement of `bindgen` in `seq-eqstructs` with a custom
Python codegen script, and migration of the XML opcode files to TOML.

---

## Part 1: Custom struct codegen (replaces bindgen)

### Why

- `libclang-dev` is a heavy, version-sensitive build dependency for what is essentially
  a list of 20 flat structs
- Bindgen's bitfield API returns `u64` and requires manual `sign_extend()` wrappers
- Generated `bindings.rs` lives in `OUT_DIR` (opaque, not diffable)
- We maintain an explicit allowlist anyway — the "automatic" part of bindgen doesn't help us

### Script: `tools/gen_eqstructs.py`

Reads `everquest.h`, parses the allowlisted structs using the `/*offset*/` comment
annotations, and writes a committable `seq-eqstructs/src/bindings.rs`.

**Parsing strategy:** Each field line has the form:
```
/*HHHH*/ ctype  name[N];      // optional comment
```
Fields without offset comments (continuation lines of bitfield packs) are attached
to the preceding annotated line. The `/*HHHH*/` at end-of-struct gives total size —
used to generate the `size_of` layout test.

**Bitfield handling:** For `spawnPositionUpdate` the packed `int64_t y:19, z:19, u3:7, x:19; unsigned heading:12` is represented as a `[u8; 12]` opaque byte blob with hand-written accessor methods (matches current bindgen output semantics, but in readable committed code).

### Allowlisted structs (20 total)

All sizes verified by existing layout tests in `seq-eqstructs/src/lib.rs`.

```
struct spawnPositionUpdate       14 bytes   HAS BITFIELDS — special-case
struct deleteSpawnStruct          4 bytes
struct removeSpawnStruct          5 bytes
struct hpNpcUpdateStruct         18 bytes
struct mobHealthStruct            6 bytes
struct spawnAppearanceStruct      8 bytes
struct expUpdateStruct           16 bytes
struct levelUpUpdateStruct       16 bytes
struct skillIncStruct            12 bytes
struct manaDecrementStruct       20 bytes
struct staminaStruct              8 bytes
struct endUpdateStruct           10 bytes
struct considerStruct            28 bytes
struct spawnRenameStruct        195 bytes
struct clientTargetStruct         4 bytes
struct newCorpseStruct           40 bytes
struct remDropStruct             12 bytes
struct spawnIllusionStruct      332 bytes
struct buffStruct               168 bytes
struct action2Struct             48 bytes
```

### Extracted struct definitions from everquest.h (daemon-codegen/src/everquest.h)

```c
struct spawnPositionUpdate
{
/*0000*/ int16_t  spawnId;
/*0002*/ uint8_t unk1[2];
/*0004*/ int64_t  y:19, z:19, u3:7,x:19;
         unsigned heading:12;
         signed unused2:4;
/*0014*/
};

struct deleteSpawnStruct
{
/*0000*/ uint32_t spawnId;
/*0004*/
};

struct removeSpawnStruct
{
/*0000*/ uint32_t spawnId;
/*0004*/ uint8_t  removeSpawn;
/*0005*/
};

struct hpNpcUpdateStruct
{
/*0000*/ uint16_t spawnId;
/*0002*/ int32_t curHP;
/*0006*/ uint32_t unknown0006;
/*0010*/ int32_t maxHP;
/*0014*/ uint32_t unknown0014;
/*0018*/
};

struct mobHealthStruct
{
/*0000*/ uint16_t spawnId;
/*0002*/ int32_t  hpPercent;
/*0006*/
};

struct spawnAppearanceStruct
{
/*0000*/ uint16_t spawnId;
/*0002*/ uint16_t type;
/*0004*/ uint32_t parameter;
/*0008*/
};

struct expUpdateStruct
{
/*0000*/ uint32_t exp;
/*0004*/ uint32_t unknown0004;
/*0008*/ uint32_t type;
/*0012*/ uint32_t unknown0012;
/*0016*/
};

struct levelUpUpdateStruct
{
/*0000*/ uint32_t level;
/*0004*/ uint32_t levelOld;
/*0008*/ uint32_t exp;
/*0012*/ uint32_t unknown0012;
/*0016*/
};

struct skillIncStruct
{
/*0000*/ uint32_t skillId;
/*0004*/ int32_t  value;
/*0008*/ uint8_t  unknown0008[4];
/*0012*/
};

struct manaDecrementStruct
{
/*0000*/ int32_t newMana;
/*0004*/ int32_t maxMana;
/*0008*/ int32_t spellId;
/*0012*/ uint8_t unknown0012[4];
/*0016*/ uint8_t unknown0016[4];
/*0020*/
};

struct staminaStruct
{
/*0000*/ uint32_t food;
/*0004*/ uint32_t water;
/*0008*/
};

struct endUpdateStruct
{
/*0000*/ uint16_t spawn_id;
/*0002*/ uint32_t cur;
/*0006*/ uint32_t max;
/*0010*/
};

struct considerStruct
{
/*0000*/ uint32_t playerid;
/*0004*/ uint32_t targetid;
/*0008*/ int32_t  faction;
/*0012*/ int32_t  level;
/*0016*/ int32_t  unknown0016;
/*0020*/ int32_t  unknown0020;
/*0024*/ int32_t  unknown0024;
/*0028*/
};

struct spawnRenameStruct
{
/*000*/ char     old_name[64];
/*064*/ char     old_name_again[64];
/*128*/ char     new_name[64];
/*192*/ uint8_t  unknown0192[3];
/*195*/
};

struct clientTargetStruct
{
/*0000*/ uint32_t newTarget;
/*0004*/
};

struct newCorpseStruct
{
/*0000*/ uint32_t spawnId;
/*0004*/ uint32_t killerId;
/*0008*/ uint32_t corpseid;
/*0012*/ int32_t  type;
/*0016*/ uint32_t spellId;
/*0020*/ uint16_t zoneId;
/*0022*/ uint16_t zoneInstance;
/*0024*/ uint32_t damage;
/*0028*/ uint8_t  unknown0028[12];
/*0040*/
};

struct remDropStruct
{
/*0000*/ uint16_t dropId;
/*0002*/ uint8_t  unknown0002[2];
/*0004*/ uint16_t spawnId;
/*0006*/ uint8_t  unknown0006[2];
/*0008*/ uint8_t  unknown0008[4];
/*0012*/
};

struct spawnIllusionStruct
{
/*0000*/ uint32_t spawnId;
/*0004*/ char     name[64];
/*0068*/ uint32_t race;
/*0072*/ uint8_t  gender;
/*0073*/ uint8_t  texture;
/*0074*/ uint8_t  helm;
/*0075*/ uint8_t  unknown0075;
/*0076*/ uint32_t unknown0076;
/*0080*/ uint32_t face;
/*0084*/ uint8_t  unknown0084[248];
/*0336*/
};

struct buffStruct
{
/*0000*/ uint32_t spawnid;
/*0004*/ uint8_t  unknown0004[112];
/*0116*/ uint32_t spellid;
/*0120*/ uint32_t duration;
/*0124*/ int32_t  unknown0024;
/*0128*/ uint8_t  unknown0080[25];
/*0153*/ int8_t   level;
/*0154*/ uint8_t  unknown0106[6];
/*0160*/ uint32_t spellslot;
/*0164*/ uint32_t changetype;
/*0168*/
};

struct action2Struct
{
/*0000*/ uint16_t target;
/*0002*/ uint16_t source;
/*0004*/ uint8_t  unknown0004[4];
/*0008*/ int32_t  damage;
/*0012*/ int8_t   unknown0012[8];
/*0020*/ int32_t  spell;
/*0024*/ uint8_t  uknown0024[16];
/*0040*/ uint8_t  type;
/*0041*/ uint8_t  unknown0042[7];
/*0048*/
};
```

### C type → Rust type mapping

| C type | Rust type |
|--------|-----------|
| `uint8_t` | `u8` |
| `int8_t` | `i8` |
| `uint16_t` | `u16` |
| `int16_t` | `i16` |
| `uint32_t` | `u32` |
| `int32_t` | `i32` |
| `uint64_t` | `u64` |
| `int64_t` | `i64` |
| `char[N]` | `[u8; N]` |
| `type[N]` | `[type; N]` |
| bitfield pack | `[u8; N]` + hand-written accessors |

### Generated bindings.rs structure

```rust
// @generated by tools/gen_eqstructs.py — DO NOT EDIT
// Regenerate: python3 tools/gen_eqstructs.py ../showeq-daemon/src/everquest.h
#![allow(non_camel_case_types, non_snake_case, dead_code)]

#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
pub struct deleteSpawnStruct {
    pub spawnId: u32,
}

// ... one block per struct ...

#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test] fn deleteSpawnStruct_size()       { assert_eq!(std::mem::size_of::<deleteSpawnStruct>(), 4); }
    // ... one assertion per struct ...
}
```

### Migration steps (seq-eqstructs)

1. Write `tools/gen_eqstructs.py` (parse + emit)
2. Run it; verify all 20 layout tests pass (`cargo test -p seq-eqstructs`)
3. Remove `build.rs`
4. Remove `wrapper.h`
5. Update `seq-eqstructs/src/lib.rs`: replace `include!(concat!(env!("OUT_DIR"), "/bindings.rs"))` with `include!("bindings.rs")`
6. Remove `bindgen` from `[workspace.dependencies]` in root `Cargo.toml`
7. Update `seq-eqstructs/Cargo.toml`: remove `build = "build.rs"` and the `bindgen` dep
8. Add `gen_eqstructs.py` invocation note to the crate CLAUDE.md

---

## Part 2: TOML opcode format (replaces XML)

### Why

- TOML diffs cleanly (no angle brackets)
- `#` comments are first-class
- Native to the Rust ecosystem
- Single canonical source; XML regenerated from it for C++ daemon

### Tool: `tools/toml_to_xml.py`

~80 lines, Python stdlib only (`tomllib` + `xml.etree.ElementTree`).

- Reads `conf/opcodes.toml`
- Writes `conf/zoneopcodes.xml` and `conf/worldopcodes.xml`
- CMake `add_custom_command` in `daemon-codegen/CMakeLists.txt` runs it as a PRE_BUILD dep

### TOML schema

```toml
# conf/opcodes.toml
# Zone-server opcodes
[[zone]]
id      = "4a4f"
name    = "OP_MobUpdate"
updated = "2026-05-04"
comment = "MobUpdateCode"

  [[zone.payloads]]
  dir           = "both"
  typename      = "spawnPositionUpdate"
  sizechecktype = "match"

[[zone]]
id      = "ffff"
name    = "OP_BeginCast"
comment = "BeginCastCode"

  [[zone.payloads]]
  dir           = "both"
  typename      = "beginCastStruct"
  sizechecktype = "match"

# World-server opcodes
[[world]]
id      = "022f"
name    = "OP_GuildList"
updated = "2026-05-04"
comment = "old GuildListCode"

  [[world.payloads]]
  dir           = "server"
  typename      = "worldGuildListStruct"
  sizechecktype = "none"
```

Rules:
- `id = "ffff"` means opcode not yet mapped (kept for named log entries)
- `updated` omitted for unmapped opcodes
- Multiple `[[zone.payloads]]` entries under one `[[zone]]` for multi-direction opcodes

### `toml_to_xml.py` structure

```python
import tomllib, sys
from xml.etree.ElementTree import Element, SubElement, tostring, indent

def emit_xml(opcodes, outpath):
    root = Element("seqopcodes")
    for op in opcodes:
        oc = SubElement(root, "opcode",
            id=op["id"], name=op["name"],
            **({"updated": op["updated"]} if "updated" in op else {}))
        if "comment" in op:
            c = SubElement(oc, "comment"); c.text = op["comment"]
        for p in op.get("payloads", []):
            SubElement(oc, "payload",
                dir=p["dir"], typename=p["typename"],
                sizechecktype=p["sizechecktype"])
    indent(root, space="    ")
    ...
```

### CMake integration (daemon-codegen)

```cmake
find_package(Python3 REQUIRED COMPONENTS Interpreter)

set(OPCODES_TOML "${CMAKE_CURRENT_SOURCE_DIR}/conf/opcodes.toml")
set(ZONE_XML     "${CMAKE_CURRENT_SOURCE_DIR}/conf/zoneopcodes.xml")
set(WORLD_XML    "${CMAKE_CURRENT_SOURCE_DIR}/conf/worldopcodes.xml")

add_custom_command(
    OUTPUT  ${ZONE_XML} ${WORLD_XML}
    COMMAND ${Python3_EXECUTABLE}
            "${CMAKE_CURRENT_SOURCE_DIR}/tools/toml_to_xml.py"
            "${OPCODES_TOML}"
            "${ZONE_XML}" "${WORLD_XML}"
    DEPENDS ${OPCODES_TOML}
    COMMENT "Regenerating opcode XML from opcodes.toml"
)
add_custom_target(opcode_xml ALL DEPENDS ${ZONE_XML} ${WORLD_XML})
add_dependencies(seq-daemon-core opcode_xml)
```

### Migration steps (opcode XML)

1. Write `tools/toml_to_xml.py`
2. Convert `zoneopcodes.xml` + `worldopcodes.xml` to `conf/opcodes.toml` (one-time script or manual)
3. Run `toml_to_xml.py`; diff output against current XML (should be structurally identical)
4. Add `add_custom_command` to `CMakeLists.txt`
5. Gitignore `conf/zoneopcodes.xml` and `conf/worldopcodes.xml` (generated)
6. Delete `scripts/update_zoneopcodes.py` (retired; edit `opcodes.toml` directly)
7. Update daemon CLAUDE.md

---

## Files to create / modify

### decoder-rs worktree (`worktrees/decoder-rs-codegen/`)

| File | Action |
|------|--------|
| `tools/gen_eqstructs.py` | CREATE |
| `seq-eqstructs/src/bindings.rs` | CREATE (generated output, committed) |
| `seq-eqstructs/src/lib.rs` | EDIT: `include!("bindings.rs")` instead of OUT_DIR |
| `seq-eqstructs/build.rs` | DELETE |
| `seq-eqstructs/wrapper.h` | DELETE |
| `seq-eqstructs/Cargo.toml` | EDIT: remove `build`, remove `bindgen` dep |
| `Cargo.toml` (workspace) | EDIT: remove `bindgen` from `[workspace.dependencies]` |

### daemon worktree (`worktrees/daemon-codegen/`)

| File | Action |
|------|--------|
| `tools/toml_to_xml.py` | CREATE |
| `conf/opcodes.toml` | CREATE (canonical source) |
| `conf/zoneopcodes.xml` | GITIGNORE (generated) |
| `conf/worldopcodes.xml` | GITIGNORE (generated) |
| `CMakeLists.txt` | EDIT: add `add_custom_command` |
| `scripts/update_zoneopcodes.py` | DELETE |

---

## Verification

```bash
# Struct layout tests
cargo test -p seq-eqstructs --manifest-path worktrees/decoder-rs-codegen/Cargo.toml

# Full decoder build
cargo build --workspace --manifest-path worktrees/decoder-rs-codegen/Cargo.toml

# Daemon build with Rust decoder
(cd worktrees/daemon-codegen && cmake -B build -DSEQ_USE_RUST=ON && cmake --build build)

# Replay sanity check (if goldens available)
./build/showeq-daemon --replay tests/replay/<fixture>.vpk \
    --config-dir conf --no-listen \
    --rust-opcodes OP_MobUpdate \
    --record-golden /tmp/verify.pbstream
```
