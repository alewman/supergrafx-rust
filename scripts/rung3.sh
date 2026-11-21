#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Rung 3: the five library cards, 600 frames each with a RUN press; Daimakaimura into its first
# stage (run@400+10 run@700+10, to frame 1800), the same under PRIORITY_LABELS = sgxtech, and in
# PC Engine position for 300 frames; every other named switch away from its default on the
# probe card and, the window switches, on Daimakaimura's first stage. Then
# scripts/sgx_switch_effect.py: each switch must change a case it is on.
# `--short` runs only the probe's cases (seconds with the reference cache).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ONLY='^(lib-|dmm-|sw-)'
if [ "${1:-}" = "--short" ]; then
    shift
    ONLY='^sw-.*-probe$'
fi
python3 "$ROOT/scripts/sgx_lockstep.py" --only "$ONLY" --jobs "${JOBS:-6}" \
    --reference-cache "$ROOT/external/refcache" "$@"
if [ "$ONLY" != '^sw-.*-probe$' ]; then
    python3 "$ROOT/scripts/sgx_lockstep.py" --only '^(probe|gate-dmm-base)$' --quiet \
        --reference-cache "$ROOT/external/refcache"
    python3 "$ROOT/scripts/sgx_switch_effect.py"
fi
echo "rung 3: PASS"
