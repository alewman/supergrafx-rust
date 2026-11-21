#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Frames per second of the reference on a card, nothing logged or digested (run on PyPy with
PYTHONPATH on the reference's checkouts). The counterpart of ``sgx-bench``.

    sgx_bench.py IMAGE [--frames N] [--mode sgx|compat|base] [--press BUTTONS@FRAME+HOLD ...]

Prints the whole run and its second half alone (PyPy's JIT has warmed up by then).
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sgx_trace  # noqa: E402

pce_trace = sgx_trace.pce_trace


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("image")
    p.add_argument("--frames", type=int, default=300)
    p.add_argument("--mode", choices=("sgx", "compat", "base"), default="sgx")
    p.add_argument("--press", action="append", default=[])
    a = p.parse_args()
    a.vpc, a.device, a.seed, a.start_line, a.start_offset = [], "pad2", None, 0, 0
    a.vblank_dots, a.dma_resume = "-16", True
    presses = [pce_trace.parse_press(spec) for spec in a.press]
    board, _sg, pressed = sgx_trace.make(a)
    system = sgx_trace.build(board)
    began = time.perf_counter()
    half = None
    for n in range(a.frames):
        if n == a.frames // 2:
            half = time.perf_counter()
        for pad in pressed:
            pad.pressed = pce_trace.held(presses, system.frame)
        system.run_frame()
        system.board.take_audio()
    end = time.perf_counter()
    print(
        f"{a.frames} frames in {end - began:.3f} s: {a.frames / (end - began):.2f} frames per second; "
        f"the second half {(a.frames - a.frames // 2) / (end - half):.2f} (mclk {system.board.mclk})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
