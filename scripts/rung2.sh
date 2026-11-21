#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 2: the gate itself. scripts/sgx_trace.py runs the Python SuperGrafx (PyPy, the checkouts
# of scripts/fetch_reference.sh) and target/release/sgx-trace this crate; scripts/sgx_lockstep.py
# compares every frame's CPU bus log, mixed picture, audio and CPU state and narrows a
# difference to its first bus cycle. Here on Covell's CPU test and on the probe card
# (scripts/sgx_probe.py), which drives every VPC register through the frame.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
python3 "$ROOT/scripts/sgx_lockstep.py" --only '^(covell|probe)$' --jobs "${JOBS:-6}" \
    --reference-cache "$ROOT/external/refcache" "$@"
echo "rung 2: PASS"
