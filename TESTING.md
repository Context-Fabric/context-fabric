# Testing

Context-Fabric is a monorepo of three packages, each with its own test suite:

| Package | Path | Suites |
|---|---|---|
| `context-fabric` (Rust core + Python overlay) | `libs/core` | Rust (`cargo test`), Python (`tests/python`), parity (`tests/parity`) |
| `cfabric-mcp` | `libs/mcp` | `tests/` (unit + `integration/`) |
| `cfabric-benchmarks` | `libs/benchmarks` | `tests/` (`unit/` + `integration/`) |

## Setup

Python 3.13+ and a stable Rust toolchain. Each package uses its own virtual
environment; the core extension is built into it with maturin:

```bash
cd libs/core
python -m venv .venv
.venv/bin/pip install maturin
.venv/bin/pip install -e ".[dev]"          # or: .venv/bin/maturin develop --release
```

The MCP and benchmark packages install the core from source
(`pip install -e ../core`) followed by `pip install -e ".[dev]"`.

## Rust core (`libs/core`)

```bash
cargo fmt --check
cargo clippy --all-targets --features python -- -D clippy::correctness -D clippy::suspicious
PYO3_PYTHON="$PWD/.venv/bin/python" cargo test --features python
```

Integration tests live in `libs/core/tests/*.rs`: `core_port.rs` (engine
behaviour), `compiled_robustness.rs` (corrupted / truncated / concurrently
replaced `.cfr` caches), `cache_freshness.rs` and `cache_readonly_fallback.rs`
(cache invalidation and placement), plus locality and performance checks.

## Python (`libs/core/tests/python`)

```bash
.venv/bin/python -m pytest tests/python
```

Most tests use the small fixture corpora in `libs/core/tests/fixtures`. Tests
that need a real corpus (BHSA, N1904, ...) skip when it is unavailable; the
shared fixtures in `libs/core/tests/conftest.py` locate them:

- `CFABRIC_CORPORA`: directory holding `<name>/tf` corpora (default
  `libs/benchmarks/.corpora`).
- `CFABRIC_BHSA_CFR`: an explicit compiled BHSA cache to open. By default BHSA
  is loaded with `loadAll`, which builds and reuses the cache that `Fabric`
  keeps next to the sources.

## Parity with Text-Fabric (`libs/core/tests/parity`)

The parity suites compare Context-Fabric with Text-Fabric 13.0.19 on real
corpora and are opt-in:

```bash
.venv/bin/pip install "git+https://github.com/annotation/text-fabric@v13.0.19"
CONTEXT_FABRIC_RUN_PARITY=1 .venv/bin/python -m pytest tests/parity
```

## MCP and benchmarks

```bash
cd libs/mcp && pytest tests/
cd libs/benchmarks && pytest tests/
```

Tests never write into the source tree: fixtures that get compiled are copied
to a temporary directory first, and `.cfr/` caches are git-ignored.

## Distributions

`.github/workflows/core-wheels.yml` builds an abi3 wheel per platform with
cibuildwheel (configured under `[tool.cibuildwheel]` in
`libs/core/pyproject.toml`) and runs `tests/python` against each installed
wheel; it also builds the sdist, builds a wheel from it, and tests that. To
reproduce the Linux build locally (requires Docker):

```bash
pip install "cibuildwheel>=3,<4"
cibuildwheel --platform linux libs/core --output-dir wheelhouse
```
