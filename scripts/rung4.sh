#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 4: the gate of supergrafx-python's tests/test_gate.py: with VDC 2 idle and the VPC at its
# power-up values the add-on draws what the one-VDC board draws, on Covell's CPU test (read in
# place: $COVELL). Then the same card in lockstep with the reference in the fallback (base) mode
# and in PC Engine position, and Daimakaimura on the fallback.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
export COVELL=${COVELL:-$ROOT/../pcengine-python/tests/test_roms/CPU_Test/CPU_Test_10.pce}
[ -f "$COVELL" ] || { echo "rung 4: FAILED: no Covell CPU test at $COVELL" >&2; exit 1; }
nice cargo test --release -q --manifest-path "$ROOT/Cargo.toml" --test gate -- --nocapture 2>&1 | grep -E "compared|test result"
python3 "$ROOT/scripts/sgx_lockstep.py" --only '^(covell|gate-)' --jobs "${JOBS:-6}" \
    --reference-cache "$ROOT/external/refcache" "$@"
echo "rung 4: PASS"
