<!-- SPDX-License-Identifier: MIT -->
# Provenance

## Licence provenance (audit 2026-11-13)

supergrafx-rust is a transcription of supergrafx-python (ours, MIT) and **inherits its verdicts**
(supergrafx-python `docs/provenance.md`, same date; audited from the records, no GPL source opened):

| Rust module | Verdict |
| --- | --- |
| `src/vpc.rs`: registers, ST steering, the mix, `PRIORITY_LABELS` (closed-box confirmed) | **clean**; 2026-11-19: the C-SGX1 and U-SGX3 comments withdrawn unread and rewritten from the reference's new wording (no GPL emulator's internals named) |
| `src/vpc.rs` `WindowOrigin`, `window_origin`, `window_edge`, `mix_run` and the both-transparent rule (U-SGX5) | was **unclear**; 2026-11-19: withdrawn unread and transcribed again from supergrafx-python `f04d648` ([restart.md](restart.md)). Form from sgxtech (DOC); origin `{4: 24, 3: 38, 2: 24}` a closed-box measurement (emulator tier, Beetle PCE b96c11e, Mesen2 b9fa69d); U-SGX5 a closed-box measurement on both, MAME `huc6202.cpp` (BSD-3-Clause, read upstream, nothing ported) the same. Now **clean** (closed-box measurement; 7.16 MHz split) |
| `src/board.rs`, `src/machine.rs`, `src/cards.rs` | **clean** |

Added by the Rust tier: `src/trace.rs`, `src/bin/*`, `scripts/*` (lockstep, faults, benches),
`tests/`: ours, **clean**. Crates (Cargo.lock) are the base board's set, all permissive.

Counts (2026-11-19): clean 4, cite 0, rewrite 0, unclear 0. Before (2026-11-13): clean 3,
unclear 1. Both VDCs are pcengine-rust's, whose line machine was marked **rewrite** there and was
rewritten clean (pcengine-rust `26ea621`, from pcengine-python `3318027`), the revision this
crate pins since 2026-11-19. The seven seeded faults that held lines of the withdrawn code were
withdrawn with it and written anew (restart.md). provenance_check (2026-11-13, permissive trees) found nothing here.
History: 8 commits, no deleted files.
