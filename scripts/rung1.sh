#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 1: the unit tests. supergrafx-python's tests/test_vpc.py, test_board.py, test_core_st.py
# (and test_state.py, test_gate.py, which rungs 4 and 5 run again by name) transcribed: the
# decode, the mirrors, the gap, the windows, each priority code under both label readings, the
# ST steering, compat mode, reset.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=$(nice cargo test --release --manifest-path "$ROOT/Cargo.toml" 2>&1) || { printf '%s\n' "$OUT" | tail -n 40; echo "rung 1: FAILED" >&2; exit 1; }
printf '%s\n' "$OUT" | grep -E "Running|test result" 
TOTAL=$(printf '%s\n' "$OUT" | sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' | paste -sd+ | bc)
echo "rung 1: PASS ($TOTAL tests)"
