# Testing

This document describes how to run tests for Context-Fabric.

## Prerequisites

1. Python 3.13+
2. Virtual environment with dev dependencies:

```bash
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

## Running Tests

### All Tests

Run the complete test suite (unit + integration):

```bash
PYTHONPATH=packages pytest tests/
```

### Unit Tests Only

```bash
PYTHONPATH=packages pytest tests/unit_tests/
```

### Integration Tests Only

```bash
PYTHONPATH=packages pytest tests/integration_tests/
```

### With Verbose Output

```bash
PYTHONPATH=packages pytest tests/ -v
```

### With Coverage Report

```bash
PYTHONPATH=packages pytest tests/ --cov=packages/core --cov-report=term-missing
```

## Using tox

Run tests in isolated environments:

```bash
# Run all test environments
tox

# Run specific environment
tox -e py313    # Python tests
tox -e lint     # Linting
tox -e format   # Check formatting
```

## Test Structure

```
tests/
├── fixtures/
│   └── mini_corpus/       # Test data (TF files)
├── unit_tests/            # 478 unit tests
│   ├── conftest.py
│   ├── test_*.py
│   └── search/
│       └── test_*.py
└── integration_tests/     # 87 integration tests
    ├── conftest.py
    └── test_*.py
```

## Writing Tests

- Unit tests use mocks and test individual functions/classes
- Integration tests use real TF data from `tests/fixtures/mini_corpus/`
- Use `loaded_api` fixture for integration tests that need a loaded API
