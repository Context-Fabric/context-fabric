#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
python3 "$ROOT_DIR/libs/core/tests/golden/compare.py" "$@"
