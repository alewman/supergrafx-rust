<!-- SPDX-License-Identifier: MIT -->
# Handoff brief: supergrafx-rust

> **History.** This is the brief the crate was built from on 2026-11-12, kept as written, with
> "What was built" appended by the session that built it. Its instructions (the rules on
> processes, commits, "do not push") were for that session and are history; paths under
> `/data/emu/` and `~/.local/opt/` are the author's machine (read them as "your checkout", "your ROM
> directory" and "your PyPy"). The README is the current account.

## The request

Aubrey, 2026-11-12: the PC Engine family in Rust, then its fast forks, then **play-pce, a browser
page that runs any game of the family**. The CPU core (`/data/emu/huc6280-rust`) and the base board
(`/data/emu/pcengine-rust`, all seven rungs, 275 lockstep cases identical) are built. This
repository is the SuperGrafx add-on; `pcenginecd-rust` is being built beside it.

## What it is

A Rust NEC SuperGrafx **transcribed from**
[supergrafx-python](https://github.com/alewman/supergrafx-python) at **`v0.1.0`**
(`eee789a`; local clone `/data/emu/supergrafx-python`), as an add-on crate that plugs into
pcengine-rust's socket traits exactly as the Python add-on plugs into pcengine-python's sockets:
the `SUPERGRAFX` machine row (two VDCs, 32 KiB), the HuC6202 `Vpc` (registers, windows, priority
mix as the board's `PixelPath`, every named switch with the Python default: `PRIORITY_LABELS`,
`WINDOW_ORIGIN`, `WINDOW_OFF_AT_OR_BELOW`, ...), the `VideoPage` decode with the compatibility
switch, `ST0-ST2` steering by the CPU's opcode, and the `SuperGrafx` builder.

Crate `supergrafx_rust`, package `supergrafx-rust`, edition 2021, MIT, `publish = false`.
Depend on the base board and core by path (`pcengine_rust = { path = "../pcengine-rust" }`); the
session that started you repins to GitHub revisions later.

Read first: supergrafx-python `README.md`, `docs/start-here.md`, `docs/m6-record.md`, then
`src/supergrafx_python/` (`machine.py`, `vpc.py`, `board.py`) and `tests/`; then pcengine-rust's
`README.md` (the socket table, "Rust-forced differences"), `src/sockets.rs` and
`src/fake_devices.rs` (`LockstepPath` is the skeleton of the VPC: a pixel path that claims pages,
reaches `pins.vdcs` and steers by `pins.opcode`), and its `scripts/` (the lockstep gate
`lockstep.py`, `pce_trace.py`, `make_cases.py`, `state_roundtrip.py`, `seeded_faults.py`): reuse
that shape, importing the base repository's scripts by path where it saves copying (the Python
add-on does the same with `PCENGINE_PYTHON`).

## The ladder (each rung a `scripts/rungN.sh` that says PASS)

1. **Unit tests**: supergrafx-python's `tests/test_vpc.py`, `test_board.py`, `test_core_st.py`
   transcribed (the decode, the mirrors, the gap, the windows, each priority code under both
   label readings, the ST steering, compat mode, reset).
2. **The gate**: a trace script that runs the Python `SuperGrafx` (PyPy, checkouts at the pins
   under `external/`) and a Rust `sgx-trace`, compared by the base gate's per-frame digests (CPU
   bus log, mixed picture, audio, CPU state), narrowing to the first differing bus cycle; a
   reference cache.
3. **The five library cards** (`/data/emu/roms-retrobat/supergrafx/*.7z`, read in place, extracted
   one at a time into `scratch/`, never committed): 600 frames each with a RUN press, identical.
   **Daimakaimura into its first stage** with `run@400+10 run@700+10` to frame 1800 (the VPC holds
   `$75 $56` with a window there), identical; the same under `PRIORITY_LABELS = sgxtech` (the
   switch is wired); Daimakaimura in compat mode (the switch in PC Engine position) 300 frames;
   every other named switch in a non-default position on a case it changes.
4. **The gate of supergrafx-python `tests/test_gate.py`**: with VDC 2 idle the add-on draws what
   the one-VDC board draws (Covell's CPU test, `/data/emu/pcengine-python/tests/test_roms/`).
5. **Save states**: the base board's `save_state`/`load_state` and the snapshot pair cover this
   machine (the VPC and the page save themselves through `DeviceState`): the round trip on the
   five cards and on Daimakaimura in stage 1, and a state made here refused by a plain board.
6. **Seeded faults**: at least 12, each caught by a rung; a fault that changes nothing is not one.

Measure speed (frames per second on two cards against the Python add-on on PyPy).

## Rules

- supergrafx-python is the spec. **Do not modify** it, pcengine-python, pcengine-rust,
  huc6280-rust or 6502-rust. If a socket of pcengine-rust lacks something the VPC needs, stop and
  report exactly what (the base board is shared with the CD crate being built at the same time).
- **No GPL or LGPL code, copied or transcribed** (pcengine-python `docs/provenance.md`): the only
  sources you transcribe are ours. Do not open any `reference/` directory.
- Never commit ROMs. At most 8 processes (another build runs beside you), `nice`; PyPy at
  `~/.local/opt/pypy3.11-v7.3.20-linux64/bin/pypy3.11` with `PYPY_GC_NURSERY=1M`. No `sed -i`,
  `rm -rf` or `ln -sfn`: small Python helpers. Temp files under `external/`, `target/`, `scratch/`.
- clippy: `~/.rustup/toolchains/1.93.1-*/bin/cargo-clippy` works (`cargo clippy` on PATH is too
  old); `-D warnings` clean, `cargo fmt`. Build the library for `wasm32-unknown-unknown` too.
- Commits locally at good points, trailer `Co-Authored-By: Claude Fable 5.2
  <noreply@anthropic.com>`. **Do not create a GitHub repository and do not push.**
- Report at the end: each rung with numbers, the faults table, the Rust-forced differences, speed,
  anything in the Python add-on that looked wrong, the final commit, and the public API a wasm page
  needs (building a SuperGrafx from card bytes, compat mode, how a page should decide that a card
  is a SuperGrafx card).

## What was built (2026-11-12)

The crate, all six rungs passing; the README has the certification table, the seeded-fault
table, the Rust-forced differences, the API and the notes on the reference.

- **`src/`**: `machine.rs` (`SUPERGRAFX`), `vpc.rs` (`Vpc` with the reference's named switches
  as fields; `VideoPage`, the decode with the compatibility switch and the ST steering by
  `pins.opcode`, which is the board's `PixelPath` and owns the `Vpc`), `board.rs` (`SuperGrafx`,
  `SuperGrafxOptions`, `video_page`, `set_compat`); not from the reference: `cards.rs`
  (`is_supergrafx_card`: the five images by SHA-1, or a `.sgx` name), `trace.rs` and the
  binaries `sgx-trace`, `sgx-state-roundtrip`, `sgx-bench`.
- **Nothing was missing from pcengine-rust's sockets**, and nothing outside this repository was
  modified. Two things would have saved code here, neither needed: the base's
  `trace::roundtrip` and `runahead` build their own boards from a `Setup`, so they are repeated
  in `src/trace.rs` over a machine built here (a builder argument would let add-ons call
  them); and the opcode a device sees cannot be set by a test (`Pins::opcode` and
  `Board::opcode` are private), so the ST unit tests drive the page with `DetachedPins` and,
  through the board, the real core.
- **The gate** imports the base Rust board's `scripts/pce_trace.py` and `lockstep.py` by path
  (`$PCENGINE_RUST`) for the frame line, the bus tap and the comparison; the reference runs on
  PyPy with `PYTHONPATH` on four checkouts at the pins (`scripts/fetch_reference.sh`), no venv.
- **Rungs**: 1: 52 tests. 2: Covell's CPU test and the probe card identical. 3: 17 of 17 cases
  identical (9,660 frames: the five cards at 600 frames, Daimakaimura to frame 1,800 in stage 1
  with the VPC at `$75 $56`, the same under `sgxtech`, compat 300 frames, the switches), 8 of 8
  switches change a case. 4: the gate test and 4 lockstep cases. 5: 8 of 8 round trips and
  run-ahead passes, a state refused by a plain board. 6: 30 faults caught, 0 missed. In all
  10,260 frames in 22 cases; none ever differed.
- **The probe card** (`scripts/sgx_probe.py`, built into `scratch/`, this repository's own code)
  exists because the library's first seconds reach little of the VPC. It was corrected twice
  after the first runs, both holes in the case and not in the crate: its sprites pointed at an
  empty pattern (so `PRIORITY_LABELS` changed nothing on it), and nothing made the two VDCs'
  transparent values differ (so the U-SGX5 fault was missed).
- **Speed**: 161-213 frames a second against the reference's 10-11 on PyPy (17 to 19 times;
  about 3 times real time).
- **Open**: the dependencies are by path (repin once pushed); no CI workflow (no remote yet);
  `is_supergrafx_card` knows the five images of this library by content and anything named
  `.sgx`, nothing else; no fast fork.

## Since then (2026-11-13)

The 52 tests and the 22 lockstep cases pass with the base board replaced by pcengine-rust-fast,
and the crate runs in play-pce. A CI workflow was added (it builds once the dependency is a GitHub
revision), and the README's example became a documentation test (53 tests). The README and
`docs/` were prepared for publication (the long tables moved into `docs/`), and the numbers
re-checked: `cargo test` and rung 3, against pcengine-rust `9c4608f`.
