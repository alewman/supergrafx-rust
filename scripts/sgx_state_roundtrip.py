#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Rung 5's driver: the save-state round trip and the run-ahead pass on real cards.

For each case below the card is extracted into scratch/ and ``sgx-state-roundtrip`` runs it:
saved a case-dependent number of instructions into a frame, loaded into a fresh machine and
into one that had run, the frames after compared by the lockstep gate's own lines (bus log,
picture, audio, CPU state); the loaded machine saved at once must give the same bytes; then a
run with a snapshot taken, two frames run ahead and the snapshot put back after every frame.
The cases are the reference's (``scripts/sgx_state.py``, docs/start-here.md "Save states"): the
five library cards saved inside frame 300 with a RUN press before, Daimakaimura in stage 1
(saved inside frame 1650, pad input after the save) and in PC Engine position; and the probe
card with the switch moved before the save.

    sgx_state_roundtrip.py [--only REGEX] [--jobs N] [--binary PATH]
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import sgx_lockstep as gate  # noqa: E402

DMM = "$LIB/Daimakaimura (Japan) (En).zip"
LIBRARY = [case for case in gate.cases(HERE / "cases.tsv") if case[0].startswith("lib-")]
# name, image, the machine's arguments, the round trip's
CASES = [
    *((name, image, args, ["--at", "300", "--to", "360", "--extra", str(148 + 137 * n)])
      for n, (name, _, image, args) in enumerate(LIBRARY)),
    ("dmm-stage1", DMM, ["--press", "run@400+10", "--press", "run@700+10", "--press", "right-ii@1660+40"],
     ["--at", "1650", "--to", "1710", "--extra", "777", "--ahead-frames", "60"]),
    ("dmm-compat", DMM, ["--mode", "compat"], ["--at", "300", "--to", "360", "--extra", "777"]),
    ("probe-compat-at", "$PROBE", ["--compat-at", "40"], ["--at", "50", "--to", "110", "--extra", "500"]),
]  # fmt: skip


def one(case, binary: Path) -> tuple[str, bool, str]:
    name, image, args, trip = case
    for key, value in {**gate.DEFAULTS, **{k: os.environ[k] for k in gate.DEFAULTS if k in os.environ}}.items():
        image = image.replace(f"${key}", value)
    work = gate.SCRATCH / f"state-{gate.safe_name(name)}-{os.getpid()}"
    try:
        if not Path(image).is_file():
            return name, False, f"no image at {image}"
        card = gate.extracted(image, work)
        done = gate.run([str(binary), str(card), *args, *trip])
        text = (done.stdout + done.stderr).strip()
        return name, done.returncode == 0, f"{' '.join(trip)}: {text}"
    finally:
        if work.is_dir():
            shutil.rmtree(work)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--only")
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    binary = args.binary
    if binary is None:
        build = subprocess.run(["cargo", "build", "--release", "-q", "--manifest-path", str(ROOT / "Cargo.toml")])
        if build.returncode:
            return 1
        binary = ROOT / "target" / "release" / "sgx-state-roundtrip"
    todo = [case for case in CASES if not args.only or re.search(args.only, case[0])]
    if any(case[1] == "$PROBE" for case in todo):
        subprocess.run([sys.executable, str(HERE / "sgx_probe.py")], check=True)
    failed = 0
    with ThreadPoolExecutor(max(1, min(args.jobs, gate.MAX_JOBS))) as pool:
        for name, ok, text in pool.map(lambda case: one(case, binary), todo):
            print(f"{name}: {text}", flush=True)
            failed += not ok
    print(f"{len(todo) - failed} of {len(todo)} cases: a loaded run continues the same")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
