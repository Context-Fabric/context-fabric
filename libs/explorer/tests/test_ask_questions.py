"""Tests for the parts of `scripts/ask_questions.py` that read the apps'
output; asking itself needs an account and a running Explorer."""

import json
import sys
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parent.parent / "scripts"))

import ask_questions as ask


def lines(*events):
    return "\n".join(json.dumps(e) for e in events)


def test_claudes_tool_calls():
    output = lines(
        {"type": "system", "subtype": "init"},
        {
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Let me look."},
                    {
                        "type": "tool_use",
                        "id": "a",
                        "name": "mcp__caccht__corpus_guide",
                    },
                ]
            },
        },
        {
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "tool_use", "id": "b", "name": "mcp__caccht__try_search"},
                    {"type": "tool_use", "id": "c", "name": "mcp__other__try_search"},
                ]
            },
        },
        {"type": "result", "result": "Proposed: 2,703 results."},
    )
    assert ask.tool_calls(output) == ["corpus_guide", "try_search"]
    assert ask.answer_text(output) == "Proposed: 2,703 results."


def test_codexs_tool_calls():
    started = {"id": "item_1", "type": "mcp_tool_call", "tool": "try_search"}
    output = lines(
        {"type": "thread.started"},
        {"type": "item.started", "item": started},
        {"type": "item.completed", "item": {**started, "status": "completed"}},
        {
            "type": "item.completed",
            "item": {"id": "item_2", "type": "mcp_tool_call", "tool": "propose_search"},
        },
        {"type": "item.completed", "item": {"type": "agent_message", "text": "Done."}},
        "not json",
    )
    assert ask.tool_calls(output) == ["try_search", "propose_search"]
    assert ask.answer_text(output) == "Done."


def test_same_template():
    assert ask.same_template("book book=Ruth\n  word  \n\n", "book book=Ruth\n  word")
    assert not ask.same_template("book book=Ruth\nword", "book book=Ruth\n  word")


def test_commands_keep_to_the_explorer():
    args = SimpleNamespace(url="http://127.0.0.1:8107/", model="sonnet", bypass=False)
    claude = ask.command("claude", args, "Find kings")
    assert claude[1:4] == ["-p", "--model", "sonnet"]
    assert claude[claude.index("--tools") + 1] == ""
    assert "--strict-mcp-config" in claude
    config = json.loads(claude[claude.index("--mcp-config") + 1])
    assert config["mcpServers"]["caccht"]["url"] == "http://127.0.0.1:8107/mcp"
    codex = ask.command("codex", args, "Find kings")
    assert codex[1:4] == ["exec", "--model", "sonnet"] and codex[-1] == "Find kings"
    assert "--sandbox" in codex
    assert 'mcp_servers.caccht.url="http://127.0.0.1:8107/mcp"' in codex


def test_summary():
    row = {
        "number": 3,
        "level": "easy",
        "request": "JHWH in Genesis",
        "expected": 165,
        "proposed": True,
        "count": 164,
        "correct": False,
        "sameTemplate": False,
        "trials": 2,
        "checks": 1,
        "seconds": 40.0,
        "error": "",
    }
    text = ask.summary([row, {**row, "number": 4, "count": 165, "correct": True}])
    assert "2 requests: 2 proposed, 1 with the right count (50%)" in text
    assert "3. JHWH in Genesis: count 164 instead of 165" in text
