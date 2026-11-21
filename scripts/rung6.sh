#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 6: seeded faults. One single-line fault at a time, each to be caught by a rung
# (scripts/sgx_seeded_faults.py). Needs the reference cache a full gate run left.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
python3 "$ROOT/scripts/sgx_seeded_faults.py" --jobs "${JOBS:-6}" "$@"
echo "rung 6: PASS"
