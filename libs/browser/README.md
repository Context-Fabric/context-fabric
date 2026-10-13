# cfabric-browser

A graphical browser for [Context-Fabric](https://context-fabric.ai) corpora, the successor of the Text-Fabric browser. It runs on your own computer and works offline.

> **Status: pre-alpha.** The server and its API work; the interface is being built.

## Usage

```bash
pip install cfabric-browser
cfabric-browser bhsa
```

This opens the browser at <http://localhost:8107/>. You can give a corpus in several ways:

| Argument | Meaning |
|---|---|
| `bhsa`, `ETCBC/dss` | A corpus that Text-Fabric has downloaded or cloned, in `~/text-fabric-data/github` or `~/github` |
| `~/github/ETCBC/bhsa` | A repository in Text-Fabric layout (`tf/<version>/`); the version in its app config is used, otherwise the newest |
| `path/to/tf/2021` | A data directory with `.tf` files or a `.cfm` cache |
| `hebrew=…` | Any of the above, shown under the name `hebrew` |

Options:
- `--port`: default 8107.
- `--workers`: how many searches can run at once (default 2).
- `--timeout`: time limit per search in seconds (default 60).
- `--max-results`: results kept per search (default 500,000).
- `--no-open`: don't open a web browser.

The server listens on `127.0.0.1` only. Pass `--host` to change that, and only when you know what you're doing.

## Queries

Queries use Text-Fabric's search template language, run by Context-Fabric's search engine. Queries written for the TF browser work unchanged and give the same results.

Problems in a template are reported per line. That includes problems inside quantifiers such as `/without/`, which the engine itself only logs, and then returns results as if the quantifier excluded nothing.

## API

All functions of the interface are available as a JSON API; interactive documentation is at `/docs` while the browser runs. Where they overlap, the endpoints mirror the tools of `cfabric-mcp`, so that people and language models use the same operations.

| Endpoint | Purpose |
|---|---|
| `GET /api/corpora`, `GET /api/corpora/{c}` | Corpora and everything needed to work with one (types, sections, display settings) |
| `GET /api/corpora/{c}/features[/{f}]` | Features, with values and counts |
| `GET /api/corpora/{c}/sections?ref=` | The section tree, one level at a time |
| `GET /api/corpora/{c}/passage?ref=Genesis 1:1` | Render trees of a passage |
| `GET /api/corpora/{c}/nodes/{n}[/info]` | Render tree of a node; all its features, edges and surroundings |
| `POST /api/corpora/{c}/check` | Check a template without searching |
| `POST /api/corpora/{c}/search` | Start a search (`{"template": …, "limit": …, "timeout": …}`) |
| `GET /api/searches/{id}[/events]` | Status, or a live event stream of it |
| `GET /api/searches/{id}/results` | A page of results (optionally with plain renderings) |
| `GET /api/searches/{id}/results/{i}` | One result in its context, highlighted |
| `GET /api/searches/{id}/export.tsv` | All results as a spreadsheet-ready file |
| `DELETE /api/searches/{id}` | Stop a search |

Display options are query parameters: `mode` (`pretty`/`plain`), `fmt`, `full`, `hide_types`, `hidden_types`, `base_types`, `condense_type`, `extra_features`, `lang`.

## How it works

- Searches run in worker processes, so the interface stays responsive and searches can run in parallel. Because corpora are memory-mapped, the workers share most of their memory. A search that ignores its time limit or a stop request is ended by restarting its worker.
- Results stream in while the search runs. Once it has finished, they are put in canonical order.
- Displays are built by `cfabric.display` as structured render trees, which the interface draws. The corpus' Text-Fabric app configuration (`app/config.yaml`) is used as it is.
