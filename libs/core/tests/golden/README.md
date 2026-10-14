# Golden probes

Each probe in `probes/<corpus>.jsonl` is one API call (`kind` + `args`, executed by
`probes.py`); `golden/<corpus>.json` holds Text-Fabric's answer to every probe.

Compare cfabric with the golden files (core virtual environment; corpora from
`CFABRIC_CORPORA`, default `libs/benchmarks/.corpora`):

```sh
libs/core/tests/golden/run.sh            # bhsa n1904 banks
libs/core/tests/golden/run.sh bhsa
```

Regenerate the golden files only from Text-Fabric, never from cfabric (the
implementation under test):

```sh
libs/core/.venv/bin/python libs/core/tests/golden/gen_golden.py bhsa n1904 banks
```
