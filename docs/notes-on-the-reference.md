<!-- SPDX-License-Identifier: MIT -->
# Notes on the reference

Nothing found is a divergence: this crate does what supergrafx-python `v0.1.0` does in each case
(with the one exception in note 1, which no rung exercises). They are places where the reference
looked unintended or stale while it was being transcribed, left for its maintainers; they have not
been filed as an issue in supergrafx-python.

**2026-11-13, supergrafx-python `v0.1.1`**: the only change is the base board, pcengine-python
`v0.2.0` (its issue 0004 settled: the board runs its own events while the CPU waits on a VDC, the
PSG's level table follows `silent_units`) with huc6280-python `v0.4.0` (its issue 0005 settled). The
notes below are about this add-on and stand; none of them was changed upstream.

1. **`board.reset` alone leaves the VPC.** `SuperGrafx.reset(POWER_ON)` resets the board and then the
   VPC, but the base board's own `Board.reset` (which a host holding `sg.board`, or `System`, would
   call) knows nothing of the wrapper, so a power-on through it keeps the VPC's registers. Here there
   is one path (`PixelPath::reset`, called by `Board::reset`), so `board.reset(PowerOn)` does what the
   reference's `SuperGrafx.reset` does: the one place a call on the Rust board does more than the same
   call on the reference's. No rung's case resets a machine.
2. **A switch set on the instance travels in a save state; one set on the class does not.** The
   reference's `scripts/sgx_ours.py --vpc-option` uses `setattr` on the `Vpc` instance, and
   `sg.page.ST_STEERS_IN_COMPAT = True` (its test) sets an instance attribute: both are then walked
   into a state and overwrite the loading machine's, while a subclass's or the class's value is not
   saved. Here a state never carries a switch.
3. **`WINDOW_OFF_AT_OR_BELOW` moves the threshold but not the zero of the width**: `window_edge`
   still subtracts `0x40`, so with the switch below `0x40` a width between the two gives an edge left
   of the origin (or a negative one, which then means no window). Probably intended, since the two
   are separate entries (U-SGX1, U-SGX2); noted because the names read as one quantity.
4. **`_replay`'s last branch** takes any register that is not one of the other five as window 2's
   MSB. Only six registers are ever logged, so it is not reachable with another; it reads as if `$0E`
   could arrive there.
5. Stale text: `SuperGrafx._store_immediate`'s docstring says the pinned core only keeps the private
   `_opcode` (the pinned huc6280-python `v0.3.0` has the public `opcode`, as the reference's
   `docs/start-here.md` says); `tests/test_board.py` calls register 8 "BXR" in one comment (it is BYR;
   register 7 is BXR).
6. (Settled upstream 2026-11-19: the reference's `test_both_transparent_shows_vdc2s_pixel`, transcribed in `tests/vpc.rs`.) The reference had no test of U-SGX5 (two transparent pixels show VDC 2's), and no card or case of
   its own tells it apart: the two VDCs' transparent values differ only where one is in its border and
   the other is not. The seeded fault for it was missed here until the probe gave VDC 2 a narrower
   display and a unit test was added (`tests/vpc.rs`, `two_transparent_pixels_show_vdc2s`).
7. **A display that starts before the row (base board, found 2026-11-19; fixed upstream in pcengine-python `03ac699`, carried as pcengine-rust `37f17ba`).**
   On the clean base board (pcengine-python `3318027`, carried as pcengine-rust `26ea621`), the
   probe card in PC Engine position (`sw-compat-probe`, `probe-compat-at`) and on the fallback
   (`sw-base-probe`) reaches a line whose display starts before the row's first dot:
   `Vdc.render_line`'s `first = (seq.active_start - row_start) // seq.divider` is negative (-14 at
   divider 4 in frame 102 of `sw-compat-probe`; -55 at divider 3 in the round trip). The
   reference's `row[first:end] = pixels[: end - first]` then takes `first` as counted from the
   row's end: the slice is empty and the assignment inserts the pixels, so the row grows from 341
   to 682 dots. Its comment ("first <= dots") assumes `first >= 0`. pcengine-rust's transcription
   indexes with `first as usize` and panics (`src/vdc/chip.rs:353`). The SuperGrafx add-on is not
   involved (the fallback has no VPC); the probe's registers simply reach it. Repro on the
   reference: wrap `pcengine_python.vdc.chip.Vdc.render_line` to report `len(self._row) != 1365 //
   seq.divider`, build the probe card's machine with `scripts/sgx_trace.py`'s `make` (`--mode
   compat`, `scripts/sgx_probe.py`'s card) and run 103 frames: frame 102 reports a 682-dot row.
   **Fixed upstream and carried (2026-11-19):** pcengine-python `03ac699` draws such a line into
   a left-padded work row (the sprites still see every display column), then drops the pad, so the
   row keeps 1365 // divider dots (its test
   `test_a_display_that_starts_before_the_row_is_cropped_and_the_row_keeps_its_length`);
   pcengine-rust `37f17ba` transcribes it with the matching unit test. supergrafx-python `8347704`
   pins `03ac699`, and this crate takes pcengine-rust `37f17ba`. The three cases now pass (lockstep
   22 of 22, round trips 8 of 8) and the seeded-fault run needs no `--known-failing`. The base
   board's library never reaches the case (pcengine-python checked all 179 cards: its
   `docs/restart.md`); only this probe card does, in compat and base mode (4 lines, `first` = -14).
