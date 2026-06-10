"""34-query output-parity gate against Text-Fabric golden baselines.

Each query from ``.claude/parity_queries.json`` is executed through the
context-fabric mapped search engine (``Fabric(locations=...).openMapped(cache)``)
in an isolated subprocess so that a per-query timeout can be enforced with a hard
kill (native search code does not reliably honour in-process signals).

For every query we assert:

* ``len(results) == tf_count``
* each tuple in ``tf_first_results`` is a member of the result set.

The suite is gated behind ``CONTEXT_FABRIC_RUN_PARITY=1`` (mirroring
``test_tf_oracle_parity.py``) and skipped if the compiled ``.cfr`` cache is
missing.

Currently-failing queries are marked ``xfail`` with a ``bug:`` reason tag so the
suite runs green today and ratchets as Phase 1-3 fixes land. ``strict=False`` is
used so that an unexpected pass surfaces as ``xpass`` (visible) rather than a
failure.

Environment overrides:

* ``CFQ_TIMEOUT``      per-query timeout in seconds (default 60).
* ``CFABRIC_BHSA_CFR`` path to the compiled cache (default ``/tmp/cf_bhsa.cfr``).
* ``CFABRIC_BHSA_TF``  path to the BHSA TF source dir (default repo corpora).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

pytestmark = pytest.mark.skipif(
    os.environ.get("CONTEXT_FABRIC_RUN_PARITY") != "1",
    reason="set CONTEXT_FABRIC_RUN_PARITY=1 to run Text-Fabric query parity tests",
)

ROOT = Path(__file__).resolve().parents[4]
QUERIES_JSON = ROOT / ".claude" / "parity_queries.json"
TF_DIR = Path(
    os.environ.get(
        "CFABRIC_BHSA_TF",
        str(ROOT / "libs" / "benchmarks" / ".corpora" / "bhsa" / "tf"),
    )
)
CACHE = Path(os.environ.get("CFABRIC_BHSA_CFR", "/tmp/cf_bhsa.cfr"))
TIMEOUT = float(os.environ.get("CFQ_TIMEOUT", "60"))

# ---------------------------------------------------------------------------
# Known current outcomes (June 2026 evaluation, cfabric 0.6.0rc1 on BHSA).
# Mapped engine baseline from /tmp/cf_query_parity/mapped_progress.jsonl.
# Reason tags map each defect to its Phase 1 remediation task.
#   bug:quantifier-perf      -> T1.4 quantifier set-algebra rewrite (timeouts)
#   bug:edge-relation-perf   -> T1.3/T1.5 edge-relation cross-products (timeouts)
#   bug:op-prefix-embedding  -> T1.3 operator-prefixed atom two-edge semantics
#   bug:knear-zero           -> T1.5 k-near sibling adjacency (wrong-zero)
#   bug:overlap-parent-zero  -> T1.3/T1.4 && overlap vs parent in /with/ (zero)
#   bug:tokenizer-trailing-eq-> T1.1 constraint tokenizer (vbe#H= parse crash)
# Indices not listed here are expected to PASS today.
# ---------------------------------------------------------------------------
XFAIL_REASONS: dict[int, str] = {
    0: "bug:quantifier-perf",        # /with//or/ + <:        TIMEOUT
    1: "bug:quantifier-perf",        # /where//have//without/ TIMEOUT
    2: "bug:quantifier-perf",        # /where//have/ + ..     TIMEOUT
    4: "bug:edge-relation-perf",     # -mother>               TIMEOUT
    5: "bug:edge-relation-perf",     # -mother> named/#       TIMEOUT
    6: "bug:edge-relation-perf",     # <mother-               TIMEOUT
    7: "bug:op-prefix-embedding",    # =: := + .nu.           TIMEOUT
    8: "bug:op-prefix-embedding",    # =: := + .nu#nu.        TIMEOUT
    9: "bug:op-prefix-embedding",    # := =: <: .lex.         CRASH
    10: "bug:op-prefix-embedding",   # <20: + <               TIMEOUT
    11: "bug:knear-zero",            # :30>                   WRONG-ZERO
    18: "bug:op-prefix-embedding",   # =: :=                  TIMEOUT
    20: "bug:op-prefix-embedding",   # =: + ##                TIMEOUT
    21: "bug:op-prefix-embedding",   # < subphrase + ||       TIMEOUT
    22: "bug:overlap-parent-zero",   # /with/ && ..           WRONG-ZERO
    23: "bug:op-prefix-embedding",   # <:                     TIMEOUT
    24: "bug:op-prefix-embedding",   # :>                     TIMEOUT
    25: "bug:op-prefix-embedding",   # deep nest + <:         WRONG-ZERO
    26: "bug:op-prefix-embedding",   # =: :=                  TIMEOUT
    27: "bug:op-prefix-embedding",   # <: < ||                TIMEOUT
    28: "bug:op-prefix-embedding",   # =: < > || [[           TIMEOUT
    29: "bug:op-prefix-embedding",   # << + <:                WRONG-ZERO
    30: "bug:quantifier-perf",       # /without/ + <<         TIMEOUT
    31: "bug:op-prefix-embedding",   # := word + numeric      CRASH
    32: "bug:quantifier-perf",       # /without/ + ]]         TIMEOUT
    33: "bug:tokenizer-trailing-eq", # vbe#H=                 CRASH
}


def _load_queries() -> list[dict[str, Any]]:
    with open(QUERIES_JSON) as handle:
        return json.load(handle)["queries"]


QUERIES = _load_queries()
QUERY_BY_ID = {q["id"]: q for q in QUERIES}


# Child program: open the mapped corpus, run one query, report membership of the
# golden first-result tuples plus per-column otypes for the within-tuple order
# check.  Kept self-contained so it can be executed via ``python -c`` with a JSON
# payload on stdin from a non-repo cwd.
_CHILD = r"""
import sys, json
payload = json.load(sys.stdin)
from cfabric import Fabric
api = Fabric(locations=payload["locations"]).openMapped(payload["cache"])
F = api.F
res = [tuple(int(x) for x in row) for row in api.S.search(payload["template"])]
seen = set(res)
first = [tuple(int(x) for x in t) for t in payload["first_results"]]
out = {
    "count": len(res),
    "present": [t in seen for t in first],
    "first_tuple": list(res[0]) if res else None,
    "first_tuple_otypes": [F.otype.v(n) for n in res[0]] if res else None,
}
exp = payload.get("expected_otypes")
if exp is not None:
    out["order_ok"] = all([F.otype.v(n) for n in row] == exp for row in res)
sys.stdout.write("@@CFRESULT@@" + json.dumps(out) + "\n")
"""

_MARKER = "@@CFRESULT@@"


def _run_query(
    template: str,
    first_results: list[Any],
    *,
    expected_otypes: list[str] | None = None,
    timeout: float = TIMEOUT,
) -> dict[str, Any]:
    """Run a single query in an isolated subprocess with a hard timeout."""
    payload: dict[str, Any] = {
        "locations": str(TF_DIR),
        "cache": str(CACHE),
        "template": template,
        "first_results": [list(t) for t in first_results],
    }
    if expected_otypes is not None:
        payload["expected_otypes"] = expected_otypes
    proc = subprocess.run(
        [sys.executable, "-c", _CHILD],
        input=json.dumps(payload),
        text=True,
        capture_output=True,
        cwd="/tmp",  # importing cfabric from the repo root breaks
        timeout=timeout,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"child exited {proc.returncode}\nstderr:\n{proc.stderr[-2000:]}"
        )
    for line in proc.stdout.splitlines():
        if line.startswith(_MARKER):
            return json.loads(line[len(_MARKER) :])
    raise RuntimeError(
        f"no result marker in child output\n"
        f"stdout:\n{proc.stdout[-1000:]}\nstderr:\n{proc.stderr[-1000:]}"
    )


def _require_cache() -> None:
    if not CACHE.exists():
        pytest.skip(f"missing compiled cache at {CACHE}")
    if not (TF_DIR / "otype.tf").exists():
        pytest.skip(f"missing BHSA TF source at {TF_DIR}")


def _query_params() -> list[Any]:
    params = []
    for idx, query in enumerate(QUERIES):
        marks = []
        if idx in XFAIL_REASONS:
            marks.append(
                pytest.mark.xfail(reason=XFAIL_REASONS[idx], strict=False)
            )
        params.append(pytest.param(idx, query, marks=marks, id=query["id"]))
    return params


@pytest.mark.parametrize("idx,query", _query_params())
def test_query_parity(idx: int, query: dict[str, Any]) -> None:
    _require_cache()
    out = _run_query(query["template"], query["tf_first_results"])
    assert out["count"] == query["tf_count"], (
        f"{query['id']}: count {out['count']} != tf_count {query['tf_count']}"
    )
    missing = [
        t
        for t, ok in zip(query["tf_first_results"], out["present"])
        if not ok
    ]
    assert not missing, f"{query['id']}: golden tuples absent from results: {missing}"


# ---------------------------------------------------------------------------
# Within-tuple column order: column i of every result tuple must carry the
# otype of the i-th atom in the template (within-tuple order == template atom
# order).  Verified on two passing multi-atom queries.
# ---------------------------------------------------------------------------
ORDER_CASES = [
    # template atoms in order:  clause typ~^W[xX]  /  word sp=verb vs#qal
    ("q_regex_typ", ["clause", "word"]),
    # template atoms in order:  v:verse  /  s:sentence
    ("q_slot_equal", ["verse", "sentence"]),
]


@pytest.mark.parametrize("qid,expected", ORDER_CASES, ids=[c[0] for c in ORDER_CASES])
def test_within_tuple_order(qid: str, expected: list[str]) -> None:
    _require_cache()
    query = QUERY_BY_ID[qid]
    out = _run_query(
        query["template"], query["tf_first_results"], expected_otypes=expected
    )
    assert out["count"] == query["tf_count"], f"{qid}: unexpected count"
    assert out["first_tuple_otypes"] == expected, (
        f"{qid}: first tuple otypes {out['first_tuple_otypes']} != {expected}"
    )
    assert out["order_ok"], (
        f"{qid}: some result tuple has columns out of template atom order"
    )
