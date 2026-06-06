# cf-rust Golden Probes

Golden files in `golden/*.json` are Python truth data. Regenerate them only with:

```sh
libs/core/.venv/bin/python libs/cf-rust/tests/golden/gen_golden.py bhsa n1904 banks
```

Do not generate committed golden files from `cf_rust_dump`; that binary is the Rust implementation under test. `compare.py` runs both materialized and mapped Rust probe execution against the same Python-generated golden output.
