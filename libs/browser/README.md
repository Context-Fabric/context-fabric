# cfabric-browser

A graphical browser for [Context-Fabric](https://context-fabric.ai) corpora, the successor of the Text-Fabric browser. It runs on your own computer and works offline.

> **Status: pre-alpha.** Reading, searching and inspecting work, also with several corpora side by side.

## Usage

```bash
pip install cfabric-browser
cfabric-browser bhsa
```

This opens the browser at <http://localhost:8107/>. Give several corpora to compare them, e.g. `cfabric-browser bhsa CenterBLC/LXX ETCBC/peshitta`; others can be opened from the browser with *Open a corpus*. Without corpora (`cfabric-browser`), the browser starts empty and shows that list straight away. You can give a corpus in several ways:

| Argument | Meaning |
|---|---|
| `bhsa`, `ETCBC/dss` | A corpus that Text-Fabric has downloaded or cloned, in `~/text-fabric-data/github` or `~/github` |
| `~/github/ETCBC/bhsa` | A repository in Text-Fabric layout (`tf/<version>/`); the version in its app config is used, otherwise the newest |
| `path/to/tf/2021` | A data directory with `.tf` files or a `.cfm` cache |
| `DT-UCPH/sp@7.2.4`, `sp@latest` | A specific version, or the newest on this computer (by default the version named in the corpus' app config) |
| `hebrew=…` | Any of the above, shown under the name `hebrew` |

Options:
- `--port`: default 8107.
- `--workers`: how many searches can run at once (default 2).
- `--timeout`: time limit per search in seconds (default 60).
- `--max-results`: results kept per search (default 500,000).
- `--no-open`: don't open a web browser.
- `--window` (`--no-window`): show the browser in a window of its own instead of a web browser (needs pywebview: `pip install cfabric-browser[desktop]`). Closing the window stops the server. Without `--port` it uses the first free port from 8107, so several windows can run at once. The window keeps its settings, search history and saved workspaces in `~/.cfabric/window`.
- `--no-library`: don't let users open other corpora found on this computer. Then at least one corpus must be given.
- `--hide ORG/REPO`: leave a corpus out of the *Open a corpus* list (repeatable). To do this permanently, list them in `~/.cfabric/browser.yaml`:

  ```yaml
  library:
    hide:
      - CLARIAH/descartes-tf
  ```

  The list also shows copies of one corpus (same title, such as a fork) only once, keeping the one with the newest data.

The server listens on `127.0.0.1` only. Pass `--host` to change that, and only when you know what you're doing.

## Queries

Queries use Text-Fabric's search template language, run by Context-Fabric's search engine. Queries written for the TF browser work unchanged and give the same results.

Problems in a template are reported per line. That includes problems inside quantifiers such as `/without/`, which the engine itself only logs, and then returns results as if the quantifier excluded nothing.

## Display settings

How a corpus is displayed comes from its Text-Fabric app configuration (`<repo>/app/config.yaml`). To adjust it without editing downloaded files, put the settings that differ in `~/.cfabric/display/<org>/<repo>.yaml`, or, for one version only, `<repo>@<version>.yaml`. These files use the same format as the app configuration and are layered over it. For example, `~/.cfabric/display/DT-UCPH/sp@7.2.4.yaml`:

```yaml
typeDisplay:
  word:
    base: true
    featuresBare: gloss
  phrase:
    label: '{typ} {function}'
  phrase_atom:
    hidden: true
```

## API

All functions of the interface are available as a JSON API; interactive documentation is at `/docs` while the browser runs. Where they overlap, the endpoints mirror the tools of `cfabric-mcp`, so that people and language models use the same operations.

| Endpoint | Purpose |
|---|---|
| `GET /api/corpora`, `GET /api/corpora/{c}` | Corpora and everything needed to work with one (types, sections, display settings) |
| `GET /api/corpora/{c}/features[/{f}]` | Features, with values and counts |
| `GET /api/corpora/{c}/sections?ref=` | The section tree, one level at a time |
| `GET /api/corpora/{c}/passage?ref=Genesis 1:1` | Render trees of a passage, with its section path and the previous and next passages |
| `GET /api/corpora/{c}/align?from=bhsa&ref=Genesis 1:1` | The corresponding passage in this corpus; `exact` is false when only the enclosing chapter exists |
| `GET /api/corpora/{c}/books` | For a corpus of manuscripts (DSS): the biblical books in it, with chapters, verses and scrolls |
| `GET /api/corpora/{c}/witnesses?ref=Genesis 1` | A chapter or verse in the book view: per verse, the text of each scroll that contains it |
| `GET /api/agreement?a=bhsa&aref=…&b=sp&bref=…&c=dss&cref=…` | At each place where `a` and `b` differ, which scrolls of `c` (book view) agree with `a`, with `b`, or with neither |
| `GET /api/compare?a=bhsa&aref=Genesis 1&b=sp&bref=Genesis 1` | Words that differ between two corresponding Hebrew passages, compared on their consonants verse by verse: `matres` (only matres lectionis) or `variant` |
| `GET /api/library`, `POST /api/corpora` | Corpora found on this computer; open one by its id (e.g. `ETCBC/dss`) |
| `GET /api/corpora/{c}/nodes/{n}[/info]` | Render tree of a node; all its features, edges and surroundings |
| `POST /api/corpora/{c}/check` | Check a template without searching |
| `POST /api/corpora/{c}/search` | Start a search (`{"template": …, "limit": …, "timeout": …}`) |
| `GET /api/searches/{id}[/events]` | Status, or a live event stream of it |
| `GET /api/searches/{id}/results` | A page of results (optionally with plain renderings) |
| `GET /api/searches/{id}/results/{i}` | One result in its context, highlighted |
| `GET /api/searches/{id}/stats/features?position=` | Features with values on one part of the results; which the template mentions, and which it leaves open |
| `GET /api/searches/{id}/stats/values?position=&feature=` | How often each value occurs there (distinct nodes) |
| `GET /api/searches/{id}/stats/sections?level=` | Results per book (0), chapter (1), ... in text order |
| `GET /api/searches/{id}/export.tsv` | All results, one row per result; features in the template become columns (write `sp*` to add one without narrowing the search); `extra=3:lex,gloss` adds columns for the third node; `excel=true` for Excel |
| `DELETE /api/searches/{id}` | Stop a search |

Display options are query parameters: `mode` (`pretty`/`plain`), `fmt`, `full`, `hide_types`, `hidden_types`, `base_types`, `condense_type`, `extra_features`, `lang`.

## How it works

- Searches run in worker processes, so the interface stays responsive and searches can run in parallel. Because corpora are memory-mapped, the workers share most of their memory. A search that ignores its time limit or a stop request is ended by restarting its worker.
- Results stream in while the search runs. Once it has finished, they are put in canonical order.
- Displays are built by `cfabric.display` as structured render trees, which the interface draws. The corpus' Text-Fabric app configuration (`app/config.yaml`) is used as it is.

## Development

The interface is a Vite + React + TypeScript app in `frontend/`. It is built into `cfabric_browser/static/`, which is not in git; the release workflow builds it before packaging.

```bash
cd libs/browser/frontend
npm install
npm run build        # type-check and build into cfabric_browser/static
npm run dev          # live-reloading interface on :5173, using a running cfabric-browser on :8107
```

Fonts are bundled from npm (`@fontsource`), so the browser needs no internet. For Hebrew, Ezra SIL or SBL Hebrew are used when installed on the computer, otherwise the bundled Noto Serif Hebrew.

Tests: `pytest tests/` in `libs/browser` (they start real search workers on a small fixture corpus).


### Standalone program

`packaging/cfabric-browser.spec` builds a program that runs without a Python installation (with PyInstaller; about 65 MB on Windows). It shows the browser in a window of its own (`--window`), without a console. Started without corpora, for example by double-clicking it, it offers the corpora on this computer. Build it in a separate virtual environment with normal, not editable, installs: PyInstaller does not find editable installs. On Windows, make that environment with a Python from python.org: PyInstaller cannot bundle the Microsoft Store Python.

```bash
cd libs/browser
(cd frontend && npm run build)
python -m venv build/env
build/env/Scripts/python -m pip install ../core ".[desktop]"   # Linux/macOS: build/env/bin/python
build/env/Scripts/pyinstaller packaging/cfabric-browser.spec --noconfirm
dist/cfabric-browser/cfabric-browser bhsa
```

Its entry point is `packaging/cfabric-browser.py`, which adds `--window` (`--no-window` overrides it). As it has no console, it writes its output and errors to `~/.cfabric/logs/cfabric-browser.log`, and on Windows shows a message when it cannot start. If no window toolkit is available (on Linux, pywebview needs GTK or Qt), it opens a web browser instead.

`packaging/smoke-test.sh` starts a built program as a server with a small corpus and checks that the interface is served and that a search runs; it needs only bash and curl. The workflow `browser-desktop.yml` builds the program on Windows, macOS (Apple silicon and Intel, as `Context-Fabric.app`) and Linux, smoke-tests each build, runs the Windows build once more on a runner where no Python can be reached, and keeps the builds as artifacts for 14 days. The builds are not signed, so Windows SmartScreen and macOS Gatekeeper warn about them. Like `cfabric_browser/__main__.py` (`python -m cfabric_browser`), it calls `multiprocessing.freeze_support()` so that search workers start correctly in the frozen program.
