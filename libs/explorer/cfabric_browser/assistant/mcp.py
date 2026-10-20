"""The tools as an MCP server, for the user's own AI app (Claude Code,
Claude Desktop, ...).

They are served at `/mcp` on the browser (Streamable HTTP, stateless, with
JSON answers), for apps that reach a local HTTP server (Claude Code). It
accepts only 127.0.0.1/localhost as host and no web page as origin, so
another tab in a web browser can't use it. Apps that start their own server
over stdio (Claude Desktop) go through a small proxy that passes each message
on to `/mcp`: `cfabric-browser mcp` (`stdio.py`), or the Desktop Extension's
own (`packaging/mcpb/`).
"""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from typing import Any

from anyio import to_thread
from mcp.server.fastmcp import FastMCP
from mcp.server.transport_security import TransportSecuritySettings

# calls a tool by name with its arguments; returns what the tool returns
Call = Callable[[str, dict[str, Any]], Awaitable[Any]]

INSTRUCTIONS = """\
You are helping a user search corpora (the Hebrew Bible as the BHSA, and
others) that are open in CACCHT Explorer. The user says in their
own words what they want to find; you write a search template for it.

The results always come from the browser's search, never from your own
knowledge: don't answer with counts or lists of passages yourself.

1. list_corpora, then corpus_guide for the corpus (it explains the features
   and conventions, with examples); describe_corpus, list_features and
   describe_feature for more.
2. Write a template (search_syntax_guide explains the language) and check it
   with check_template.
3. Run it as a trial with try_search: look at the count and the examples. If
   there are 0 results, read the hints; if far too many, narrow it. Repeat.
4. propose_search with the template, the user's request in their own words
   and a short explanation of each line. The browser shows it to the user,
   who runs it with Run. Tell the user it is there, with the trial's count.
"""

# the tools, by name
TOOLS = (
    "list_corpora",
    "corpus_guide",
    "describe_corpus",
    "list_features",
    "describe_feature",
    "search_syntax_guide",
    "check_template",
    "try_search",
    "propose_search",
)


def build_server(call: Call, name: str = "CACCHT Explorer") -> FastMCP:
    server = FastMCP(
        name,
        instructions=INSTRUCTIONS,
        stateless_http=True,
        json_response=True,
        streamable_http_path="/mcp",
        transport_security=TransportSecuritySettings(
            enable_dns_rebinding_protection=True,
            allowed_hosts=["127.0.0.1:*", "localhost:*", "[::1]:*"],
            # no web page may call it, not even the browser's own
            allowed_origins=[],
        ),
    )

    @server.tool()
    async def list_corpora() -> dict[str, Any]:
        """The corpora open in the browser: their names (to pass to the other
        tools), titles, ids, and section types (book, chapter, verse)."""
        return await call("list_corpora", {})

    @server.tool()
    async def corpus_guide(corpus: str) -> dict[str, Any]:
        """A guide to searching this corpus: its object types, its main
        features in plain words, conventions (such as how lexemes are
        written), worked examples and common mistakes. Read it first."""
        return await call("corpus_guide", {"corpus": corpus})

    @server.tool()
    async def describe_corpus(corpus: str) -> dict[str, Any]:
        """The corpus' object types (word, phrase, clause, verse, ...) with
        their numbers, and its sections."""
        return await call("describe_corpus", {"corpus": corpus})

    @server.tool()
    async def list_features(
        corpus: str, node_types: list[str] | None = None
    ) -> dict[str, Any]:
        """The features of the corpus, with descriptions; only those of
        `node_types` (such as ["word"]) if given."""
        return await call("list_features", {"corpus": corpus, "node_types": node_types})

    @server.tool()
    async def describe_feature(corpus: str, features: list[str]) -> dict[str, Any]:
        """Features in detail: their most frequent values with counts, so
        that a template uses values the corpus has."""
        return await call("describe_feature", {"corpus": corpus, "features": features})

    @server.tool()
    async def search_syntax_guide(section: str | None = None) -> dict[str, Any]:
        """The search template language: objects on lines, indentation for
        what lies inside what, features, relations and quantifiers. Without
        `section`, an overview with the sections there are."""
        return await call("search_syntax_guide", {"section": section})

    @server.tool()
    async def check_template(corpus: str, template: str) -> dict[str, Any]:
        """Check a template without searching: its problems (syntax, unknown
        object types or features), each with its line; `ok` when none."""
        return await call("check_template", {"corpus": corpus, "template": template})

    @server.tool()
    async def try_search(
        corpus: str, template: str, examples: int = 5
    ) -> dict[str, Any]:
        """Run a template as a trial: the number of results (`atLeast` when
        there are more than the trial counts), a few examples with their
        passage and text, and for 0 results hints such as a value a feature
        never has. For checking your template, not as the answer."""
        return await call(
            "try_search", {"corpus": corpus, "template": template, "examples": examples}
        )

    @server.tool()
    async def propose_search(
        corpus: str,
        template: str,
        request: str,
        explanation: str,
        model: str | None = None,
    ) -> dict[str, Any]:
        """Show the search in the user's browser, where they run it with Run,
        edit it, or dismiss it. `request`: what the user asked, in their own
        words; `explanation`: what each line of the template does, briefly;
        `model`: your name as the user should see it (such as "Claude").
        A template with problems is not shown: they come back to be fixed."""
        return await call(
            "propose_search",
            {
                "corpus": corpus,
                "template": template,
                "request": request,
                "explanation": explanation,
                "model": model,
            },
        )

    return server


def local(tools: Any) -> Call:
    """Calls on the browser's own tools (for `/mcp`), in a thread: a trial
    search waits for the pool."""
    from cfabric_browser.assistant.tools import ToolError

    async def call(name: str, args: dict[str, Any]) -> Any:
        if name not in TOOLS:
            raise ValueError(f"No tool {name}")

        def run() -> Any:
            try:
                return getattr(tools, name)(**args)
            except ToolError as e:
                return {"ok": False, "error": str(e)}

        return await to_thread.run_sync(run)

    return call
