<!-- SPDX-License-Identifier: MIT -->
# Changelog

Not on crates.io; pin a commit (README, "Using it").

## Unreleased (2026-11-19): the clean rewrite

- **Pinned by git revision (2026-11-20)**: the base board is pcengine-rust `8f672f0` by git
  revision, not a path to a sibling clone (`37f17ba`'s code; the base board now takes
  huc6280-rust `e1d24a3` by git pin too, which a git dependency needs). `Cargo.toml` names the
  `--config` patch for a local checkout. CI no longer clones the base board and the core beside
  the checkout. The gate's scripts still import the base board's tooling from a checkout
  (`$PCENGINE_RUST`, default `../pcengine-rust`).
- **The base board's fix carried**: pcengine-rust `37f17ba` (pcengine-python `03ac699`: a display
  that starts before the row is cropped, so the row keeps its length; it panicked on the probe card
  in compat and base mode) and supergrafx-python `8347704` (repinned to it). Lockstep 22 of 22,
  save states 8 of 8, seeded faults 34 caught, 0 missed without `--known-failing`. The reference
  cache key now includes the reference checkouts' commits (`scripts/sgx_lockstep.py`).

Follows supergrafx-python `f04d648` (its clean rewrite `5c5ada6` and the repin to the clean base
board) on pcengine-python `3318027` and huc6280-python `6ce348e`. [docs/restart.md](docs/restart.md).

- **Window origin** `Vpc::window_origin` default `{24, 38, 24}` for dividers 4, 3, 2 (was
  `{32, 46, 32}`): the reference re-measured it on the clean base board, whose picture starts 8
  dots earlier in the row (closed-box measurement, emulator tier; 7.16 MHz provisional).
- **Clean rewrite**: `WindowOrigin`, `window_edge`, `mix_run`, the module documentation and the
  C-SGX1 / U-SGX3 comments moved aside unread and transcribed again from the reference's new
  `vpc.py`; no other behaviour change. Provenance: unclear 1 -> 0.
- **Base board: pcengine-rust `26ea621`** (its clean VDC rewrite; huc6280-rust `e1d24a3`), by path
  from a sibling clone until both were pushed (a git pin since 2026-11-20, above). Save states are the base's format version 3; 0.2.0 states are refused.
- Gate: `--vertical-at-line-start` gone, `--vblank-dots` default `-16` on both sides;
  `sw-origin-probe` with `WINDOW_ORIGIN=40,40,40`; the cards read as No-Intro `.zip` under
  `$NO_INTRO_DIR` (or `$LIB`), no machine paths in scripts or tests.
- Seeded faults: the seeder copies the README into its tree (the doctest had failed every faulted
  build) and checks the unfaulted tree first; the seven faults on withdrawn lines rewritten, two
  added.
- SPDX headers on every source, test, script and document file.

## 0.2.0 (2026-11-13)

Follows supergrafx-python 0.1.1 (`7bf3c52`), whose only change is its base board:
pcengine-python 0.2.0 (`4c32f26`) with huc6280-python 0.4.0 (`265df88`).

- **Base board: pcengine-rust 0.2.0** (by its GitHub revision, `064c009`). It
  brings huc6280-rust 0.2.0 (`CpuState::polled_irq_lines`) and pcengine-python's issue 0004: the
  board runs its own events while the CPU waits on a VDC access. On this machine that wait reaches
  both VDCs.
- **The video page reaches the VDCs through the base board's wait**: every access it steers is
  made with `pins.vdc_read` / `vdc_write` (the base board's new `BoardPins` methods) instead of
  calling `Vdc::read` / `write` itself, so that an access that has to wait stops and is made again
  once the board has served it (`vpc.rs`).
- Save states are the base board's: format version 2; a state of supergrafx-rust 0.1.0 is refused.
- Two new seeded faults (a waiting VDC access that misses the wait).

## 0.1.0 (2026-11-12)

The SuperGrafx transcribed from supergrafx-python v0.1.0 (`eee789a`); all six rungs pass.
