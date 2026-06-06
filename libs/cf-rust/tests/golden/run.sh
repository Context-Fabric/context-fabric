#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
python3 "$ROOT_DIR/libs/cf-rust/tests/golden/compare.py" "$@"
