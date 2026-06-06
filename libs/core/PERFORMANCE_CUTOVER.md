# Context-Fabric 0.6.0rc1 Cutover Performance

Measurements from the Rust-core/PyO3 cutover on 2026-06-06 using the local BHSA corpus at
`libs/benchmarks/.corpora/bhsa/tf`.

## Wheel vs Raw Rust

Command:

```sh
python scripts/compare_bhsa_mapped_wheel_overhead.py \
  --tf-path ../benchmarks/.corpora/bhsa/tf \
  --cache-path /tmp/context-fabric-bhsa-wheel.cfr \
  --limit 5
```

Result:

- Mapped load overhead: `1.103x` (`43.281 ms` wheel, `39.222 ms` raw Rust)
- Mapped query geomean overhead over 104 shared queries: `0.993x`

## Wheel vs Text-Fabric

Mapped cache load measurement:

- Text-Fabric load: `4427.677 ms`
- Context-Fabric mapped open: `40.896 ms`
- Load speedup: `108.267x`
- RSS ratio: `0.027`

Latency benchmark:

```sh
python -m cfabric_benchmarks.cli latency \
  --corpus bhsa \
  --corpora-dir libs/benchmarks/.corpora \
  --queries 104 \
  --iterations 1 \
  --runs 1 \
  --output-dir /tmp/context-fabric-latency-gate
```

Result:

- Validated queries: `100/100`
- Query geomean speedup: `7.865x`
