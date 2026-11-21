#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# The reference at its pins, into external/: supergrafx-python 8347704 (the clean rewrite of
# 2026-11-19, repinned to the clean base board and then to its early-display-start crop), the
# base board it pins (pcengine-python 03ac699), its CPU core (huc6280-python 6ce348e) and that core's frame (6502-python 2146eba).
# All four are on GitHub (the first three since 2026-11-20). No package is installed: the gate runs PyPy with PYTHONPATH on the
# four src/ directories (scripts/lockstep.py), so nothing is fetched but the four checkouts.
# Usage: scripts/fetch_reference.sh [SOURCE]
#   SOURCE: where the four repositories are, by name (default https://github.com/alewman;
#   a directory of local clones works: scripts/fetch_reference.sh ..). Each commit is
#   fetched from SOURCE, so local clones that have the commits work too.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SOURCE=${1:-https://github.com/alewman}
fetch() {  # name, the pinned commit (and the release it is)
    local dest="$ROOT/external/$1"
    if [ ! -d "$dest/.git" ]; then
        git clone -q "$SOURCE/$1" "$dest"
    fi
    git -C "$dest" fetch -q --tags "$SOURCE/$1" "$2" 2>/dev/null || true
    git -C "$dest" checkout -q "$2"
    [ "$(git -C "$dest" rev-parse HEAD)" = "$2" ] || { echo "$1: not at $2" >&2; exit 1; }
    echo "$1 ${3:-} ($2) in $dest"
}
mkdir -p "$ROOT/external"
fetch supergrafx-python 83477041a8d4ce69b93ada7463d20d3b9481e50a
fetch pcengine-python 03ac6998c812a291e5f91882552df5ae712ba870
fetch huc6280-python 6ce348e13c9ef3009bd07e7938c4b48956eeadc6
fetch 6502-python 2146eba3b3da4d2cc6c8fa3994f100bcb5abf039
