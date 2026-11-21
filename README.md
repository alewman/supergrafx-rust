<!-- SPDX-License-Identifier: MIT -->
# supergrafx-rust

> **Lineage:** supergrafx-python → **supergrafx-rust** → play-pce ·
> fixes start in supergrafx-python and flow down by merges ·
> [family charter](https://github.com/alewman/emulator-lineage/blob/main/docs/charter.md)

A Rust **NEC SuperGrafx**, transcribed from
[supergrafx-python](https://github.com/alewman/supergrafx-python) at `f04d648` and proven
equivalent to it: every CPU bus cycle (physical address, value, kind, master clock), every frame's
mixed picture at the 9-bit palette value, every mixed audio sample and the CPU's state, frame by
frame. It is an add-on crate on the family's Rust PC Engine board,
[pcengine-rust](https://github.com/alewman/pcengine-rust), whose CPU is
[huc6280-rust](https://github.com/alewman/huc6280-rust); the CD-ROM² add-on beside it is
[pcenginecd-rust](https://github.com/alewman/pcenginecd-rust). supergrafx-python is the
specification (readable, cited, judged against Mesen2 and Beetle PCE); this crate adds nothing to the
machine and tunes nothing. MIT.

The base board runs the HuC6280, the VCE, the PSG and the VDCs; this crate adds what the SuperGrafx
has beyond a PC Engine, through the base board's socket traits, and copies nothing from it:

- `machine::SUPERGRAFX`, the machine row: two VDCs, 32 KiB of work RAM;
- `vpc::Vpc`, the HuC6202 video priority controller: its registers, the two windows, the four
  priority groups and the mix, with every named switch of the reference as a field with the
  reference's default;
- `vpc::VideoPage`, the decode of `$FF:0000-03FF` with the back-panel compatibility switch and the
  steering of `ST0`-`ST2` by the CPU's opcode; it is the board's `PixelPath`;
- `board::SuperGrafx`, the builder; `cards::is_supergrafx_card`, which images are SuperGrafx cards.

Written by Claude Fable 5.2 (Anthropic) for alewman, and prepared for publication by Claude Opus
5.5. No ROMs are included: bring your own.

## Status

**2026-11-19, later: the base board's fix carried.** pcengine-python `03ac699` (a display that
starts before the row is cropped; [notes on the reference](docs/notes-on-the-reference.md), 7),
through pcengine-rust `37f17ba` and supergrafx-python `8347704`: lockstep **22 of 22 identical**,
every switch changes a case, save states **8 of 8**, seeded faults **34 caught, 0 missed** with no
exclusions. The paragraph below is the clean rewrite's first run, before the fix.

**2026-11-19 (unreleased, local): the clean rewrite.** Follows supergrafx-python `f04d648` (its
clean rewrite and the repin to the clean base board) on pcengine-python `3318027` and
huc6280-python `6ce348e`, over pcengine-rust `26ea621` and huc6280-rust `e1d24a3`
([docs/restart.md](docs/restart.md)). `Cargo.toml` took pcengine-rust by path from a sibling
clone until the base commits were pushed; since 2026-11-20 it is a git pin (pcengine-rust
`8f672f0`: `37f17ba` plus its own git pin of the core and docs). Results:
rung 1 55 tests pass; the lockstep gate **20 of 22 cases identical**: the two others
(`sw-compat-probe`, `sw-base-probe`) and the round trip `probe-compat-at` stop in the base board,
where the probe card reaches a display that starts before the row (a reference bug in
pcengine-python `3318027`, reported, not fixed here: [notes on the
reference](docs/notes-on-the-reference.md), 7); every switch changes a case; save states 7 of 8;
seeded faults 34 caught, 0 missed (the seeder's test stage fixed: in 0.2.0 one
fault was counted caught by a doctest that could not build; a test added). The table below is the 0.2.0 record.

**2026-11-13: 0.2.0, following supergrafx-python `v0.1.1` on pcengine-python `v0.2.0` (its issue 0004
settled: the board runs its own events while the CPU waits on a VDC) and huc6280-python `v0.4.0`; all
six rungs pass against the new reference.** Each rung is a script that says PASS; the
lockstep gate compares this crate with the reference frame by frame, on PyPy 7.3.20 against rustc
1.93.1 on Linux x86_64.

| Rung | What | Result |
| --- | --- | --- |
| 1 | The reference's unit tests, transcribed | 53 tests pass |
| 2 | The gate itself: Covell's CPU test (60 frames) and the probe card (120 frames) | identical, both |
| 3 | The five SuperGrafx cards, 600 frames each; Daimakaimura into its first stage (1,800 frames), under the other priority reading and in PC Engine position; every named switch away from its default | 17 of 17 cases identical (9,660 frames); each of the 8 switches changes a case |
| 4 | With VDC 2 idle the add-on draws what the one-VDC board draws (the reference's `test_gate.py`), and the fallback in lockstep | the test passes; 4 of 4 cases identical |
| 5 | Save states: round trip and run-ahead | 8 of 8 cases equal, twice each |
| 6 | Seeded faults | 32 caught, 0 missed (two new, on the base board's VDC wait) |

In all 10,260 frames in 22 lockstep cases, and no case has ever differed: at `v0.1.0` (2026-11-12) and
again at `v0.1.1` (2026-11-13, the reference cache rebuilt, 22 of 22 identical on the first run). The 52 unit tests of that
day (the README's example has joined them since) and the 22 lockstep cases also pass with the base
board replaced by its speed fork, pcengine-rust-fast, and the crate runs in the family's browser
page, [play-pce.supertux.com](https://play-pce.supertux.com).
Nothing here was run on a real console. Not covered: a SuperGrafx card other than the five dumps of
the author's library is recognised only by a `.sgx` file name; the cards were run through their
openings and first stage, not played through; the reference itself is judged against emulators
(Mesen2, Beetle PCE), not hardware, and this crate inherits that.

## Using it

```toml
[dependencies]
supergrafx-rust = { git = "https://github.com/alewman/supergrafx-rust", rev = "<commit>" }
```

Not on crates.io; name a commit of this repository. The crate re-exports the base board as
`supergrafx_rust::pcengine_rust`, so a host needs only this dependency. The example is compiled
with the documentation tests:

```rust
use supergrafx_rust::pcengine_rust::pads;
use supergrafx_rust::pcengine_rust::state::{load_state, save_state};
use supergrafx_rust::{is_supergrafx_card, set_compat, SuperGrafx};

fn run(image: &[u8], file_name: &str) -> Result<(), String> {
    // Which machine? One of the five known images, or a file named .sgx.
    if !is_supergrafx_card(image, Some(file_name)) {
        return Err("a PC Engine card: build pcengine-rust's board".to_string());
    }
    // A SuperGrafx from card bytes, the switch in SuperGrafx position, a 2-button pad.
    let mut system = SuperGrafx::with_card(image, false)?.build();

    // One frame: hold buttons, run, take the sound and the picture (the base board's API).
    system.board_mut().set_pressed(0, pads::RUN | pads::I);
    system.run_frame();
    let _samples: Vec<(i16, i16)> = system.board_mut().take_audio(); // 44,100 Hz stereo
    if let Some(frame) = system.board().capture.last_frame.as_ref() {
        let table = &system.board().vce.rgb_table;
        for row in &frame.rows {
            // 242 rows of 9-bit colours, already mixed by the VPC
            let _rgb: Vec<(u8, u8, u8)> = row.iter().map(|&value| table[value as usize]).collect();
        }
    }

    set_compat(&mut system, true); // the back-panel switch, moved while it runs
    let state: Vec<u8> = save_state(&system); // the base board's; the VPC is in it
    load_state(&mut system, &state)
}
```

- **Building**: `SuperGrafx::with_card(&bytes, compat)` (a 512-byte copier header is stripped; a
  2-button pad), then `.build()`, which gives the base board's `System`. For anything else (another
  pad, a port device, a power-on seed, a VPC with other switches) `SuperGrafx::new(SuperGrafxOptions
  { compat, vpc, st_steers_in_compat, board })`, `board` being the base's `BoardOptions`; its
  `machine` is replaced by the `SUPERGRAFX` row and its `pixel_path` must be `None`.
- **Everything after `build` is the base board's**: `run_frame`, `step`, `set_pressed`,
  `take_audio`, `capture.last_frame`, `vce.rgb_table`, `save_state`/`load_state`,
  `save_snapshot`/`load_snapshot` (pcengine-rust's README).
- **Compat mode**: `compat = true` is the switch on the back in PC Engine position: VDC 1 alone is
  decoded, mirrored every 4 bytes, and `ST0`-`ST2` are not steered. The mix still runs (with the VPC
  at its power-up values it shows VDC 1 only) and the machine keeps its 32 KiB. `set_compat` moves
  it later; `video_page(system.board_mut())` gives the page and its `vpc`.
- **Which machine for a card**: a HuCard has no header that says. `is_supergrafx_card(&bytes,
  Some(file_name))` is true for the five cards sold for the SuperGrafx alone (1941 Counter Attack,
  Aldynes, Battle Ace, Daimakaimura, Madou King Granzort), recognised by the SHA-1 of the image
  without a copier header (the dumps of the author's library, so another dump or a translation is
  not recognised by content), and for any file named `.sgx`. `supergrafx_title(&bytes)` names a
  known one. Everything else belongs on pcengine-rust's board. The five do not run correctly on the
  plain board: four of five are visibly wrong without VDC 2 (supergrafx-python's
  `docs/m6-record.md`).
- **Save states** made here are refused by a plain board and the other way round, with the system
  untouched.
- **wasm**: the library builds for `wasm32-unknown-unknown`; nothing in it touches a file, a clock
  or a thread outside `trace::SgxSetup::parse` and the binaries.

## How it is proven

**The gate** (`scripts/sgx_lockstep.py`) runs each case of `scripts/cases.tsv` on both sides: the
reference's `SuperGrafx` on PyPy (`scripts/sgx_trace.py`) and this crate's (`sgx-trace`). Each writes
one line a frame (the master clock, the count and SHA-1 of the CPU bus log, the SHA-1 of the mixed
picture and of the audio, every field of the CPU's state); a case passes when the two files are
identical, and a difference is narrowed to the first differing bus cycle. The frame line, the bus
tap and the comparison are pcengine-rust's scripts, imported by path from a checkout of it. The
library's first seconds reach little of the VPC, so `scripts/sgx_probe.py` builds a synthetic 8 KiB
card, this repository's own code and not a ROM of anything, that writes every VPC register on every
dot of a line; and a switch that changes no trace proves nothing, so `scripts/sgx_switch_effect.py`
fails if one does not. [docs/ladder.md](docs/ladder.md) has the cases, the switch table and the save
states; [docs/seeded-faults.md](docs/seeded-faults.md) the thirty-two faults and which check caught each.

```text
cargo test --release                      # rung 1: needs nothing (the Covell test is skipped without $COVELL)
scripts/fetch_reference.sh [SOURCE]       # supergrafx-python, pcengine-python, huc6280-python, 6502-python at their pins
scripts/rung1.sh
scripts/rung2.sh
scripts/rung3.sh --short                  # the probe's switch cases (seconds from the cache)
scripts/rung3.sh                          # the cards (about ten minutes of six PyPy processes, once)
scripts/rung4.sh
scripts/rung5.sh
scripts/rung6.sh                          # after a full gate run (it uses the reference cache)
```

What the rungs need:

- **The reference** on PyPy 3.11 (`$PYPY`): `scripts/fetch_reference.sh` clones the four
  repositories at their pins into `external/`; nothing is installed, the gate runs PyPy with
  `PYTHONPATH` over their `src/` directories. supergrafx-python is private at the time of writing,
  so rungs 2 to 6 need access to it; `SOURCE` may be a directory of local clones.
- **A checkout of pcengine-rust** (`$PCENGINE_RUST`, default `../pcengine-rust`) at the revision
  `Cargo.toml` names: the gate imports its `scripts/`.
- **The user's files**: the five SuperGrafx cards under their No-Intro names as `.zip` (`$LIB`,
  default `$NO_INTRO_DIR/NEC - PC Engine SuperGrafx`), extracted one at a time into the ignored
  `scratch/` directory and removed; and Covell's CPU test, `CPU_Test_10.pce` (`$COVELL`, default
  in a sibling pcengine-python checkout), which pcengine-python's `scripts/fetch_references.py`
  fetches into its `tests/test_roms/CPU_Test/`.
- **The reference cache**: each reference trace is kept in `external/refcache`, named by the case and
  a hash of its inputs, so a rerun costs the Rust side only.

`$PYPY` defaults to `pypy3.11` on `PATH`. The rungs' `cargo` needs rustc 1.85 or later.
`JOBS` sets the parallel cases (at most 8). CI (`.github/workflows/ci.yml`) runs `cargo fmt
--check`, clippy with `-D warnings`, `cargo test` and the wasm build, after cloning pcengine-rust and
huc6280-rust beside the checkout at the pinned commits (failing until those are pushed); the gate
needs the reference and the cards and runs by hand.

## Rust, speed, the reference, provenance

**The base board's VDC wait** (pcengine-rust 0.2.0) reaches both VDCs: the video page makes every
access it steers with `pins.vdc_read` / `vdc_write`, so an access that has to wait stops and the base
board makes it again once the chip is served.

**Where Rust forced a difference**, each module's header says so, and
[docs/rust-differences.md](docs/rust-differences.md) lists them together. The ones a host meets: the
reference's `Vpc` and `VideoPage` are one trait object here (the page is the board's `PixelPath` and
owns the `Vpc`); the board owns the VPC and the CPU owns the board, so `build` hands back the base
board's `System` and the devices are reached through it; the named switches are fields, and a save
state never carries them (the reference carries one set on an instance).

**Speed** is the fast fork's job; this is the figure to measure it against. From power-on, nothing
logged, 600 frames with a RUN press, one core (`sgx-bench`, `scripts/sgx_bench.py`):

| Card | supergrafx-rust | supergrafx-python on PyPy 7.3.20 (whole run; second half) |
| --- | --- | --- |
| Daimakaimura | 161-183 frames a second | 9.7; 10.2 |
| 1941 Counter Attack | 169-213 | 11.1; 10.5 |

About three times real time (59.8 frames a second), some 17 to 19 times the reference, on a machine
at load 13-15 (three Rust runs each). Two VDCs and a per-line mix cost about a third of the base
board's speed.

**The reference**: nothing found is a divergence. supergrafx-python `v0.1.1` changed only its base
board; the base board's own notes (pcengine-python issue 0004, settled) are pcengine-rust's README. Six places looked unintended or stale while it was
being transcribed ([docs/notes-on-the-reference.md](docs/notes-on-the-reference.md); not filed as an
issue in supergrafx-python). The one that shows: the reference's `board.reset` alone leaves the VPC
(only its `SuperGrafx.reset` resets it), while here `Board::reset` resets the VPC on power-on; no
rung resets a machine.

**Provenance**: no GPL or LGPL code was copied or transcribed. The only source transcribed is
supergrafx-python, which is ours and MIT; no emulator's source was opened. The rule and the check
that looks for copied code are pcengine-python's
[docs/provenance.md](https://github.com/alewman/pcengine-python/blob/main/docs/provenance.md). The 2026-11-13 audit's one unclear item
(the window origin and the both-transparent rule, inherited) was withdrawn unread and transcribed
again from the reference's clean rewrite: [docs/provenance.md](docs/provenance.md),
[docs/restart.md](docs/restart.md).

## Layout

`src/machine.rs`, `src/vpc.rs` and `src/board.rs` are the transcriptions of the `supergrafx_python`
modules of the same names. Not from the reference's package: `src/cards.rs` (the five cards),
`src/trace.rs` (a traced machine from a command line, `roundtrip` and `runahead`) and the binaries
`sgx-trace`, `sgx-state-roundtrip`, `sgx-bench`. Tests: `tests/vpc.rs`, `board.rs`, `core_st.rs`,
`gate.rs` and `state.rs` are the reference's `tests/test_*.py` of the same names. Not transcribed: the
reference's scripts that drive Mesen2, Beetle PCE and MAME (the reference has been judged against
those; this crate is judged against the reference).

[docs/handoff-brief.md](docs/handoff-brief.md) is the brief this was built from, with what was built
appended.

| What it was built and proven against | Version |
| --- | --- |
| supergrafx-python (the reference) | `83477041a8d4ce69b93ada7463d20d3b9481e50a` (2026-11-19, unreleased; 0.2.0 was proven against `v0.1.1`, `7bf3c52`; 0.1.0 against `v0.1.0`, `eee789a`) |
| pcengine-python, huc6280-python, 6502-python (what it runs on) | `03ac699`, `6ce348e`, `2146eba` (0.2.0: `v0.2.0` `4c32f26`, `v0.4.0` `265df88`) |
| pcengine-rust (the base board) | `8f672f0` by git revision (the code of `37f17ba`, by path until 2026-11-20, plus the base board's git pin of the core); 0.2.0 was `064c009` |
| huc6280-rust (through the base board) | `e1d24a3`, by the base board's git pin; 0.2.0 was `43e5978` |
| The SuperGrafx cards | the five No-Intro images (the SHA-1 of each in `src/cards.rs`; the same bytes as the `.7z` images 0.2.0 was gated on) |
| Covell's CPU test | `CPU_Test_10.pce` of pcengine-python's pinned `CPU_Test.zip` |

## Licence

MIT (see [LICENSE](LICENSE)). Nothing third-party is committed: the reference, the cards and Covell's
test are fetched or read in place, and `.gitignore` refuses ROM and disc image extensions.
