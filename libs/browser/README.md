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
- `--ui NAME`: the interface to start in until the user chooses one: `panes` (the default), `parallel`, `reader`, `interlinear` or `concordance`. Permanently, and to leave interfaces out of the menu, in `~/.cfabric/browser.yaml`:

  ```yaml
  ui:
    default: parallel
    hide: [concordance]
  ```

  (`ui: parallel` alone also works.) See *Interfaces*.
- `--no-library`: don't let users open other corpora found on this computer. Then at least one corpus must be given.
- `--only ORG/REPO`: limit the *Open a corpus* list to these corpora (repeatable), e.g. for a test with a few corpora; others can't be opened from the browser either. Permanently: `library: only:` in `~/.cfabric/browser.yaml`.
- `--hide ORG/REPO`: leave a corpus out of the *Open a corpus* list (repeatable). To do this permanently, list them in `~/.cfabric/browser.yaml`:

  ```yaml
  library:
    hide:
      - CLARIAH/descartes-tf
  ```

  The list also shows copies of one corpus (same title, such as a fork) only once, keeping the one with the newest data.

The server listens on `127.0.0.1` only. Pass `--host` to change that, and only when you know what you're doing.

## Interfaces

The browser has several interfaces, chosen in the menu at the right of the top bar (it shows the name of the one in use). Switching keeps the passage, the corpora, the search, the lexicon and the settings; what an interface cannot show is kept for the next one. The interface used last is remembered by the web browser (or the window); `--ui` only sets the one for a first start. *Choose which to offer…* in the menu leaves interfaces out of it, though never the one in use, nor both *Panes* and *Parallel*.

All five are built: *Panes*, *Parallel*, *Reader*, *Interlinear* and *Concordance*.

### Reader

*Reader* (`/read/bhsa/Genesis/1?v=3&panel=parallels`) is after Sefaria. It shows one text in a quiet, wide column.

- **Reading:**
  - *Verses* gives a line per verse; *Running text* runs the verses on, with a new paragraph after each petuhah (פ) or setumah (ס) in the text.
  - Scrolling goes on into the next chapters, and the address follows the chapter at the top.
  - `‹ ›`, the arrow keys and the chapter name (for the books and chapters) work as in *Parallel*. The menu next to it chooses the text to read.
- **The panel:** clicking a verse marks it and opens the panel, on the right (on a phone, from the bottom). Clicking a word opens the panel at that word. Its tabs:
  - *Parallels*: the verse in the other texts, chosen with *Texts*; the DSS per scroll; *Show differences* marks them, in the text read as well.
  - *Words*: the words of the verse with gloss and morphology; click one for everything about it.
  - *Lexicon*: the verse's lexemes with gloss and frequency. *All* opens the lexicon in *Parallel*.
  - *Search*: a search for words like one of the verse's, opened in *Parallel*.
  - *Notes*: your own note and bookmark (★) on the verse, and a list of all your notes. They are kept in this browser (or the program's window), never sent anywhere, and are included in exported workspaces.
- *Cite*, next to the chapter, cites the chapter, or the verse chosen.

- *Panes* (`/?c0=bhsa&ref0=Genesis+1&...`): up to four panes side by side, each with its own corpus, passage and view.
- *Parallel* (`/Genesis/1?texts=bhsa,sp,dss`), laid out after [Parabible](https://parabible.com): a slim top bar with the chapter (`‹ ›` and ←/→ for the neighbours, the chapter name for a drawer with the books and their chapters), and a column per corpus. *Texts* turns open corpora on and off, orders the columns, and adds more from the corpora on this computer; `–` on a column removes it. The chapters follow the BHSA's numbering, also when the BHSA is not shown (it is opened in the background when it is on this computer); without it, the first column's. Each row is a verse: verses are matched by book (through the book concordance), chapter and verse number, there is a row for every verse number in any of the texts, and a text that lacks a verse has an empty cell. A corpus read by biblical book (DSS) shows in each cell the text of every scroll that has the verse, labelled with the scroll. A corpus without the chapter keeps a narrow column marked *not in this corpus*.

Hovering over a word shows the same card as in *Panes*; clicking it opens the word panel on the left, with the same contents as the inspector and, for a BHSA word, *Lexicon entry*. *Lexicon* in the top bar (when a corpus with a lexicon is open) shows the lexicon instead of the chapter; a verse in it opens its chapter with that verse marked (`&v=26`; the lexicon is `&lexicon=bhsa&lx=…`). Its *Search* goes to the search in *Panes* for now.

*Differences* in the top bar (`&diff=1`) marks the words where the texts differ from the first column, with the same rules and colours as *Show differences* in *Panes*; each column header says how many words differ, and hovering over a marked word says from what. With MT, SP and the DSS shown, a DSS cell lists, for each place in that verse where MT and SP differ and a scroll preserves the text, which scrolls side with which.

*Search* in the top bar searches one of the shown texts (`&search=1&sc=bhsa&q=…&s=…`), in either of two ways:

- *Search terms*, as on Parabible: click a word, then *Add as search term* in the word panel. A term finds words with the features you tick (its transliterated lexeme, or else its lexeme, to start with; also part of speech, stem, tense, person, gender, number, state). Choose whether all terms must be in one verse, sentence, clause or phrase, and, for corpora with books, which books (by the book feature's own values: the BHSA's `book` says `Jesaia` where the list says Isaiah). The search template this makes is shown below.
- *Template*: the query editor of *Panes*, starting from the template the terms make.

Results are rows: the passage, then the verse the result lies in, in the searched text with the result's innermost nodes highlighted, and the same verse in the other texts (the DSS: per scroll); 25 per page. *Statistics*, *Download…* and *Cite* are as in *Panes*; clicking a passage opens its chapter at that verse. A shared link to a search the server no longer has runs it again.

The top bar also has *Display* (text format and what is shown, shared with *Panes*), *Workspaces* (a saved workspace remembers its interface: loading a parallel one from *Panes* switches over, and the other way round), *Cite* next to the chapter (a citation per shown text), and light or dark colours. On a phone the texts of a verse are shown below each other, each with its name.


### Interlinear

*Interlinear* (`/interlinear/bhsa/Genesis/1?under=sp&struct=1`) is after STEPBible, BibleHub and the Logos interlinears. It opens with the BHSA, and shows each word as a small block with lines under it. The blocks run from right to left, so a verse reads as a sentence.

- **Lines** chooses the lines and their order. They are read by role, as on the hover card, so the SP and the DSS work too:
  - the word;
  - the transliteration of the lexeme (`BR>[`);
  - the lexeme;
  - the gloss;
  - the part of speech;
  - the morphology, in words;
  - and any feature of the corpus (*Add a feature*).
- **Under** shows another text word by word under the words, with its differences marked. The SP under the BHSA, for example: words are matched on their consonants, as in the comparison, so a different word division still lines up.
- **Structure** groups the words in phrases, underlined with their function (`Pred`, `Subj`, `Objc`, …), and in clauses with their type (`WayX`, `xQtX`, …).
- **Practise** hides a line; click a word's **?** to see it, or use *Show all* and *Hide again*.
- **Colour** colours the words by part of speech, or the verbs by stem, with a legend.
- Clicking a word opens the word panel on the right.
- *Cite*, next to the chapter, cites the chapter, or the verse chosen.

A transliteration of the word as it sounds is not available yet: the BHSA's `phono` is a module of its own.

### Concordance

*Concordance* (`/concordance/bhsa?q=…`) is after Sketch Engine and AntConc. It starts from a search: a template, or search terms made from words in the results (click a word, then *Add as search term*). The results come as lines (KWIC, key word in context): the hit in the middle and the words before and after it on either side, within the same book. Clicking a passage opens it in the *Reader*.

- **Sort by** the hit or a word before or after it (up to the third), by its form (consonants), lexeme or part of speech. Patterns show at once, such as what comes before HLK[.
- **Facets** on the left: book, part of speech, stem, tense, person, gender, number, state, language, each with counts. A click filters the lines, and the facets follow.
- **Above the lines:**
  - the number of lines and of hits per 10,000 words;
  - *Context* (3, 6 or 10 words);
  - *One per verse*;
  - *Sample* (a random 50 to 500; *another* draws again);
  - *Download lines*: a TSV table of passage, before, hit and after, in the order and with the filters shown, at most 20,000.
- **Collocations:** the lexemes (or forms, or parts of speech) within 1 to 5 words of the hits, with their count there, their frequency in the corpus and their logDice score. logDice measures how typical a word is near the hit (at most 14).
- **Statistics:** as in the other interfaces.

*Cite* in the line with the number of results cites the search. On a phone the facets fold away behind *Filters*.

## Queries

Queries use Text-Fabric's search template language, run by Context-Fabric's search engine. Queries written for the TF browser work unchanged and give the same results.

Problems in a template are reported per line. That includes problems inside quantifiers such as `/without/`, which the engine itself only logs, and then returns results as if the quantifier excluded nothing.

## Words

Hovering over a word shows a card with its essentials: gloss, lexeme, transliteration, part of speech and morphology (stem, tense, person, gender, number, state, suffix), in words rather than codes, and, with *Show differences*, what it differs from. Each is read from the first feature of its role that has a value; values such as `NA` and `absent` are left out (`cfabric_browser/summary.py`). The DSS has its own analysis and the ETCBC's (features ending in `_etcbc`, as in the BHSA): the ETCBC's comes first (e.g. `sp_etcbc`, which tells prepositions and proper nouns apart), the DSS's own where the ETCBC has nothing. The DSS has no glosses: they come from the lexicon of another open corpus (the BHSA), by the word's ETCBC lexeme and language (`lex_etcbc`, `lang_etcbc`), marked `· bhsa`. Clicking a word opens the inspector with the same essentials, all other features (folded away; without empty values, and without the `g_` features unless asked for), its relations and what it lies in.

## Lexicon

A corpus with lexeme nodes (`lex`) that have glosses, such as the BHSA, gets a third view besides *Read* and *Search*: *Lexicon*, after SHEBANQ's *Words* page.

- Lexemes per language (Hebrew, Aramaic) and first letter, in Hebrew alphabetical order (sin and shin as one letter), with their vocalised form, transliteration, gloss, part of speech and frequency.
- *Find* searches all letters at once, by Hebrew (with or without vowels), transliteration or gloss; lexemes can also be limited to a part of speech, and sorted by frequency.
- A lexeme shows every verse it occurs in, as running text with its words marked (a verse with several occurrences once, with `×2`), how often it occurs per book (choose a book to see only its verses), and links to search for it (`word lex=BR>[ language=Hebrew`) and to download its occurrences as TSV. Clicking a verse reads it; with *Sync passages*, the other panes follow.

The features used are, per role, the first one the corpus has: `lex`, `voc_lex_utf8` or `lex_utf8`, `gloss`, `sp`, `language`, `freq_lex` (otherwise the words are counted), `rank_lex` and `nametype` (`cfabric_browser/lexicon.py`).

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
| `GET /api/searches/{id}/parallel?offset=&limit=&texts=` | A page of results as parallel rows: each result's verse in the searched corpus, highlighted, and in `texts` |
| `GET /api/corpora/{c}/interlinear?ref=&features=&under=` | A passage word by word: per word its lines in words, the extra features, and its phrase and clause (with function and type); with `under`, another corpus' words aligned under them, with their differences |
| `GET /api/searches/{id}/kwic?offset=&limit=&left=&right=&sort=&filter=&per=&sample=&seed=&features=` | A page of the results as concordance lines: the hit and its context, sorted (`L1.text`, `hit.lex`, …), filtered by features of the hit (`vs:qal,book:Genesis`), one per verse, or sampled |
| `GET /api/searches/{id}/kwic/facets?feature=&filter=&per=` | How often each value of a feature of the hit occurs in the filtered results (`book`: in book order) |
| `GET /api/searches/{id}/collocations?feature=&window=&filter=&per=&min=&top=` | The values of a feature near the hits, with count, corpus frequency and logDice |
| `GET /api/parallel?lead=&ref=&texts=` | A chapter of `lead` (`Genesis 1`) in the corpora `texts`, as rows of verses with a cell per corpus (the DSS: per scroll), for the parallel interface |
| `GET /api/corpora/{c}/lexicon` | All lexemes of a corpus with a lexicon (BHSA), in alphabetical order per language |
| `GET /api/corpora/{c}/lexicon/{n}` | One lexeme, with its number of verses and how often it occurs per book |
| `GET /api/corpora/{c}/lexicon/{n}/occurrences?offset=&limit=&book=` | A page of the verses it occurs in, as plain render trees with its words highlighted |
| `GET /api/corpora/{c}/lexicon/{n}/occurrences.tsv` | Every occurrence: section, word and verse text; `excel=true` for Excel |
| `GET /api/library`, `POST /api/corpora` | Corpora found on this computer; open one by its id (e.g. `ETCBC/dss`) |
| `GET /api/corpora/{c}/nodes/{n}[/info]` | Render tree of a node; all its features, edges and surroundings |
| `GET /api/corpora/{c}/nodes/{n}/summary` | The essentials of a word, as on the card shown when hovering over it |
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

Its entry point is `packaging/cfabric-browser.py`, which adds `--window` (`--no-window` overrides it), `--ui parallel` (the parallel interface on a first start) and, for the test version, `--only` for the corpora in its `CORPORA` list (BHSA, SP and DSS); empty that list to offer all corpora. As it has no console, it writes its output and errors to `~/.cfabric/logs/cfabric-browser.log`, and on Windows shows a message when it cannot start. If no window toolkit is available (on Linux, pywebview needs GTK or Qt), it opens a web browser instead.

The program icons in `packaging/icons/` are drawn from the Context-Fabric mark (`frontend/public/favicon.svg`) by `packaging/make_icons.py` (needs Pillow); run it again when the mark changes.

`packaging/smoke-test.sh` starts a built program as a server with a small corpus and checks that the interface is served and that a search runs; it needs only bash and curl. The workflow `browser-desktop.yml` builds the program on Windows, macOS (Apple silicon and Intel, as `Context-Fabric.app`) and Linux, smoke-tests each build, runs the Windows build once more on a runner where no Python can be reached, and keeps the builds as artifacts for 14 days. The builds are not signed, so Windows SmartScreen and macOS Gatekeeper warn about them. Like `cfabric_browser/__main__.py` (`python -m cfabric_browser`), it calls `multiprocessing.freeze_support()` so that search workers start correctly in the frozen program.
