#!/usr/bin/env python3
"""Generate seq-eqstructs/src/bindings.rs from showeq-daemon/src/everquest.h.

Replaces bindgen for the explicit allowlist of EQ wire structs. Parses each
struct's `/*OFFSET*/` field comments to derive layout, emits #[repr(C, packed)]
Rust mirrors with original C field names, and emits size_of layout tests.

Run manually after editing everquest.h:
  python3 tools/gen_eqstructs.py [path/to/everquest.h]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Allowlist mirrors the previous bindgen .allowlist_type() set.
ALLOWLIST = [
    "spawnPositionUpdate",
    "deleteSpawnStruct",
    "removeSpawnStruct",
    "hpNpcUpdateStruct",
    "mobHealthStruct",
    "spawnAppearanceStruct",
    "expUpdateStruct",
    "levelUpUpdateStruct",
    "skillIncStruct",
    "manaDecrementStruct",
    "staminaStruct",
    "endUpdateStruct",
    "considerStruct",
    "spawnRenameStruct",
    "clientTargetStruct",
    "newCorpseStruct",
    "remDropStruct",
    "spawnIllusionStruct",
    "buffStruct",
    "action2Struct",
    # Stage A+6 — second small-fixed POD batch.
    "SpawnUpdateStruct",
    "zoneChangeStruct",
    "dzInfo",
    "dzSwitchInfo",
    "startCastStruct",
    "actionStruct",
    "actionAltStruct",
    "groupDisbandStruct",
    "corpseLocStruct",
    # Stage A+7 — variable-length / array opcodes.
    "doorStruct",
    "zonePointStruct",
]

CTYPE_TO_RUST = {
    "uint8_t":  ("u8",  1),
    "int8_t":   ("i8",  1),
    "uint16_t": ("u16", 2),
    "int16_t":  ("i16", 2),
    "uint32_t": ("u32", 4),
    "int32_t":  ("i32", 4),
    "uint64_t": ("u64", 8),
    "int64_t":  ("i64", 8),
    "char":     ("u8",  1),
    "unsigned": ("u32", 4),
    "signed":   ("i32", 4),
    "float":    ("f32", 4),
    "double":   ("f64", 8),
}

# Rust reserved words that may appear as C field names. Bindgen suffixes with `_`.
RUST_KEYWORDS = {
    "type", "match", "self", "super", "crate", "mod", "use", "fn", "let",
    "if", "else", "loop", "while", "for", "return", "break", "continue",
    "struct", "enum", "trait", "impl", "pub", "const", "static", "ref",
    "move", "in", "as", "where", "async", "await", "dyn", "box", "yield",
    "unsafe", "extern", "true", "false",
}

OFFSET_RE = re.compile(r"^\s*/\*\s*(\d+)\s*\*/\s*(.*)")


def find_struct(text: str, name: str) -> str | None:
    """Return the body between the opening { and matching } for `struct name`."""
    pattern = re.compile(r"^struct\s+" + re.escape(name) + r"\s*\{", re.MULTILINE)
    m = pattern.search(text)
    if not m:
        return None
    pos = m.end()
    depth = 1
    while pos < len(text) and depth > 0:
        ch = text[pos]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[m.end():pos]
        pos += 1
    return None


def safe_name(name: str) -> str:
    return name + "_" if name in RUST_KEYWORDS else name


def parse_field_decl(decl: str) -> list[tuple[str, str, str, int]]:
    """Parse a single field-declaration body (no /*offset*/, no trailing ;).

    Returns list of (rust_type, rust_name, c_repr, byte_size) tuples — supports
    comma-separated bitfield lists (`y:19, z:19, u3:7, x:19`) and simple `type name[N]`.
    """
    decl = decl.strip()
    if not decl:
        return []
    decl = re.sub(r"//.*$", "", decl).strip()
    decl = re.sub(r"/\*.*?\*/", "", decl).strip()
    if not decl:
        return []

    m = re.match(r"((?:unsigned\s+|signed\s+)?[A-Za-z_][A-Za-z0-9_]*)\s+(.+)", decl)
    if not m:
        return []

    base_c = m.group(1).strip()
    rest = m.group(2).strip()

    out: list[tuple[str, str, str, int]] = []
    for part in [p.strip() for p in rest.split(",") if p.strip()]:
        part = part.rstrip(";").strip()
        arr = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*\[\s*(\d+)\s*\]\s*$", part)
        if arr:
            field_name, count = arr.group(1), int(arr.group(2))
            mapping = CTYPE_TO_RUST.get(base_c)
            if mapping is None:
                raise ValueError(f"unknown C type: {base_c!r} in decl {decl!r}")
            base_rust, base_size = mapping
            rust_type = f"[{base_rust}; {count}]"
            out.append((rust_type, safe_name(field_name),
                        f"{base_c} {field_name}[{count}]", base_size * count))
            continue
        bf = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(\d+)\s*$", part)
        if bf:
            out.append(("__BITFIELD__", bf.group(1),
                        f"{base_c} {bf.group(1)}:{bf.group(2)}", 0))
            continue
        plain = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*$", part)
        if plain:
            mapping = CTYPE_TO_RUST.get(base_c)
            if mapping is None:
                raise ValueError(f"unknown C type: {base_c!r} in decl {decl!r}")
            base_rust, base_size = mapping
            out.append((base_rust, safe_name(plain.group(1)),
                        f"{base_c} {plain.group(1)}", base_size))
            continue
        raise ValueError(f"could not parse field part: {part!r} in decl {decl!r}")
    return out


def parse_struct_body(name: str, body: str) -> tuple[list[tuple[str, str, str]], int]:
    """Return (fields, total_size).

    Fields are (rust_type, rust_name, comment). For bitfield packs we emit a
    single opaque [u8; N] field; size derives from the trailing /*OFFSET*/.
    """
    if name == "spawnPositionUpdate":
        return parse_spawn_position_update(body)

    fields: list[tuple[str, str, str]] = []
    total_size = 0

    offset_lines: list[tuple[int, str]] = []  # (offset, payload)
    trailing_marker: int | None = None
    for line in body.splitlines():
        m = OFFSET_RE.match(line)
        if not m:
            continue
        offset = int(m.group(1))
        payload = m.group(2).strip()
        if payload:
            offset_lines.append((offset, payload))
        else:
            trailing_marker = offset

    for i, (offset, payload) in enumerate(offset_lines):
        decl_text = payload.rstrip(";").strip()
        decl_text = re.sub(r"//.*$", "", decl_text).strip()
        decl_text = re.sub(r"/\*.*?\*/", "", decl_text).strip()
        parsed = parse_field_decl(decl_text)
        if not parsed:
            raise ValueError(f"{name}: empty parse for line {decl_text!r}")
        if len(parsed) != 1:
            raise ValueError(f"{name}: unexpected multi-field decl {decl_text!r}")
        rust_type, rust_name, comment, byte_size = parsed[0]

        # Cross-check against the next offset if available — catches header drift early.
        if i + 1 < len(offset_lines):
            decl_extent = offset_lines[i + 1][0] - offset
            if decl_extent != byte_size:
                raise ValueError(
                    f"{name}: field {rust_name} declared size={byte_size} but "
                    f"adjacent offsets imply {decl_extent}"
                )

        fields.append((rust_type, rust_name, comment))
        if offset != total_size:
            raise ValueError(
                f"{name}: field {rust_name} at /*{offset}*/ but running total is {total_size}"
            )
        total_size += byte_size

    # Trailing marker is informational only — warn if it disagrees with the field-sum.
    # (e.g. spawnIllusionStruct's /*0336*/ is wrong; actual size is 332.)
    if trailing_marker is not None and trailing_marker != total_size:
        print(
            f"warning: struct {name}: trailing /*{trailing_marker}*/ disagrees with "
            f"field-sum size {total_size}; trusting the field declarations.",
            file=sys.stderr,
        )

    return fields, total_size


def parse_spawn_position_update(body: str) -> tuple[list[tuple[str, str, str]], int]:
    """Special-case: this is the only allowlisted struct with bitfield packing."""
    # Layout from /*offset*/ markers in everquest.h:
    #   /*0000*/ int16_t  spawnId;        (2 bytes)
    #   /*0002*/ uint8_t  unk1[2];        (2 bytes)
    #   /*0004*/ int64_t  y:19, z:19, u3:7, x:19;  + heading:12 + unused2:4
    #   /*0014*/                          (total = 14 bytes; bitfield pack = 10 bytes)
    fields = [
        ("i16",       "spawnId", "int16_t spawnId"),
        ("[u8; 2]",   "unk1",    "uint8_t unk1[2]"),
        ("[u8; 10]",  "_bits",   "packed bitfield: y:19 z:19 u3:7 x:19 heading:12 unused2:4"),
    ]
    return fields, 14


def emit_rust(structs: list[tuple[str, list[tuple[str, str, str]], int]]) -> str:
    out = []
    out.append("// @generated by tools/gen_eqstructs.py — DO NOT EDIT.")
    out.append("// Regenerate with: python3 tools/gen_eqstructs.py [everquest.h]")
    out.append("// Lints (#![allow(non_camel_case_types)] etc.) are applied at the crate")
    out.append("// root in lib.rs since this file is included via include!() and cannot")
    out.append("// carry inner attributes itself.")
    out.append("")

    for name, fields, size in structs:
        out.append(f"#[repr(C, packed)]")
        out.append(f"#[derive(Copy, Clone)]")
        out.append(f"pub struct {name} {{")
        for rust_type, field_name, comment in fields:
            out.append(f"    /// {comment}")
            out.append(f"    pub {field_name}: {rust_type},")
        out.append("}")
        out.append("")
        # Default impl (Copy/Clone derive doesn't auto-impl Default for arrays > 32)
        out.append(f"impl Default for {name} {{")
        out.append("    fn default() -> Self {")
        out.append("        // SAFETY: all fields are POD and zero-bit-pattern is valid.")
        out.append(f"        unsafe {{ core::mem::zeroed() }}")
        out.append("    }")
        out.append("}")
        out.append("")

    # Special accessors for spawnPositionUpdate bitfields.
    out.append("impl spawnPositionUpdate {")
    out.append("    #[inline]")
    out.append("    fn pack(&self) -> [u8; 10] {")
    out.append("        // Copy out the packed bytes via addr_of! — the field is on a #[repr(C, packed)]")
    out.append("        // struct and may be unaligned.")
    out.append("        unsafe { core::ptr::addr_of!(self._bits).read_unaligned() }")
    out.append("    }")
    out.append("    #[inline]")
    out.append("    fn lo64(&self) -> u64 {")
    out.append("        let p = self.pack();")
    out.append("        u64::from_le_bytes([p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]])")
    out.append("    }")
    out.append("    #[inline]")
    out.append("    fn hi16(&self) -> u16 {")
    out.append("        let p = self.pack();")
    out.append("        u16::from_le_bytes([p[8], p[9]])")
    out.append("    }")
    out.append("    /// Low 19 bits of the packed int64 (offset 4) — Y coordinate.")
    out.append("    #[inline]")
    out.append("    pub fn y(&self) -> u64 { self.lo64() & ((1 << 19) - 1) }")
    out.append("    /// Bits 19..38 of the packed int64 — Z coordinate.")
    out.append("    #[inline]")
    out.append("    pub fn z(&self) -> u64 { (self.lo64() >> 19) & ((1 << 19) - 1) }")
    out.append("    /// Bits 38..45 of the packed int64 — unknown 7-bit field.")
    out.append("    #[inline]")
    out.append("    pub fn u3(&self) -> u64 { (self.lo64() >> 38) & ((1 << 7) - 1) }")
    out.append("    /// Bits 45..64 of the packed int64 — X coordinate.")
    out.append("    #[inline]")
    out.append("    pub fn x(&self) -> u64 { (self.lo64() >> 45) & ((1 << 19) - 1) }")
    out.append("    /// Low 12 bits of the trailing u16 — heading.")
    out.append("    #[inline]")
    out.append("    pub fn heading(&self) -> u64 { (self.hi16() & 0xFFF) as u64 }")
    out.append("    /// High 4 bits of the trailing u16 — signed 4-bit unused field.")
    out.append("    #[inline]")
    out.append("    pub fn unused2(&self) -> i64 {")
    out.append("        let v = ((self.hi16() >> 12) & 0xF) as i64;")
    out.append("        if v & 0x8 != 0 { v - 0x10 } else { v }")
    out.append("    }")
    out.append("}")
    out.append("")

    # Layout tests.
    out.append("#[cfg(test)]")
    out.append("mod __layout_tests {")
    out.append("    use super::*;")
    for name, _fields, size in structs:
        out.append(f"    #[test] fn {name}_size() {{ assert_eq!(core::mem::size_of::<{name}>(), {size}); }}")
    out.append("}")
    out.append("")
    return "\n".join(out)


def main(argv: list[str]) -> int:
    here = Path(__file__).resolve().parent.parent
    default_header = here.parent.parent / "showeq-daemon" / "src" / "everquest.h"
    header_path = Path(argv[1]) if len(argv) > 1 else default_header
    if not header_path.exists():
        print(f"error: {header_path} not found", file=sys.stderr)
        return 1
    text = header_path.read_text()

    structs = []
    for name in ALLOWLIST:
        body = find_struct(text, name)
        if body is None:
            print(f"error: struct {name} not found in {header_path}", file=sys.stderr)
            return 1
        try:
            fields, size = parse_struct_body(name, body)
        except ValueError as e:
            print(f"error: parsing struct {name}: {e}", file=sys.stderr)
            return 1
        structs.append((name, fields, size))

    out_path = here / "seq-eqstructs" / "src" / "bindings.rs"
    out_path.write_text(emit_rust(structs))
    print(f"wrote {out_path} ({len(structs)} structs)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
