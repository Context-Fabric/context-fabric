#!/usr/bin/env python3
"""Run the BHSA mapped-subset query timings against Python cfabric."""

from __future__ import annotations

import argparse
import time
from pathlib import Path


FEATURES = (
    "otype",
    "oslots",
    "sp",
    "vt",
    "vs",
    "gn",
    "nu",
    "ps",
    "language",
    "function",
    "typ",
    "kind",
    "domain",
)

QUERIES: tuple[tuple[str, str], ...] = (
    ("generic_001", "."),
    ("generic_002", ". sp=verb"),
    (
        "generic_003",
        """
p:. function=Pred
w:word sp=verb
p [[ w
""",
    ),
    ("lex_001", "word sp=verb"),
    ("lex_002", "word sp=subs"),
    ("lex_003", "word sp=prep"),
    ("lex_004", "word sp=conj"),
    ("lex_005", "word sp=nmpr"),
    ("lex_006", "word sp=art"),
    ("lex_007", "word sp=adjv"),
    ("lex_008", "word sp=advb"),
    ("lex_009", "word vt=perf"),
    ("lex_010", "word vt=impf"),
    ("lex_011", "word vt=wayq"),
    ("lex_012", "word vt=impv"),
    ("lex_013", "word vt=ptca"),
    ("lex_014", "word vt=infc"),
    ("lex_015", "word vs=qal"),
    ("lex_016", "word vs=piel"),
    ("lex_017", "word vs=hif"),
    ("lex_018", "word vs=nif"),
    ("lex_019", "word gn=m"),
    ("lex_020", "word gn=f"),
    ("lex_021", "word nu=sg"),
    ("lex_022", "word nu=pl"),
    ("lex_023", "word nu=du"),
    ("lex_024", "word sp=verb vt=perf"),
    ("lex_025", "word sp=verb vs=qal"),
    ("lex_026", "word sp=subs gn=m nu=pl"),
    ("lex_027", "word sp=verb vs=qal vt=perf"),
    ("lex_028", "word language=Aramaic"),
    ("lex_029", "word sp=verb vs=hif vt=perf"),
    ("lex_030", "word sp=subs gn=f nu=sg"),
    ("lex_031", "word sp=verb vt=impv gn=m"),
    ("struct_001", "phrase function=Pred"),
    ("struct_002", "phrase function=Subj"),
    ("struct_003", "phrase function=Objc"),
    ("struct_004", "phrase function=Cmpl"),
    ("struct_005", "phrase function=Adju"),
    ("struct_006", "phrase function=Time"),
    ("struct_007", "phrase function=Loca"),
    ("struct_008", "phrase typ=VP"),
    ("struct_009", "phrase typ=NP"),
    ("struct_010", "phrase typ=PP"),
    ("struct_011", "phrase typ=CP"),
    ("struct_012", "phrase typ=AdvP"),
    ("struct_013", "clause kind=VC"),
    ("struct_014", "clause kind=NC"),
    ("struct_015", "clause domain=Q"),
    ("struct_016", "clause domain=N"),
    ("struct_017", "clause typ=Way0"),
    ("struct_018", "clause typ=NmCl"),
    ("struct_019", "clause typ=InfC"),
    (
        "struct_020",
        """
clause
  phrase function=Subj
""",
    ),
    (
        "struct_021",
        """
clause
  phrase function=Pred
""",
    ),
    (
        "struct_022",
        """
clause kind=VC
  phrase function=Objc
""",
    ),
    (
        "struct_023",
        """
phrase typ=NP
  word sp=subs
""",
    ),
    (
        "struct_024",
        """
phrase typ=VP
  word sp=verb
""",
    ),
    (
        "struct_025",
        """
phrase typ=PP
  word sp=prep
""",
    ),
    (
        "struct_026",
        """
clause
  phrase function=Subj
  phrase function=Pred
""",
    ),
    (
        "struct_027",
        """
clause
  phrase function=Pred
  phrase function=Objc
""",
    ),
    (
        "struct_028",
        """
sentence
  clause
""",
    ),
    (
        "struct_029",
        """
verse
  clause kind=VC
""",
    ),
    (
        "struct_030",
        """
clause
  phrase
    word sp=verb vt=wayq
""",
    ),
    (
        "quant_001",
        """
clause
/with/
  phrase function=Subj
/-/
""",
    ),
    (
        "quant_002",
        """
clause kind=VC
/with/
  phrase function=Objc
/-/
""",
    ),
    (
        "quant_003",
        """
clause
/without/
  phrase function=Subj
/-/
""",
    ),
    (
        "quant_004",
        """
phrase typ=NP
/with/
  word sp=art
/-/
""",
    ),
    (
        "quant_005",
        """
phrase typ=NP
/without/
  word sp=art
/-/
""",
    ),
    (
        "quant_006",
        """
clause
/with/
  phrase function=Time
/-/
""",
    ),
    (
        "quant_007",
        """
clause
/with/
  phrase function=Loca
/-/
""",
    ),
    (
        "quant_008",
        """
phrase function=Pred
/with/
  word vt=perf
/-/
""",
    ),
    (
        "quant_009",
        """
phrase function=Pred
/with/
  word vt=impf
/-/
""",
    ),
    (
        "quant_010",
        """
clause domain=Q
/with/
  phrase function=Voct
/-/
""",
    ),
    (
        "quant_011",
        """
clause
/with/
  phrase function=Nega
/-/
""",
    ),
    (
        "quant_012",
        """
phrase typ=NP
/with/
  word sp=adjv
/-/
""",
    ),
    (
        "quant_013",
        """
clause kind=NC
/with/
  phrase function=PreC
/-/
""",
    ),
    (
        "quant_014",
        """
verse
/with/
  clause domain=Q
/-/
""",
    ),
    (
        "quant_015",
        """
sentence
/with/
  clause kind=VC
  clause kind=NC
/-/
""",
    ),
    (
        "quant_016",
        """
phrase function=Subj
/with/
  word sp=nmpr
/-/
""",
    ),
    (
        "quant_017",
        """
clause
/with/
  phrase function=Subj
  phrase function=Objc
/-/
""",
    ),
    (
        "quant_018",
        """
clause
/without/
  phrase function=Objc
/-/
""",
    ),
    (
        "quant_019",
        """
phrase typ=VP
/with/
  word vs=hif
/-/
""",
    ),
    (
        "quant_020",
        """
clause
/with/
  phrase function=Cmpl
/-/
""",
    ),
    (
        "complex_001",
        """
clause kind=VC
  phrase function=Subj
    word sp=nmpr
  phrase function=Pred
    word vt=perf
""",
    ),
    (
        "complex_002",
        """
clause kind=VC
  phrase function=Pred
    word vt=wayq
  phrase function=Objc
""",
    ),
    (
        "complex_003",
        """
sentence
  clause kind=VC
    phrase function=Subj
    phrase function=Pred
      word vt=perf
""",
    ),
    (
        "complex_004",
        """
clause domain=Q
  phrase function=Pred
    word sp=verb
  phrase function=Objc
    word sp=subs
""",
    ),
    (
        "complex_005",
        """
verse
  clause kind=VC
    phrase function=Pred
      word vs=qal vt=perf
""",
    ),
    (
        "complex_006",
        """
clause
  phrase function=Subj
    word sp=subs gn=m nu=sg
""",
    ),
    (
        "complex_007",
        """
clause
  phrase function=Pred
  phrase function=Subj
  phrase function=Objc
""",
    ),
    (
        "complex_008",
        """
clause kind=NC
  phrase function=Subj
  phrase function=PreC
""",
    ),
    (
        "complex_009",
        """
sentence
  clause
    phrase typ=PP
      word sp=prep
""",
    ),
    (
        "complex_010",
        """
clause
  phrase function=Pred
    word sp=verb vt=impv
""",
    ),
    (
        "complex_011",
        """
clause
/with/
  phrase function=Subj
    word sp=nmpr
/-/
  phrase function=Pred
""",
    ),
    (
        "complex_012",
        """
clause kind=VC
/with/
  phrase function=Time
/-/
  phrase function=Pred
    word vt=perf
""",
    ),
    (
        "complex_013",
        """
clause domain=N
  phrase function=Pred
    word vt=wayq vs=qal
""",
    ),
    (
        "complex_014",
        """
verse
  clause
    phrase function=Subj
    phrase function=Pred
    phrase function=Objc
""",
    ),
    (
        "complex_015",
        """
clause
  phrase function=Pred
    word sp=verb vs=piel
  phrase function=Objc
""",
    ),
    (
        "complex_016",
        """
clause
/with/
  phrase function=Adju
/-/
  phrase function=Pred
  phrase function=Subj
""",
    ),
    (
        "complex_017",
        """
sentence
  clause kind=VC
  clause kind=VC
""",
    ),
    (
        "complex_018",
        """
chapter
/with/
  verse
    clause domain=Q
/-/
""",
    ),
    (
        "complex_019",
        """
clause
  phrase function=Pred
    word sp=verb gn=m nu=sg ps=p3
""",
    ),
    (
        "complex_020",
        """
clause kind=VC domain=N
  phrase function=Pred
    word vt=wayq
  phrase function=Subj
    word sp=nmpr
""",
    ),
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "tf_path",
        nargs="?",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "benchmarks/.corpora/bhsa/tf",
    )
    parser.add_argument("limit", nargs="?", type=int, default=5)
    args = parser.parse_args()

    from cfabric import Fabric

    load_start = time.perf_counter()
    fabric = Fabric(locations=str(args.tf_path), silent="deep")
    api = fabric.load(" ".join(FEATURES), silent="deep")
    if api is False:
        raise RuntimeError("failed to load BHSA Python cfabric API")
    load_ms = (time.perf_counter() - load_start) * 1000
    print(f"loaded python_features={len(FEATURES)} load_ms={load_ms:.3f}")

    failures: list[str] = []
    for query_id, template in QUERIES:
        start = time.perf_counter()
        try:
            results = []
            for result in api.S.search(template):
                results.append(result)
                if len(results) >= args.limit:
                    break
            elapsed_ms = (time.perf_counter() - start) * 1000
            print(f"ok query={query_id} results={len(results)} elapsed_ms={elapsed_ms:.3f}")
        except Exception as error:  # noqa: BLE001 - script reports query-level failures.
            print(f"fail query={query_id} error={error}")
            failures.append(query_id)

    if failures:
        print(f"failed queries: {', '.join(failures)}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
