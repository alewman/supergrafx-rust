#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""The lockstep gate's reference side: supergrafx-python, traced frame by frame.

Run on PyPy with PYTHONPATH on the reference's checkouts (external/, scripts/fetch_reference.sh;
scripts/lockstep.py does both). The lines are the base gate's: this imports the base Rust
board's ``scripts/pce_trace.py`` by path (``$PCENGINE_RUST``, default ``../pcengine-rust``) for
the bus tap, the frame line, the CPU text and the press spelling, and only builds the machine
itself. ``target/release/sgx-trace`` writes the same lines; ``scripts/lockstep.py`` compares.

    sgx_trace.py IMAGE --frames N [--mode sgx|compat|base] [--vpc NAME=VALUE ...]
                 [--compat-at FRAME] [--press BUTTONS@FRAME+HOLD ...] [--seed N]
                 [--start-line N] [--start-offset N] [--vblank-dots N|none]
                 [--no-dma-resume] [--device pad2|none]
                 [--detail FRAME] --out FILE

``--mode``: ``sgx`` is the add-on in native position, ``compat`` the back-panel switch in PC
Engine position, ``base`` the fallback (the card on the plain board with 32 KiB and one VDC, as
the reference's ``scripts/sgx_ours.py``). ``--vpc`` sets a named switch on the instance by its
attribute name: ``PRIORITY_LABELS=sgxtech``, ``WINDOW_ORIGIN=40,40,40`` (dividers 4, 3, 2),
``WINDOW_OFF_AT_OR_BELOW=N``, ``ST_STEERS_IN_COMPAT=true`` (the page's). ``--compat-at FRAME``
moves the switch to PC Engine position once FRAME frames have completed.
"""

from __future__ import annotations

import argparse
import os
import sys
from dataclasses import replace
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(os.environ.get("PCENGINE_RUST", ROOT.parent / "pcengine-rust"))
sys.dont_write_bytecode = True  # nothing is written into the base repository
sys.path.insert(0, str(BASE / "scripts"))
import pce_trace  # noqa: E402  (the base gate's reference side: tooling, not chips)

from pcengine_python import pads  # noqa: E402
from pcengine_python.board import Board  # noqa: E402
from pcengine_python.cards import make_card  # noqa: E402
from pcengine_python.system import build  # noqa: E402
from supergrafx_python import SUPERGRAFX, SuperGrafx, Vpc  # noqa: E402


def set_switch(vpc: Vpc, name: str, text: str):
    """A named switch on the instance; ST_STEERS_IN_COMPAT (the page's) is returned."""
    if name == "PRIORITY_LABELS":
        if text not in ("detectors", "sgxtech"):
            raise SystemExit(f"{name}: detectors or sgxtech, not {text}")
        vpc.PRIORITY_LABELS = text
    elif name == "WINDOW_ORIGIN":
        four, three, two = (int(part) for part in text.split(","))
        vpc.WINDOW_ORIGIN = {4: four, 3: three, 2: two}
    elif name == "WINDOW_OFF_AT_OR_BELOW":
        vpc.WINDOW_OFF_AT_OR_BELOW = int(text)
    elif name == "ST_STEERS_IN_COMPAT":
        return text.lower() in ("true", "1")
    else:
        raise SystemExit(f"no VPC switch {name}")
    return None


def make(a: argparse.Namespace):
    """(board, the SuperGrafx or None, the pads the presses go to)."""
    with open(a.image, "rb") as handle:
        card = make_card(handle.read())
    pad = None if a.device == "none" else pads.Pad2()
    kwargs = dict(
        pad=pad,
        power_on_seed=a.seed,
        start_line=a.start_line,
        start_offset=a.start_offset,
        vblank_dots=None if a.vblank_dots == "none" else int(a.vblank_dots),
        dma_resumes=a.dma_resume,
    )
    pressed = [pad] if pad is not None else []
    if a.mode == "base":
        row = replace(SUPERGRAFX, name="SuperGrafx (base mode)", vdc_count=1)
        return Board(row, card=card, **kwargs), None, pressed
    vpc = Vpc()
    steers = None
    for item in a.vpc:
        name, _, text = item.partition("=")
        answer = set_switch(vpc, name, text)
        steers = answer if answer is not None else steers
    sg = SuperGrafx(card=card, compat=a.mode == "compat", vpc=vpc, **kwargs)
    if steers is not None:
        sg.page.ST_STEERS_IN_COMPAT = steers
    return sg.board, sg, pressed


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("image")
    p.add_argument("--frames", type=int, default=60)
    p.add_argument("--mode", choices=("sgx", "compat", "base"), default="sgx")
    p.add_argument("--vpc", action="append", default=[], metavar="NAME=VALUE")
    p.add_argument("--compat-at", type=int)
    p.add_argument("--press", action="append", default=[])
    p.add_argument("--seed", type=int)
    p.add_argument("--start-line", type=int, default=0)
    p.add_argument("--start-offset", type=int, default=0)
    p.add_argument("--vblank-dots", default="-16")
    p.add_argument("--no-dma-resume", dest="dma_resume", action="store_false")
    p.add_argument("--device", choices=("pad2", "none"), default="pad2")
    p.add_argument("--detail", type=int)
    p.add_argument("--out", required=True)
    a = p.parse_args()
    presses = [pce_trace.parse_press(spec) for spec in a.press]
    board, sg, pressed = make(a)
    cycles: list[str] = []
    log = pce_trace.observe(board, cycles if a.detail is not None else None)
    system = build(board)
    with open(a.out, "w") as out:
        for _ in range(a.frames):
            if a.compat_at is not None and a.compat_at == system.frame and sg is not None:
                sg.compat = True
            mask = pce_trace.held(presses, system.frame)
            for pad in pressed:
                pad.pressed = mask
            wanted = a.detail is not None and a.detail == system.frame + 1
            del cycles[:]
            system.run_frame()
            line = pce_trace.frame_line(system, log)
            if a.detail is None:
                out.write(line + "\n")
            elif wanted:
                out.write("\n".join(cycles) + "\n")
                break
    return 0


if __name__ == "__main__":
    sys.exit(main())
