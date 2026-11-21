<!-- SPDX-License-Identifier: MIT -->
# The ladder in detail

The README has the results table and the commands; this is how each rung is built and what its
cases are. The figures are those of the run recorded at the commit that added them (2026-11-12,
Linux x86_64, rustc 1.93.1, the reference on PyPy 7.3.20 with `PYPY_GC_NURSERY=1M`).

## The gate

`scripts/sgx_lockstep.py` runs each case of `scripts/cases.tsv` on both sides:
`scripts/sgx_trace.py` on PyPy builds the reference's `SuperGrafx` and `target/release/sgx-trace`
this crate's, and each writes one line a frame, the base gate's line: the frame counter, the master
clock, the count and SHA-1 of the CPU bus log (`address << 8 | value`, `mclk << 2 | kind` for every
cycle), the SHA-1 of the picture (`capture.last_frame`: the rows the VPC mixed, at the 9-bit value,
then the dividers), the count and SHA-1 of the mixed audio and every field of the CPU's state. A
case passes when the two files are identical. Otherwise the first differing frame is named and both
sides are run again with `--detail` on it, which narrows the difference to the first differing bus
cycle.

The line, the bus tap, the comparison and the archive handling are the base board's, imported by
path from a checkout of [pcengine-rust](https://github.com/alewman/pcengine-rust) (`scripts/pce_trace.py`,
`scripts/lockstep.py`; `$PCENGINE_RUST`, default `../pcengine-rust`), as the reference's scripts import
pcengine-python's; only the building of the machine is this repository's. That checkout should be at
the revision `Cargo.toml` names. The reference's traces are kept (`--reference-cache
external/refcache`), so a rerun costs the Rust side only. Archives are extracted one at a time into
the ignored `scratch/` directory and removed; the library is only read.

In all, 10,260 frames in 22 cases, and no case has ever differed: the transcription was identical to
the reference on the first run of each. (The per-cycle narrowing has therefore only been exercised by
the seeded faults.)

## Rung 3: the cards and the switches

The five cards are 1941 Counter Attack, Aldynes, Battle Ace, Daimakaimura and Madou King Granzort,
each 600 frames from power-on with RUN at frame 250. Daimakaimura with `run@400+10 run@700+10` is in
its first stage at frame 1,800, where the VPC holds `$75 $56` and the windows `$000` and `$3FF`
(`sgx-bench` prints them): identical, and identical again under `PRIORITY_LABELS = sgxtech`, whose
picture leaves the default's at frame 155. In PC Engine position (`--mode compat`) it runs 300
frames, identical.

**A switch that changes nothing proves nothing** (the base Rust board's rule):
`scripts/sgx_switch_effect.py` compares each switch case's reference trace with the same card's trace
without the switch and fails if a switch has no case it changes. The library's first seconds do not
reach most of the VPC (supergrafx-python's `docs/m6-record.md`: no card moves a window edge into the
picture or writes a register mid-line near its checkpoints), so `scripts/sgx_probe.py` builds a
synthetic 8 KiB card, this repository's own code and not a ROM of anything, that does: both VDCs with
background, sprites and holes, VDC 2's display narrower than VDC 1's; every turn of a free-running
loop writes both priority registers from a table of all 256 values and both window widths (window 1
sweeps its 10 bits), so that writes land on every dot of a line; `$08-$0F`, the gap and two mirrors
are read back; `ST0`-`ST2` go to VDC 2 with `$0E` set and to VDC 1 with it clear, with plain stores to
both in between; VDC 2's VRAM is read back; the dot clock changes every 256 turns.

| Switch | Away from its default on | First frame it changes, and what |
| --- | --- | --- |
| `PRIORITY_LABELS = sgxtech` | Daimakaimura's first stage; the probe | frame 155, the picture; frame 0, the picture |
| `WINDOW_ORIGIN = 40, 40, 40` (48 until 2026-11-19) | the probe | frame 0, the picture |
| `WINDOW_OFF_AT_OR_BELOW` | Daimakaimura's first stage (1023: no window at all); the probe (256) | frame 1,506, the picture; frame 0, the picture |
| `ST_STEERS_IN_COMPAT = true` | the probe with the back-panel switch moved after frame 60, `$0E` left set | frame 60, the picture |
| the back-panel switch | Daimakaimura and the probe from power-on; the probe, moved after frame 60 | frame 0, the bus log; frame 60, the bus log |
| `power_on_seed` (a base board switch, here over two VDCs and 32 KiB) | the probe | frame 4, the bus log |
| the fallback (`--mode base`: one VDC, no VPC, 32 KiB) | Daimakaimura, the probe, Covell's test | frame 0, the bus log |

`WINDOW_ORIGIN` has no case on a game: Daimakaimura's first stage holds its windows at `$000` and
`$3FF` (off, and the whole row), where an origin cannot show; the case was tried, had no effect, and
was dropped.

## Rung 4: the add-on with VDC 2 idle

`tests/gate.rs` is the reference's `tests/test_gate.py`: Covell's CPU test on the add-on and on the
fallback row (`trace::BASE_MODE`), four frames, every row equal and the same master clock. It reads
the card from `$COVELL` and is skipped when the file is absent, which `scripts/rung4.sh` does not
allow. The lockstep cases add the reference's side: Covell's test on the add-on, on the fallback and
in PC Engine position, and Daimakaimura on the fallback (the measured picture of supergrafx-python's
`docs/m6-record.md`), each identical to the reference.

## Rung 5: save states

As the reference's `scripts/sgx_state.py`: the five cards saved 148 to 696 instructions into frame 300
with a RUN press before, Daimakaimura in stage 1 (saved inside frame 1,650, pad input after the save)
and in PC Engine position, and the probe card with the switch moved before the save. Each state is
loaded into a fresh machine and into one that had run, the 60 frames after compared by the gate's own
lines, and the loaded machine saved at once gives the same bytes. States are 4 to 71 KiB. Then the
snapshot pair (which the reference does not have): after each frame a snapshot, two frames run ahead
with other buttons held, the snapshot put back; every frame's line and the end state equal the plain
run's. A snapshot is 338 to 394 KiB. `tests/state.rs` has the reference's two tests (the VPC, the
page, the switch and the second VDC come back; a plain board refuses the state, with the reference's
message, and is left untouched) and the round trip on a synthetic card.

## Rung 6: seeded faults

[seeded-faults.md](seeded-faults.md).

## 0.2.0 (2026-11-13)

Run again against supergrafx-python `v0.1.1` (`7bf3c52`) on pcengine-python `v0.2.0` (`4c32f26`) and
huc6280-python `v0.4.0` (`265df88`), with pcengine-rust 0.2.0 by path, the reference cache rebuilt
(the old one kept as `external/refcache-v0.1.0`). The frame line's CPU text ends with
`polled_irq_lines` (the base gate's spelling). Results are in the README's status table.

## 2026-11-19: the clean rewrite

Run against supergrafx-python `f04d648` on pcengine-python `3318027` and huc6280-python `6ce348e`,
with pcengine-rust `26ea621` by path, the reference cache rebuilt (the old one kept as
`external/refcache-v0.1.1`). The cards are now read as their No-Intro `.zip` images under
`$NO_INTRO_DIR` (the same ROM bytes), and `sw-origin-probe` sets `WINDOW_ORIGIN=40,40,40`. Results
and the two failing probe cases (a base-board bug): [restart.md](restart.md).
