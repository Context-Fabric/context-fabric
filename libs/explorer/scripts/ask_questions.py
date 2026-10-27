"""Ask an AI app the question set, and score what it proposes.

    python scripts/ask_questions.py claude [--model sonnet] [--only 1,5]
    python scripts/ask_questions.py codex [--bypass]

Each request of `tests/data/bhsa-questions.yaml` goes to the app without its
interactive screen (`claude -p`, `codex exec`), with the user's own
subscription, connected to a running CACCHT Explorer (`--url`, default the
standalone program's port) that has the BHSA open. After each request the
newest proposal is run in full in the Explorer, and its count compared with
the question's. Not in CI: it needs an account, and takes minutes per
request.

Per request: whether there was a proposal, whether its count is right,
whether its template is the question's, how many trials (`try_search`) and
checks (`check_template`) the model made, and the seconds. The rows go to
`--out` as JSON lines; a summary is printed.

Claude runs with no tools but the Explorer's (`--tools ""`,
`--strict-mcp-config`, so not the user's other MCP servers). Codex runs in
an empty temporary folder with a read-only sandbox, the Explorer given as
`-c mcp_servers.<name>.url`; `codex exec` can't ask for approval, and if it
cancels the tool calls, `--bypass` passes
`--dangerously-bypass-approvals-and-sandbox` (then Codex could also run
commands unasked; it is told to use only the Explorer).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import yaml

HERE = Path(__file__).resolve().parent
QUESTIONS = HERE.parent / "tests" / "data" / "bhsa-questions.yaml"
SERVER = "caccht"  # the Explorer's name for the app
TIMEOUT = 900  # seconds per request
FINISHED = {"done", "failed", "timeout", "cancelled"}

PROMPT = """\
{request}

(Search the corpus {corpus} with CACCHT Explorer's tools and propose the \
search there. Use only those tools.)"""


# the Explorer's API


def get(url: str) -> Any:
    with urllib.request.urlopen(url, timeout=30) as response:
        return json.load(response)


def post(url: str, body: dict[str, Any]) -> Any:
    request = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def proposals(url: str) -> list[dict[str, Any]]:
    """The proposals, the newest first."""
    return list(get(url + "api/proposals")["proposals"])


def full_count(url: str, corpus: str, template: str) -> int | None:
    """The template's number of results, run in full; None if it failed."""
    try:
        job = post(url + f"api/corpora/{corpus}/search", {"template": template})
    except OSError:
        return None
    while job["state"] not in FINISHED:
        time.sleep(0.5)
        job = get(url + f"api/searches/{job['id']}")
    return int(job["count"]) if job["state"] == "done" else None


# the apps


def command(app: str, args: argparse.Namespace, prompt: str) -> list[str]:
    mcp = args.url + "mcp"
    if app == "claude":
        config = {"mcpServers": {SERVER: {"type": "http", "url": mcp}}}
        found = [
            shutil.which("claude") or "claude",
            "-p",
            prompt,
            "--tools",
            "",
            "--mcp-config",
            json.dumps(config),
            "--strict-mcp-config",
            "--allowedTools",
            f"mcp__{SERVER}",
            "--output-format",
            "stream-json",
            "--verbose",
            "--no-session-persistence",
        ]
    else:
        found = [
            shutil.which("codex") or "codex",
            "exec",
            "--json",
            "--skip-git-repo-check",
            "-c",
            f'mcp_servers.{SERVER}.url="{mcp}"',
            "-c",
            f'mcp_servers.{SERVER}.default_tools_approval_mode="auto"',
            "-c",
            f"mcp_servers.{SERVER}.tool_timeout_sec=300",
        ]
        if args.bypass:
            found.append("--dangerously-bypass-approvals-and-sandbox")
        else:
            found += ["--sandbox", "read-only"]
        found.append(prompt)
    if args.model:
        found[2:2] = ["--model", args.model]
    return found


def events(output: str) -> Iterator[dict[str, Any]]:
    """The JSON objects of an app's output, one per line."""
    for line in output.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if isinstance(event, dict):
            yield event


def tool_calls(output: str) -> list[str]:
    """The names of the Explorer's tools the app called, in order: Claude's
    `tool_use` blocks (`mcp__caccht__try_search`), Codex's `mcp_tool_call`
    items (`tool: try_search`), each call once."""
    names: dict[str, str] = {}

    def walk(item: Any) -> None:
        if isinstance(item, list):
            for x in item:
                walk(x)
        elif isinstance(item, dict):
            kind = item.get("type")
            if kind == "tool_use" and str(item.get("name", "")).startswith(
                f"mcp__{SERVER}__"
            ):
                names[str(item.get("id"))] = str(item["name"]).split("__")[-1]
            elif kind == "mcp_tool_call" and item.get("tool"):
                names[str(item.get("id"))] = str(item["tool"])
            for value in item.values():
                walk(value)

    for event in events(output):
        walk(event)
    return list(names.values())


def answer_text(output: str) -> str:
    """The app's last words to the user."""
    text = ""
    for event in events(output):
        if event.get("type") == "result" and isinstance(event.get("result"), str):
            text = event["result"]
        item = event.get("item")
        if isinstance(item, dict) and item.get("type") == "agent_message":
            text = str(item.get("text", ""))
    return text


def rights(question: dict[str, Any]) -> list[int]:
    """The counts that are right: the question's, and those of its other
    readings (`also`)."""
    return [question["count"], *(a["count"] for a in question.get("also", []))]


def same_template(a: str, b: str) -> bool:
    """Equal but for spacing at line ends and empty lines."""

    def lines(t: str) -> list[str]:
        return [line.rstrip() for line in t.strip().splitlines() if line.strip()]

    return lines(a) == lines(b)


# asking


def ask(
    app: str, args: argparse.Namespace, number: int, question: dict[str, Any]
) -> dict[str, Any]:
    before = {p["id"] for p in proposals(args.url)}
    prompt = PROMPT.format(request=question["request"], corpus=args.corpus)
    started = time.monotonic()
    with tempfile.TemporaryDirectory() as empty:
        try:
            done = subprocess.run(
                command(app, args, prompt),
                check=False,
                cwd=empty,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=TIMEOUT,
            )
            output, failed = done.stdout, done.returncode != 0
            error = done.stderr.strip()[-500:] if failed else ""
        except subprocess.TimeoutExpired as e:
            output = e.stdout if isinstance(e.stdout, str) else ""
            failed, error = True, f"no answer within {TIMEOUT} s"
    seconds = round(time.monotonic() - started, 1)
    new = [p for p in proposals(args.url) if p["id"] not in before]
    calls = tool_calls(output)
    row: dict[str, Any] = {
        "number": number,
        "level": question["level"],
        "request": question["request"],
        "expected": question["count"],
        "app": app,
        "model": args.model,
        "proposed": bool(new),
        "proposals": len(new),
        "trials": calls.count("try_search"),
        "checks": calls.count("check_template"),
        "calls": calls,
        "seconds": seconds,
        "answer": answer_text(output)[-1000:],
        "error": error if failed else "",
    }
    if new:
        newest = new[0]
        count = full_count(args.url, newest["corpus"], newest["template"])
        row.update(
            template=newest["template"],
            proposedBy=newest["model"],
            count=count,
            correct=count in rights(question),
            sameTemplate=any(
                same_template(newest["template"], answer["template"])
                for answer in [question, *question.get("also", [])]
            ),
        )
        # the user's screen shouldn't fill up with test proposals
        post(args.url + f"api/proposals/{newest['id']}", {"state": "dismissed"})
    else:
        row.update(correct=False, sameTemplate=False)
    return row


def summary(rows: list[dict[str, Any]]) -> str:
    n = len(rows) or 1
    proposed = sum(r["proposed"] for r in rows)
    correct = sum(r["correct"] for r in rows)
    lines = [
        (
            f"{len(rows)} requests: {proposed} proposed, {correct} with the right "
            f"count ({100 * correct // n}%), "
            f"{sum(r['sameTemplate'] for r in rows)} with the question's template"
        ),
        (
            f"trials per request {sum(r['trials'] for r in rows) / n:.1f}, "
            f"checks {sum(r['checks'] for r in rows) / n:.1f}, "
            f"seconds {sum(r['seconds'] for r in rows) / n:.0f}"
        ),
    ]
    for level in ("easy", "medium", "hard"):
        some = [r for r in rows if r["level"] == level]
        if some:
            right = sum(r["correct"] for r in some)
            lines.append(f"  {level}: {right} of {len(some)}")
    wrong = [r for r in rows if not r["correct"]]
    if wrong:
        lines.append("not right:")
        for r in wrong:
            got = (
                f"count {r.get('count')} instead of {r['expected']}"
                if r["proposed"]
                else ("no proposal" + (f" ({r['error'][:80]})" if r["error"] else ""))
            )
            lines.append(f"  {r['number']:2}. {r['request'][:60]}: {got}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("app", choices=["claude", "codex"])
    parser.add_argument("--url", default="http://127.0.0.1:8107/")
    parser.add_argument("--corpus", default="bhsa")
    parser.add_argument("--model", help="passed to the app as --model")
    parser.add_argument("--only", help="question numbers, such as 1,5,12 (from 1)")
    parser.add_argument("--bypass", action="store_true", help="codex only, see above")
    parser.add_argument("--out", help="JSON lines; default results-<app>-<time>.jsonl")
    args = parser.parse_args(argv)
    if not args.url.endswith("/"):
        args.url += "/"
    try:
        names = [c["name"] for c in get(args.url + "api/corpora")["corpora"]]
    except OSError as e:
        sys.exit(f"No CACCHT Explorer at {args.url}: {e}")
    if args.corpus not in names:
        sys.exit(f"The Explorer has no corpus {args.corpus!r} open: {names}")
    questions = yaml.safe_load(QUESTIONS.read_text(encoding="utf-8"))
    chosen = (
        [int(x) for x in args.only.split(",")]
        if args.only
        else list(range(1, len(questions) + 1))
    )
    out = Path(args.out or f"results-{args.app}-{time.strftime('%Y%m%d-%H%M')}.jsonl")
    rows = []
    with out.open("a", encoding="utf-8") as sink:
        for number in chosen:
            row = ask(args.app, args, number, questions[number - 1])
            rows.append(row)
            sink.write(json.dumps(row, ensure_ascii=False) + "\n")
            sink.flush()
            mark = "ok " if row["correct"] else "-- "
            print(
                f"{mark}{number:2}. {row['request'][:56]:56} "
                f"{row.get('count')} / {row['expected']}, "
                f"{row['trials']} trials, {row['seconds']} s",
                flush=True,
            )
    print(summary(rows))
    print(f"rows in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
