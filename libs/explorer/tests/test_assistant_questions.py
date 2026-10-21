"""The counts in the BHSA guide's worked examples and in the question set,
checked against the real BHSA. Slow (about a minute) and needs the BHSA in
~/text-fabric-data, so only with CFABRIC_REAL_CORPORA=1."""

import os
import re
from pathlib import Path

import pytest
import yaml

HERE = Path(__file__).parent
GUIDE = HERE.parent / "cfabric_browser" / "assistant" / "guides" / "bhsa.md"
QUESTIONS = HERE / "data" / "bhsa-questions.yaml"
BHSA = Path.home() / "text-fabric-data/github/ETCBC/bhsa/tf/2021"


def guide_examples() -> list[tuple[str, str, int]]:
    """The rows of the guide's table: request, template, count."""
    rows = []
    for line in GUIDE.read_text(encoding="utf-8").splitlines():
        m = re.fullmatch(r"\| (.+?) \| (`.+`) \| ([\d,]+) \|", line)
        if m:
            template = "\n".join(re.findall(r"`([^`]+)`", m.group(2)))
            rows.append((m.group(1), template, int(m.group(3).replace(",", ""))))
    return rows


def test_the_guide_has_its_examples():
    examples = guide_examples()
    assert len(examples) >= 30
    _request, template, count = examples[0]
    assert (template, count) == ("word lex=>LHJM/", 2601)


def test_the_question_set_is_complete():
    questions = yaml.safe_load(QUESTIONS.read_text(encoding="utf-8"))
    assert len(questions) >= 30
    for q in questions:
        assert {"level", "request", "template", "count"} <= set(q)
        assert isinstance(q["count"], int)
        for other in q.get("also", []):
            assert {"template", "count", "why"} <= set(other)


real = pytest.mark.skipif(
    os.environ.get("CFABRIC_REAL_CORPORA") != "1" or not BHSA.is_dir(),
    reason="needs the BHSA and CFABRIC_REAL_CORPORA=1",
)


@pytest.fixture(scope="module")
def bhsa():
    from cfabric import Fabric

    return Fabric(locations=str(BHSA), silent="deep").loadAll(silent="deep")


def count(api, template: str) -> int:
    results = api.S.search(template)
    return 0 if results is None else len(list(results))


@real
def test_guide_counts(bhsa):
    wrong = [
        (request, expected, found)
        for (request, template, expected) in guide_examples()
        if (found := count(bhsa, template)) != expected
    ]
    assert wrong == []


@real
def test_question_counts(bhsa):
    questions = yaml.safe_load(QUESTIONS.read_text(encoding="utf-8"))
    wrong = [
        (q["request"], answer["count"], found)
        for q in questions
        for answer in [q, *q.get("also", [])]
        if (found := count(bhsa, answer["template"])) != answer["count"]
    ]
    assert wrong == []
