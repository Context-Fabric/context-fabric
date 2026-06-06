# Context Fabric Golden Probes

Golden files in `golden/*.json` are Text-Fabric truth data. Regenerate them only with:

```sh
libs/core/.venv/bin/python libs/core/tests/golden/gen_golden.py bhsa n1904 banks
```

Do not generate committed golden files from `cf_rust_dump` or `cfabric`; those are the implementations under test. `compare.py` runs materialized Rust, mapped Rust, and Python extension probe execution against the same Text-Fabric-generated golden output.
