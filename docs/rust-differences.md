<!-- SPDX-License-Identifier: MIT -->
# Where Rust forced a difference

Each module's header says so; together:

- **One trait object where the reference has two objects.** The reference's `Vpc` is the pixel path
  and its `VideoPage` claims the pages; they share state by reference, and the page holds both VDCs.
  Here `VideoPage` is the board's `PixelPath` (which in pcengine-rust may also claim pages), owns the
  `Vpc` (`page.vpc`) and reaches the VDCs through `pins.vdcs`, handed in with every access.
- **The base board's VDC wait** (pcengine-python issue 0004, pcengine-rust 0.2.0). The reference's
  VDC calls the board's `_serve_vram` from inside an access the page steered to it, and the board
  then runs every device, the page's own `mix` included. A Rust device cannot be called again in the
  middle of its access, so the page makes each VDC access with `pins.vdc_read` / `vdc_write`: an
  access that has to wait stops there, the base board serves the chip and makes the page's access
  again (pcengine-rust's README, "Where Rust forced a difference"). The page has nothing of its own
  that an access changes, so making it again is exact.
- **`store_immediate` is `pins.opcode`.** The reference's page calls back into the `SuperGrafx`
  object, which reads the core's `opcode` (with a fallback to a private attribute for an older core).
  The base Rust board hands the opcode of the instruction being run to the device with each access;
  with no CPU it is 0, where the reference answers `False`.
- **The board owns the VPC and the CPU owns the board**, so `SuperGrafx` cannot keep `vpc`, `page`,
  `vdc1` and `vdc2` beside the board. They are `board.vdcs[0]`, `board.vdcs[1]` and
  `video_page(&mut board)` (the page, and `page.vpc`); `build` consumes the `SuperGrafx` and gives
  the `System`, after which they are reached through `system.board_mut()`.
  `set_compat(&mut system, bool)` moves the switch of a running machine.
- **Reset** is one call: `Board::reset` calls the pixel path's `reset`, which resets the VPC on
  power-on. The reference's `SuperGrafx.reset` does the two itself, and its `board.reset` alone
  leaves the VPC as it was ([notes on the reference](notes-on-the-reference.md), 1).
- **Switches** are fields: `Vpc::priority_labels` (the enum `PriorityLabels::{Detectors, Sgxtech}`
  for the two strings), `Vpc::window_origin` (`WindowOrigin { divider4, divider3, divider2 }` for the
  dict by dot divider), `Vpc::window_off_at_or_below`, `VideoPage::st_steers_in_compat`. The
  constructor's keyword arguments are `SuperGrafxOptions { compat, vpc, st_steers_in_compat, board }`,
  `board` being the base's `BoardOptions` (the reference's `**board_kwargs`).
- **Save states**: the page saves itself through the base board's `DeviceState` (bincode of the
  registers, the line's shadows, the pending write log and the switch's position), where the
  reference's state walks the object graph. As there, the position of the back-panel switch is in a
  state and comes back with it. The named switches are not: a loaded state keeps the system's own
  (the base Rust board's rule; in the reference a switch set on the instance travels with the state
  and one set on the class does not).
- **Python's arithmetic**: the mid-line dot is `-((first - when) // div)`, a ceiling by floor
  division of a negative, spelled with the base's `py_floordiv`; window edges are `i64` so that a
  switch that makes one negative behaves as in Python.
- **The unit tests of ST steering.** The reference's test double sets the opcode with each write. The
  base Rust board takes the opcode from the core's `Pins`, which nothing outside the core can set, so
  those tests drive the page with `sockets::DetachedPins` (two VDCs and an opcode, no board) and,
  through the board, the real core (`tests/core_st.rs`).
- **Not carried over**: the fallback from `cpu.opcode` to `cpu._opcode` (one core here) and its
  test; `register_package` (a state's parts are named by the trait).
- **Added**: `cards.rs` (which images are SuperGrafx cards: a host needs to know), `trace.rs` and the
  three binaries (the gate), `SuperGrafx::with_card`, `video_page`, `set_compat`.

Two things in pcengine-rust would have saved code here, neither needed: the base's
`trace::roundtrip` and `runahead` build their own boards from a `Setup`, so they are repeated in
`src/trace.rs` over a machine built here (a builder argument would let add-ons call them); and the
opcode a device sees cannot be set by a test (`Pins::opcode` and `Board::opcode` are private), hence
`DetachedPins` above.
