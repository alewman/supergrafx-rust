#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 5: save states. The base board's save_state/load_state and the snapshot pair over this
# machine: the unit tests (tests/state.rs: the VPC, the page and the second VDC come back; a
# state made here is refused by a plain board), then the round trip and the run-ahead pass on
# the five cards, on Daimakaimura in stage 1 and in PC Engine position, and on the probe card
# with the switch moved (scripts/sgx_state_roundtrip.py).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
nice cargo test --release -q --manifest-path "$ROOT/Cargo.toml" --test state 2>&1 | grep -E "test result"
python3 "$ROOT/scripts/sgx_state_roundtrip.py" --jobs "${JOBS:-6}" "$@"
echo "rung 5: PASS"
