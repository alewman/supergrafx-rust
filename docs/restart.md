<!-- SPDX-License-Identifier: MIT -->
# The clean rewrite of 2026-11-19, carried into supergrafx-rust

Aubrey, 2026-11-19: "rewrite the modules that used GPL sources if we haven't already." The licence
audit of 2026-11-13 ([provenance.md](provenance.md)) marked one item of this crate **unclear**,
inherited from supergrafx-python: `src/vpc.rs`'s window origin (a GPL emulator's constants, read
from its source) together with the both-transparent rule (U-SGX5, stated as two emulators' source).
supergrafx-python settled both first (its `docs/restart.md`, `5c5ada6`) and then repinned its base
board to pcengine-python's clean VDC rewrite, re-measuring the window origin (`f04d648`); the base
board's Rust transcription carried its own rewrite (pcengine-rust `26ea621`, with huc6280-rust
`e1d24a3`). This pass carries all of it here. Fixes start upstream: nothing in this crate's
behaviour was decided here.

## The rules this pass kept

1. **No GPL or LGPL source was read**: Mednafen, Beetle PCE, Mesen2, Geargrafx, the MiSTer core,
   MAME's GPL files. Not to learn, not to check a constant, not when a comparison failed.
2. **The replaced Rust code was not read.** Before writing, the Rust counterparts of what the
   reference withdrew were moved, unread, into the git-ignored `scratch/withdrawn/` by small
   Python helpers (`scratch/withdraw_scan.py`, `withdraw_shape.py`, `withdraw.py`, also ignored)
   that printed only item names, line numbers and line shapes (comment, code, brace depth), never
   a line's text:
   - from `src/vpc.rs`: the module documentation; `WindowOrigin` and its `impl` (the reference's
     `WINDOW_ORIGIN`); the `window_origin` field with its comment and its initialiser;
     `window_edge`; `mix_run` (the reference's `_mix_run`); and the comments the reference
     reworded because they named GPL emulators' internals: `PriorityLabels`' and its variants'
     doc comments, the `priority_labels` field's comment (C-SGX1) and the U-SGX3 comment;
   - from `scripts/sgx_seeded_faults.py`: the seven faults whose text was a line of the withdrawn
     code (`window-of-0x40-is-on`, `window-origin-ignores-the-dot-clock`, `both-off-shows-vdc1`,
     `transparent-pair-shows-vdc1`, `vdc2-sprites-over-vdc1-sprites`,
     `code-10-is-the-default-order`, `priority-labels-not-read`), found by matching each fault's
     text against the withdrawn file and printing only the names.
   They were not opened again. The rest of `vpc.rs` (registers, `mix`, `replay`, `region_config`,
   `span`, the page), the call sites (`src/lib.rs`, `src/trace.rs`) and `tests/vpc.rs` defined the
   interface: `WindowOrigin { divider4, divider3, divider2 }` and `mix_run(&self, out, a, b, cfg)`.
3. **The reference's new code was read, never its deleted diff lines**: the new `vpc.py`, and the
   added lines only of the other files that changed (tests, `pyproject.toml`, the docs).
4. **Transcribed line by line** from supergrafx-python `f04d648`'s `vpc.py`: the same names, order
   and comments with their evidence tags (DOC, OBSERVED, closed-box measurement). The seven
   faults were written anew against the new lines, with the same names and meaning, and three
   added (`window-origin-before-the-repin`, `vdc2-alone-shows-vdc1`, and the unfaulted check).
5. **The judge is the reference**, as always for this tier: every case of the lockstep gate. How
   close the reference is to the hardware is the reference's record (its `docs/restart.md`: the
   five cards and Daimakaimura against Mesen2 and Beetle PCE as closed boxes, the window probes).

## What changed

| Item | Now |
| --- | --- |
| `Vpc::window_origin` default | `WindowOrigin { divider4: 24, divider3: 38, divider2: 24 }` (was 32 / 46 / 32): closed-box measurement (emulator tier, Beetle PCE b96c11e, Mesen2 b9fa69d) retaken by the reference on the clean base board, whose picture starts 8 dots earlier in the row; 7.16 MHz provisional (Mesen2 gives 16) |
| `window_edge`, `mix_run` | transcribed again; no behaviour change in the reference (`5c5ada6` changed no default) |
| comments | C-SGX1, U-SGX1, U-SGX3, U-SGX5 as the reference words them now: closed-box observations named as such, no GPL emulator's source or internals |
| base board | pcengine-rust `26ea621` by path (`../pcengine-rust`), the git pin named in `Cargo.toml` for after the push (since 2026-11-20 the git pin `8f672f0`); it brought huc6280-rust `e1d24a3` by its own path dependency (a git pin since). Interfaces met here: `Vdc::new(power_on, vblank_dots, dma_resumes)` (`tests/board.rs`), `BoardOptions::vertical_at_line_start` and `trace::Setup::vertical_at_line_start` gone (`src/trace.rs`), `--vertical-at-line-start` gone (`scripts/sgx_trace.py`); `--vblank-dots` default `-16` on both sides (was `-8`); the timer window 3 by the base's defaults |
| save states | the base board's format version 3; a state from 0.2.0 is refused (the base's rule) |
| reference | supergrafx-python `f04d648`, pcengine-python `3318027`, huc6280-python `6ce348e`, 6502-python `2146eba` (`scripts/fetch_reference.sh`, from local clones until they were pushed on 2026-11-20; the gate runs PyPy with `PYTHONPATH` on the checkouts, nothing installed) |
| tests | `tests/vpc.rs` follows the reference's `test_vpc.py`: the origin per divider 24 / 38 / 24, the switch test with 40 (was 48, an emulator's value), `both_transparent_shows_vdc2s_pixel` transcribed |
| gate cases | `sw-origin-probe` uses `WINDOW_ORIGIN=40,40,40` (was `48,48,48`); the cards are read as `$LIB/<No-Intro name>.zip`, `$LIB` defaulting to `$NO_INTRO_DIR/NEC - PC Engine SuperGrafx` (same ROM bytes as the `.7z` images of 0.2.0, by SHA-1); `$COVELL` defaults to a sibling pcengine-python checkout and `$PYPY` to `pypy3.11` on `PATH`: no machine paths in the repository |
| seeder | copies `README.md` into its tree (`src/lib.rs` runs it as a doctest: without it `cargo test` failed whatever the fault, so the 0.2.0 record's rung-1 column could not be trusted) and checks the unfaulted tree first, as pcengine-rust's seeder now does |

## The gate, before and after

"Before" is 0.2.0 against supergrafx-python `v0.1.1` (2026-11-13, README); "after" is this pass
against `f04d648` with the reference cache rebuilt from scratch (the old one moved aside to
`external/refcache-v0.1.1`), rustc 1.93.1, PyPy 7.3.20 with `PYPY_GC_NURSERY=1M`.

| Check | Before | After |
| --- | --- | --- |
| Rung 1: `cargo test` | 53 pass | 55 pass (the reference's U-SGX5 test, transcribed; a test of a waited VDC read through the page) |
| Lockstep, all cases (rungs 2-4) | 22 of 22 identical | **20 of 22 identical** on the first run: `sw-compat-probe` and `sw-base-probe` stop in frame 102 with a panic in pcengine-rust's `Vdc::render_line` (`src/vdc/chip.rs:353`), where the reference draws a 682-dot row: a base-board bug in pcengine-python `3318027` ([notes on the reference](notes-on-the-reference.md), 7), reported, not fixed here. The 20 include the five cards, Daimakaimura's first stage under both priority readings, in PC Engine position and on the fallback, Covell's test three ways and the probe |
| Rung 3: switches that change a case | 8 of 8 | 8 of 8 |
| Rung 4: `tests/gate.rs` (VDC 2 idle = the fallback) | passes | passes |
| Rung 5: save states, round trip and run-ahead | 8 of 8 | 7 of 8: `probe-compat-at` stops on the same base-board bug |
| Rung 6: seeded faults | "32 caught, 0 missed" (in truth 31 caught, 1 missed: [seeded-faults.md](seeded-faults.md)) | **34 caught, 0 missed**, with the three failing cases left out ([seeded-faults.md](seeded-faults.md)) |
| clippy `-D warnings` (rustc 1.93.1 and stable 1.99.1, each its own target directory), fmt, wasm32 build, `cargo test` on stable | pass | pass |

The window probes' `sw-origin-probe` case (`WINDOW_ORIGIN=40,40,40`) changes the probe's picture from
frame 0, so the new default is exercised against the reference on every probe frame.

**Carried after this pass (2026-11-19):** the base board's fix for item 7 of the notes,
pcengine-python `03ac699` through pcengine-rust `37f17ba`, with supergrafx-python `8347704`
(repinned to it; nothing in the add-on changed). The reference cache was rebuilt (the
`f04d648` one moved aside to `external/refcache-f04d648`; the cache key now includes the four
reference commits). Lockstep **22 of 22 identical**, switches 8 of 8, rung 4 passes, save states
**8 of 8**, seeded faults **34 caught, 0 missed with no exclusions**, rung 1 55 tests, clippy
`-D warnings` on rustc 1.93.1 and stable, fmt and the wasm32 build pass.

## What stays open

Everything the reference's `docs/restart.md` leaves open stays open here: the 7.16 MHz window
origin (the two detectors differ by 22 dots; 38 provisional), and no console run of any VPC
behaviour. They are questions about the hardware and belong upstream.
