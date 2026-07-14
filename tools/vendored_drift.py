#!/usr/bin/env python3
"""Report drift between seq-backend-eql's vendored parser modules and their
seq-decode twins.

seq-backend-eql deliberately vendors copies of the shared Live parsers so a
Live wire patch can't silently reach the eql decode path. The cost of that
isolation is invisible drift: a bug fix landing in a shared seq-decode parser
never reaches eql's copy, and nothing says so. This script makes the drift
visible so porting (or not) is a conscious decision.

Output per common module: `identical` or `DIVERGED (+a/-b lines)`.
Modules that exist in only one crate are listed separately (eql-only decoders
like stat_sync/ucs_chat, or crate-internal files like bindings/eqstructs).

Usage:
  tools/vendored_drift.py            # human-readable report
  tools/vendored_drift.py --quiet    # only diverged modules (for CI logs)

Always exits 0 — divergence is allowed; this is a report, not a gate.
"""

import argparse
import difflib
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SHARED = REPO / "seq-decode" / "src"
VENDORED = REPO / "seq-backend-eql" / "src"

# Crate-internal files that are not vendored parser modules.
NOT_PARSERS = {"lib.rs", "bindings.rs", "eqstructs.rs"}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--quiet", action="store_true",
                    help="print only diverged modules")
    args = ap.parse_args()

    shared = {p.name for p in SHARED.glob("*.rs")} - NOT_PARSERS
    vendored = {p.name for p in VENDORED.glob("*.rs")} - NOT_PARSERS

    common = sorted(shared & vendored)
    diverged = []

    for name in common:
        a = (SHARED / name).read_text().splitlines()
        b = (VENDORED / name).read_text().splitlines()
        if a == b:
            if not args.quiet:
                print(f"  identical  {name}")
            continue
        plus = minus = 0
        for line in difflib.unified_diff(a, b, lineterm=""):
            if line.startswith("+") and not line.startswith("+++"):
                plus += 1
            elif line.startswith("-") and not line.startswith("---"):
                minus += 1
        diverged.append(name)
        print(f"  DIVERGED   {name} (+{plus}/-{minus} lines vs seq-decode)")

    if not args.quiet:
        for name in sorted(shared - vendored):
            print(f"  live-only  {name}")
        for name in sorted(vendored - shared):
            print(f"  eql-only   {name}")

    print(f"\n{len(common)} common modules: "
          f"{len(common) - len(diverged)} identical, {len(diverged)} diverged; "
          f"{len(shared - vendored)} live-only, {len(vendored - shared)} eql-only")
    if diverged:
        print("diverged modules are ALLOWED (that's the point of vendoring) — "
              "but check whether a seq-decode fix should be ported.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
