#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""A gate that cannot fail proves nothing: build the crate with one fault at a time and check
that some rung catches each (the base Rust board's ``scripts/seeded_faults.py``, whose shape
this is).

Each fault is one exact text replacement in a copy of ``src/`` (with ``tests/``), built into a
shared target directory under ``external/faults/``. Every fault is then put to both cheap
checks, so the table says which of them see it:

    rung 1      cargo test (the transcribed unit tests)
    the gate    lockstep on the short cases: rung 2 (Covell's test, the probe card), the probe's
                switch cases of rung 3, the Covell cases of rung 4

and, only if neither catches it, to the long ones: the library cards and Daimakaimura's first
stage in lockstep (rung 3), then the save-state round trip on the cards (rung 5).

The reference's traces come from the cache a full gate run left (``--cache``), so a fault costs
a build and the Rust side of the cases. A fault nothing catches is reported as MISSED.

    sgx_seeded_faults.py [--cache DIR] [--only NAME] [--jobs N] [--list] [--known-failing REGEX]
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# name: (file, exact text, replacement, what it changes)
FAULTS = {
    # vpc.rs: the registers
    "window-msb-three-bits": (
        "vpc.rs",
        "WINDOW1_MSB => self.window[0] = (self.window[0] & 0x0FF) | ((value as u16 & 3) << 8),",
        "WINDOW1_MSB => self.window[0] = (self.window[0] & 0x0FF) | ((value as u16 & 7) << 8),",
        "window 1's MSB register keeps three bits (it reads back $04-$07)",
    ),
    "steer-register-reads-back": (
        "vpc.rs",
        "            _ => 0x00,\n",
        "            _ => self.st_to_vdc2 as u8,\n",
        "$0E and $0F read the steering bit, not $00",
    ),
    "steer-on-any-bit": (
        "vpc.rs",
        "self.st_to_vdc2 = value & 1 != 0;",
        "self.st_to_vdc2 = value != 0;",
        "any bit of $0E steers ST0-ST2, not bit 0 alone",
    ),
    # vpc.rs: the windows
    "window-two-alone-takes-the-overlap-nibble": (
        "vpc.rs",
        "(false, true) => a >> 4,",
        "(false, true) => a & 0x0F,",
        "the area inside window 2 alone is mixed by the overlap's group",
    ),
    "no-window-takes-window-ones-nibble": (
        "vpc.rs",
        "(false, false) => b >> 4,",
        "(false, false) => b & 0x0F,",
        "the area outside both windows is mixed by window 1's group",
    ),
    "window-of-0x40-is-on": (
        "vpc.rs",
        "if width as i64 <= self.window_off_at_or_below {",
        "if (width as i64) < self.window_off_at_or_below {",
        "a window of width $40 is on (U-SGX2: at or below $40 is off)",
    ),
    "window-origin-ignores-the-dot-clock": (
        "vpc.rs",
        "dots.min(width as i64 - 0x40 + self.window_origin.of(div))",
        "dots.min(width as i64 - 0x40 + self.window_origin.divider4)",
        "every dot clock takes the 5.37 MHz window origin (U-SGX1)",
    ),
    "window-origin-before-the-repin": (
        "vpc.rs",
        "                divider3: 38,\n",
        "                divider3: 46,\n",
        "the 7.16 MHz window origin measured on the old base board (46, before 3318027)",
    ),
    "second-window-msb-not-replayed": (
        "vpc.rs",
        "self.shadow_window[1] = (self.shadow_window[1] & 0x0FF) | ((value as u16 & 3) << 8)",
        "self.shadow_window[1] = (self.shadow_window[1] & 0x0FF) | ((value as u16 & 1) << 8)",
        "the mix sees one bit of window 2's MSB register",
    ),
    # vpc.rs: the mix
    "rows-swapped": (
        "vpc.rs",
        "let (vdc1, vdc2) = (&rows[0][..], &rows[1][..]);",
        "let (vdc1, vdc2) = (&rows[1][..], &rows[0][..]);",
        "the two VDCs' pixel buses are swapped",
    ),
    "mid-line-write-a-dot-early": (
        "vpc.rs",
        "let stop = dots.min(x.max(-py_floordiv(first - when, div)));",
        "let stop = dots.min(x.max(py_floordiv(when - first, div)));",
        "a register written inside a dot takes effect from that dot, not the next (U-SGX3)",
    ),
    "both-off-shows-vdc1": (
        "vpc.rs",
        "out.resize(out.len() + n, 0);",
        "out.extend_from_slice(a);",
        "an area with both layers off shows VDC 1, not colour 0",
    ),
    "vdc2-alone-shows-vdc1": (
        "vpc.rs",
        "        if cfg & 1 == 0 {\n            out.extend_from_slice(b);",
        "        if cfg & 1 == 0 {\n            out.extend_from_slice(a);",
        "an area with VDC 2 alone enabled shows VDC 1",
    ),
    "transparent-pair-shows-vdc1": (
        "vpc.rs",
        "                // MAME huc6202.cpp (BSD-3-Clause) the same. sgxtech silent.\n                front = q;",
        "                // MAME huc6202.cpp (BSD-3-Clause) the same. sgxtech silent.\n"
        "                if q & COLOUR != 0 {\n                    front = q;\n                }",
        "where both pixels are transparent VDC 1's value shows, not VDC 2's (U-SGX5)",
    ),
    "vdc2-sprites-over-vdc1-sprites": (
        "vpc.rs",
        "if q & COLOUR != 0 && q & SPRITE != 0 && p & SPRITE == 0 {",
        "if q & COLOUR != 0 && q & SPRITE != 0 {",
        "under code 01b VDC 2's sprites come in front of VDC 1's sprites too",
    ),
    "code-10-is-the-default-order": (
        "vpc.rs",
        "} else if code == 2 && p & SPRITE != 0",
        "} else if code == 4 && p & SPRITE != 0",
        "code 10b mixes as the default order (VDC 1 in front wherever it is opaque)",
    ),
    "priority-labels-not-read": (
        "vpc.rs",
        "code ^= 3; // C-SGX1: the document's labels",
        "code ^= 0; // C-SGX1: the document's labels",
        "the PRIORITY_LABELS switch is ignored (C-SGX1)",
    ),
    # vpc.rs: the decode
    # vpc.rs: the base board's wait (pcengine-python issue 0004) through the page
    "vdc2-waiting-write-dropped": (
        "vpc.rs",
        "pins.vdc_write(1, offset & 3, value, mclk);",
        "pins.vdcs[1].write(offset & 3, value, mclk);\n            pins.vdcs[1].serve_wanted = false;",
        "a write to VDC 2 that has to wait never reaches the board's wait and is lost",
    ),
    "vdc1-waiting-read-dropped": (
        "vpc.rs",
        "            pins.vdc_read(0, offset & 3, mclk)\n",
        "            { let v = pins.vdcs[0].read(offset & 3, mclk); pins.vdcs[0].serve_wanted = false; v }\n",
        "a read of VDC 1 that has to wait never reaches the board's wait and answers $FF",
    ),
    "plain-stores-steered": (
        "vpc.rs",
        "if steered && ST_OPCODES.contains(&pins.opcode) {",
        "if steered {",
        "with $0E set every write of the area goes to VDC 2, not only ST0-ST2",
    ),
    "st2-not-steered": (
        "vpc.rs",
        "pub const ST_OPCODES: [u8; 3] = [0x03, 0x13, 0x23];",
        "pub const ST_OPCODES: [u8; 3] = [0x03, 0x13, 0x33];",
        "ST2 is not taken for a store-immediate",
    ),
    "compat-still-steers": (
        "vpc.rs",
        "let steered = self.vpc.st_to_vdc2 && (self.st_steers_in_compat || !self.compat);",
        "let steered = self.vpc.st_to_vdc2;",
        "ST0-ST2 follow a left-over $0E bit in PC Engine position whatever the switch (U-SGX4)",
    ),
    "steers-in-compat-not-read": (
        "vpc.rs",
        "let steered = self.vpc.st_to_vdc2 && (self.st_steers_in_compat || !self.compat);",
        "let steered = self.vpc.st_to_vdc2 && !self.compat;",
        "the ST_STEERS_IN_COMPAT switch is not wired (U-SGX4)",
    ),
    "vdc2-write-mirror-missing": (
        "vpc.rs",
        "        } else if offset < 0x18 {\n            pins.vdc_write(1, offset & 3, value, mclk);",
        "        } else if offset < 0x14 {\n            pins.vdc_write(1, offset & 3, value, mclk);",
        "writes to $14-$17 do not reach VDC 2",
    ),
    "block-not-repeated-for-writes": (
        "vpc.rs",
        "        let offset = address & 0x1F;\n        if offset < 0x08 {\n            pins.vdc_write(0, ",
        "        let offset = address & 0x3FF;\n        if offset < 0x08 {\n            pins.vdc_write(0, ",
        "the 32-byte block takes writes only in its first copy",
    ),
    "gap-reads-zero": (
        "vpc.rs",
        "        } else {\n            0xFF\n        }",
        "        } else {\n            0x00\n        }",
        "$18-$1F read $00",
    ),
    "compat-reads-keep-the-native-decode": (
        "vpc.rs",
        "        if self.compat {\n            return pins.vdc_read(0, address & 3, mclk);",
        "        if self.compat && address & 0x18 == 0 {\n            return pins.vdc_read(0, address & 3, mclk);",
        "in PC Engine position only $00-$07 of the block read VDC 1",
    ),
    # vpc.rs: reset and save states
    "button-reset-resets-the-vpc": (
        "vpc.rs",
        "        if kind == ResetKind::PowerOn {\n            self.vpc.reset();",
        "        if kind == ResetKind::Button {\n            self.vpc.reset();",
        "the VPC is reset by the button and not by power-on",
    ),
    "state-forgets-the-switch-position": (
        "vpc.rs",
        "            compat: self.compat,\n",
        "            compat: false,\n",
        "a state is saved with the back-panel switch in SuperGrafx position",
    ),
    "state-forgets-the-pending-writes": (
        "vpc.rs",
        "        vpc.log = state.log;\n",
        "",
        "a loaded state keeps the VPC writes the system it is loaded into had not drawn yet",
    ),
    "state-loses-the-line-shadow": (
        "vpc.rs",
        "vpc.shadow_window = state.shadow_window;",
        "vpc.shadow_window = state.window;",
        "a loaded state mixes the line in progress with the windows as last written",
    ),
    # machine.rs, board.rs
    "work-ram-8k": (
        "machine.rs",
        "work_ram_bytes: 32 * 1024,",
        "work_ram_bytes: 8 * 1024,",
        "the SuperGrafx row has the PC Engine's 8 KiB, mirrored",
    ),
    "builder-ignores-the-switch": (
        "board.rs",
        "VideoPage::new(options.vpc.unwrap_or_default(), options.compat);",
        "VideoPage::new(options.vpc.unwrap_or_default(), false);",
        "a SuperGrafx built in PC Engine position is in SuperGrafx position",
    ),
    "builder-ignores-the-vpc-given": (
        "board.rs",
        "VideoPage::new(options.vpc.unwrap_or_default(), options.compat);",
        "VideoPage::new(crate::vpc::Vpc::new(), options.compat);",
        "the builder puts a VPC with default switches on the board",
    ),
}

SHORT = "^(covell|probe|sw-.*-probe|gate-covell-.*)$"
LONG = "^(lib-|dmm-|sw-.*-dmm|gate-dmm)"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--cache", type=Path, default=ROOT / "external" / "refcache")
    parser.add_argument("--work", type=Path, default=ROOT / "external" / "faults")
    parser.add_argument("--only")
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--known-failing", metavar="REGEX",
                        help="cases (lockstep and round trip) left out because the unfaulted tree fails them")
    args = parser.parse_args()
    if args.list:
        for name, (file, _, _, what) in FAULTS.items():
            print(f"{name}\t{file}\t{what}")
        return 0
    short, long, states = SHORT, LONG, None
    if args.known_failing:
        skip = f"(?!(?:{args.known_failing})$)"
        short, long, states = f"^{skip}{SHORT[1:]}", f"^{skip}{LONG[1:]}", f"^{skip}"
        print(f"left out, failing unfaulted: {args.known_failing}", flush=True)
    missed = ran = 0
    tree = args.work / "tree"
    target = args.work / "target"
    # None first: the unfaulted tree must pass rung 1 and the short cases, or every fault is
    # "caught" by a check that fails anyway (the tree once lacked the README, which src/lib.rs
    # runs as a doctest, so `cargo test` failed whatever the fault; fixed 2026-11-19, as in
    # pcengine-rust's seeder).
    for name, (file, old, new, _) in [("(unfaulted)", ("vpc.rs", "", "", "")), *FAULTS.items()]:
        if args.only and name not in (args.only, "(unfaulted)"):
            continue
        clean = name == "(unfaulted)"
        ran += not clean
        for part in ("src", "tests"):
            if (tree / part).exists():
                shutil.rmtree(tree / part)
            # shutil.copy, not copytree's copy2: a restored file with its old time looks older than
            # the last fault's build to cargo, which then keeps that fault (fixed 2026-11-19)
            shutil.copytree(ROOT / part, tree / part, copy_function=shutil.copy)
        manifest = (ROOT / "Cargo.toml").read_text().replace(
            'path = "../pcengine-rust"', f'path = "{(ROOT.parent / "pcengine-rust").as_posix()}"'
        )
        (tree / "Cargo.toml").write_text(manifest)
        shutil.copy(ROOT / "Cargo.lock", tree / "Cargo.lock")
        shutil.copy(ROOT / "README.md", tree / "README.md")
        text = (ROOT / "src" / file).read_text()
        if clean:
            pass
        elif text.count(old) != 1:
            print(f"{name}: the fault's text is in {file} {text.count(old)} times, not once")
            return 2
        if not clean:
            (tree / "src" / file).write_text(text.replace(old, new))
        cargo = ["nice", "cargo", "--quiet", "--manifest-path", str(tree / "Cargo.toml")]
        build = subprocess.run(
            [*cargo[:2], "build", "--release", *cargo[2:], "--target-dir", str(target)],
            capture_output=True, text=True,
        )  # fmt: skip
        if build.returncode:
            print(f"{name}: does not build\n{build.stderr[-800:]}")
            return 2
        binaries = target / "release"

        def lockstep(only: str) -> str | None:
            done = subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "sgx_lockstep.py"), "--rust", str(binaries / "sgx-trace"),
                 "--reference-cache", str(args.cache), "--traces", str(args.work / "traces"),
                 "--jobs", str(args.jobs), "--no-narrow", "--quiet", "--only", only],
                capture_output=True, text=True,
            )  # fmt: skip
            if done.returncode == 0:
                return None
            return next((line.split(":")[0] for line in done.stdout.splitlines()
                         if line and not line.startswith(" ") and ": " in line), "?")  # fmt: skip

        caught = []
        tests = subprocess.run(
            [*cargo[:2], "test", "--release", *cargo[2:], "--target-dir", str(target)],
            capture_output=True, text=True,
        )  # fmt: skip
        if tests.returncode:
            # `cargo test --quiet` names a failed test under "failures:", indented
            out = tests.stdout.splitlines()
            failed = [line.strip() for n, line in enumerate(out)
                      if line.startswith("    ") and " " not in line.strip()
                      and "failures:" in out[:n]]  # fmt: skip
            caught.append(f"rung 1 ({failed[0] if failed else '?'})")
        first = lockstep(short)
        if first is not None:
            caught.append(f"the gate ({first})")
        if clean:
            if caught:
                print(f"the unfaulted tree fails: {'; '.join(caught)}; fix that first")
                return 2
            print("(unfaulted): rung 1 and the short cases pass", flush=True)
            continue
        if not caught:
            first = lockstep(long)
            if first is not None:
                caught.append(f"rung 3, the cards ({first})")
        if not caught:
            states = subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "sgx_state_roundtrip.py"), "--binary",
                 str(binaries / "sgx-state-roundtrip"), "--jobs", str(args.jobs),
                 *(["--only", states] if states else [])],
                capture_output=True, text=True,
            )  # fmt: skip
            if states.returncode:
                first = next((line.split(":")[0] for line in states.stdout.splitlines() if "FAIL" in line), "?")
                caught.append(f"rung 5 ({first})")
        print(f"{name}: {'caught by ' + '; '.join(caught) if caught else 'MISSED'}", flush=True)
        missed += not caught
    print(f"{ran - missed} caught, {missed} missed")
    return 1 if missed else 0


if __name__ == "__main__":
    raise SystemExit(main())
