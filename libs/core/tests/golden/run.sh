#!/bin/sh
# Compare cfabric against the Text-Fabric golden files. Runs with the core
# package's virtual environment (override with PYTHON=...); corpora come from
# CFABRIC_CORPORA (default: libs/benchmarks/.corpora).
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PYTHON=${PYTHON:-"$HERE/../../.venv/bin/python"}
exec "$PYTHON" "$HERE/compare.py" "$@"
