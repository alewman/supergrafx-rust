#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Which case does each named switch, and each position of the back-panel switch, change?

The lockstep gate proves a switch is wired only on a case where the switch changes what the
machine does: if the reference's trace with the switch is the trace without it, the two sides
would agree whether or not the Rust side reads the switch at all (the base Rust board's
``scripts/switch_effect.py``, whose shape this is). For each case below the reference's trace
(in the reference cache a gate run left) is compared with the trace of the same card without
the switch; the first frame that differs is printed, with what differs in it (the bus log, the
picture, the audio, the CPU state). Fails if a switch has no case it changes.

    sgx_switch_effect.py [--reference-cache DIR]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import sgx_lockstep as gate  # noqa: E402

# case: (the same card without the switch, the switch)
PAIRS = {
    "dmm-stage1-sgxtech": ("dmm-stage1", "PRIORITY_LABELS"),
    "sw-labels-probe": ("probe", "PRIORITY_LABELS"),
    "sw-origin-probe": ("probe", "WINDOW_ORIGIN"),
    "sw-off-dmm": ("dmm-stage1", "WINDOW_OFF_AT_OR_BELOW"),
    "sw-off-probe": ("probe", "WINDOW_OFF_AT_OR_BELOW"),
    "sw-steers-probe": ("sw-compat-at-probe", "ST_STEERS_IN_COMPAT"),
    "sw-compat-at-probe": ("probe", "the back-panel switch, moved"),
    "sw-compat-probe": ("probe", "the back-panel switch"),
    "dmm-compat": ("lib-Daimakaimura (Japan) (En)", "the back-panel switch"),
    "sw-seed-probe": ("probe", "power_on_seed"),
    "sw-base-probe": ("probe", "the fallback (one VDC, no VPC)"),
    "gate-dmm-base": ("lib-Daimakaimura (Japan) (En)", "the fallback (one VDC, no VPC)"),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--reference-cache", type=Path, default=ROOT / "external" / "refcache")
    args = parser.parse_args()
    cases = {case[0]: case for case in gate.cases(HERE / "cases.tsv")}

    def lines(name: str) -> list[str] | None:
        path = args.reference_cache / gate.cache_name(cases[name])
        return path.read_text().splitlines() if path.is_file() else None

    live: dict[str, int] = {}
    missing = 0
    for name, (base, switch) in PAIRS.items():
        live.setdefault(switch, 0)
        ours, theirs = lines(name), lines(base)
        if ours is None or theirs is None:
            print(f"{name}: no reference trace (run the gate with --reference-cache first)")
            missing += 1
            continue
        effect = "no effect"
        for number, (a, b) in enumerate(zip(ours, theirs)):
            if a != b:
                x, y = gate.fields(a), gate.fields(b)
                what = [key for key in x if x[key] != y.get(key)]
                effect = f"differs from {base} at line {number} in {', '.join(what) or 'the frame counter'}"
                live[switch] += 1
                break
        print(f"{name} [{switch}]: {effect}")
    dead = [switch for switch, count in live.items() if not count]
    for switch in dead:
        print(f"  ** {switch}: no case on which it changes anything")
    print(f"{len(live) - len(dead)} of {len(live)} switches change at least one of their cases")
    return 1 if dead or missing else 0


if __name__ == "__main__":
    sys.exit(main())
