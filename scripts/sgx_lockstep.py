#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""The lockstep gate: run each case on the reference and on this crate and compare the traces.

Each case of ``scripts/cases.tsv`` runs ``sgx-trace`` (this crate) and ``scripts/sgx_trace.py``
on the reference (PyPy, ``PYPY_GC_NURSERY=1M``, PYTHONPATH on the checkouts of
``scripts/fetch_reference.sh``), several at a time under ``nice``; the two traces are compared
line for line (a line is a frame: the CPU bus log, the mixed picture, the mixed audio and the
CPU state, digested). A case passes when they are identical. Otherwise the first differing
frame is reported and both are run again with ``--detail`` on that frame, which narrows it to
the first differing bus cycle (when the bus logs agree, the line itself says which of picture,
audio and CPU state differs).

    sgx_lockstep.py [--only REGEX] [--jobs N] [--rust PATH] [--keep] [--reference-cache DIR]
                [--no-narrow] [--quiet]

The shape, and the helpers (the case file's reading, the archive extraction, the comparison,
the cache's names), are the base Rust board's ``scripts/lockstep.py``, imported by path
(``$PCENGINE_RUST``, default ``../pcengine-rust``). The reference's trace of a case never changes
while the pins do not: ``--reference-cache external/refcache`` keeps each one, keyed by the case
and the four reference checkouts' commits (so a repin starts afresh).

An image in a ``.zip`` or ``.7z`` is extracted (``7z e -so``) into ``scratch/`` for the case
and removed afterwards; the library is only read. ``$LIB`` (the five SuperGrafx cards under
their No-Intro names, as ``.zip``; default ``$NO_INTRO_DIR/NEC - PC Engine SuperGrafx``),
``$COVELL`` (Covell's CPU test; default in a sibling checkout of pcengine-python,
``tests/test_roms/CPU_Test/CPU_Test_10.pce``, which its ``scripts/fetch_references.py`` fetches)
and ``$PROBE`` (the synthetic card of scripts/sgx_probe.py) come from the environment. Stdlib
only.
"""

from __future__ import annotations

import argparse
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
BASE = Path(os.environ.get("PCENGINE_RUST", ROOT.parent / "pcengine-rust"))
sys.dont_write_bytecode = True  # nothing is written into the base repository
sys.path.insert(0, str(BASE / "scripts"))
import lockstep as base  # noqa: E402  (the base gate: tooling)

PYPY = Path(os.environ.get("PYPY") or shutil.which("pypy3.11") or "pypy3.11")  # PyPy 3.11 on PATH
EXTERNAL = ROOT / "external"
REFERENCE = [EXTERNAL / name / "src" for name in ("supergrafx-python", "pcengine-python", "huc6280-python", "6502-python")]
SCRATCH = ROOT / "scratch"
ENV = {
    **os.environ,
    "PYPY_GC_NURSERY": "1M",
    "PYTHONPATH": os.pathsep.join(str(path) for path in REFERENCE),
    "PYTHONDONTWRITEBYTECODE": "1",
    "PCENGINE_RUST": str(BASE),
}
MAX_JOBS = 8
DEFAULTS = {
    "LIB": str(Path(os.environ.get("NO_INTRO_DIR", "NO_INTRO_DIR-unset")) / "NEC - PC Engine SuperGrafx"),
    "COVELL": str(ROOT.parent / "pcengine-python/tests/test_roms/CPU_Test/CPU_Test_10.pce"),
    "PROBE": str(SCRATCH / "sgx-probe.pce"),
}

Case = tuple[str, int, str, list[str]]
safe_name, extracted = base.safe_name, base.extracted


def reference_commits() -> str:
    """The four reference checkouts' commits, part of every cache key: a repin of any of them
    starts a fresh cache (added 2026-11-19)."""
    heads = []
    for path in REFERENCE:
        head = subprocess.run(["git", "-C", str(path.parent), "rev-parse", "HEAD"], capture_output=True, text=True)
        heads.append(head.stdout.strip() if head.returncode == 0 else "unknown")
    return "+".join(heads)


REFERENCE_COMMITS = reference_commits()


def cache_name(case: Case) -> str:
    return base.cache_name(case, REFERENCE_COMMITS)


first_difference, fields = base.first_difference, base.fields


def cases(path: Path) -> list[Case]:
    env = {**DEFAULTS, **{k: os.environ[k] for k in DEFAULTS if k in os.environ}}
    rows = []
    for line in path.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        name, frames, image, args = (line.split("\t") + [""])[:4]
        for key, value in env.items():
            image = image.replace(f"${key}", value)
        rows.append((name, int(frames), image, shlex.split(args)))
    return rows


def run(command: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(["nice", *command], capture_output=True, text=True, env=ENV)


def reference(common: list[str], out: Path) -> subprocess.CompletedProcess:
    return run([str(PYPY), str(HERE / "sgx_trace.py"), *common, "--out", str(out)])


def narrow(case: Case, image: Path, rust: Path, traces: Path, frame: str) -> str:
    """The first differing bus cycle of `frame` (the frame counter a line starts with)."""
    name, frames, _, args = case
    ref, ours = traces / f"{safe_name(name)}.detail.ref", traces / f"{safe_name(name)}.detail.rust"
    common = [str(image), "--frames", str(frames), *args, "--detail", frame]
    a = reference(common, ref)
    b = run([str(rust), *common, "--out", str(ours)])
    if a.returncode or b.returncode:
        return "narrowing failed: " + (a.stderr or b.stderr).strip()[-300:]
    found = first_difference(ref, ours)
    if found is None:
        return "every bus cycle of that frame is the same on both"
    number, left, right = found
    before = ref.read_text().splitlines()[max(0, number - 3) : number]
    context = "".join(f"\n        {line}" for line in before)
    return (
        f"bus cycle {number} of that frame (r/w ADDRESS VALUE MCLK, i MCLK){context}\n"
        f"      reference {left!r}\n      rust      {right!r}"
    )


def one(case: Case, rust: Path, traces: Path, keep: bool, cache: Path | None, narrowing: bool):
    name, frames, image, args = case
    safe = safe_name(name)
    ref, ours = traces / f"{safe}.ref", traces / f"{safe}.rust"
    if cache is not None:
        ref = cache / cache_name(case)  # kept: the reference's trace does not change
    work = SCRATCH / f"case-{safe}-{os.getpid()}"
    began = time.monotonic()
    try:
        if not Path(image).is_file():
            return name, False, f"no image at {image}"
        card = extracted(image, work)
        common = [str(card), "--frames", str(frames), *args]
        if cache is None or not ref.is_file():
            partial = ref.with_suffix(".partial")
            done = reference(common, partial)
            if done.returncode:
                partial.unlink(missing_ok=True)
                return name, False, "reference failed: " + done.stderr.strip()[-400:]
            partial.replace(ref)
        board = run([str(rust), *common, "--out", str(ours)])
        if board.returncode:
            return name, False, "rust failed: " + board.stderr.strip()[-400:]
        seconds = time.monotonic() - began
        found = first_difference(ref, ours)
        if found is None:
            if not keep:
                if cache is None:
                    ref.unlink()
                ours.unlink()
            return name, True, f"identical, {frames} frames ({seconds:.0f} s)"
        number, left, right = found
        if not left or not right:
            return name, False, f"line {number}: one trace is longer"
        x, y = fields(left), fields(right)
        differing = [key for key in x if x.get(key) != y.get(key)]
        frame = left.split()[0]
        text = f"frame {frame} (line {number}) differs in {', '.join(differing)}"
        text += f"\n      reference {left!r}\n      rust      {right!r}"
        if narrowing and ("bus" in differing or "mclk" in differing):
            text += "\n      " + narrow(case, card, rust, traces, frame)
        return name, False, text
    finally:
        if work.is_dir():
            shutil.rmtree(work)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--only", help="a regular expression on the case name")
    parser.add_argument("--cases", type=Path, default=HERE / "cases.tsv")
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--rust", type=Path, help="a trace binary to judge instead of this tree's")
    parser.add_argument("--traces", type=Path, default=EXTERNAL / "traces")
    parser.add_argument("--keep", action="store_true", help="keep the traces of identical cases")
    parser.add_argument("--reference-cache", type=Path,
                        help="keep the reference's traces here and reuse them (the pins do not move)")
    parser.add_argument("--no-narrow", action="store_true", help="do not rerun a failing frame per cycle")
    parser.add_argument("--quiet", action="store_true", help="print only failures and the count")
    args = parser.parse_args()
    rust = args.rust
    if rust is None:
        build = subprocess.run(["cargo", "build", "--release", "-q", "--manifest-path", str(ROOT / "Cargo.toml")])
        if build.returncode:
            return 1
        rust = ROOT / "target" / "release" / "sgx-trace"
    if not REFERENCE[0].is_dir() and not args.reference_cache:
        print(f"no reference at {REFERENCE[0]}: run scripts/fetch_reference.sh")
        return 2
    args.traces.mkdir(parents=True, exist_ok=True)
    if args.reference_cache:
        args.reference_cache.mkdir(parents=True, exist_ok=True)
    todo = [c for c in cases(args.cases) if not args.only or re.search(args.only, c[0])]
    if any(Path(c[2]).name == "sgx-probe.pce" for c in todo):
        subprocess.run([sys.executable, str(HERE / "sgx_probe.py")], check=True)
    failed = 0
    jobs = max(1, min(args.jobs, MAX_JOBS))
    with ThreadPoolExecutor(jobs) as pool:
        work = lambda c: one(c, rust, args.traces, args.keep, args.reference_cache, not args.no_narrow)  # noqa: E731
        for name, ok, text in pool.map(work, todo):
            if not ok or not args.quiet:
                print(f"{name}: {text}", flush=True)
            failed += not ok
    print(f"{len(todo) - failed} of {len(todo)} cases identical")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
