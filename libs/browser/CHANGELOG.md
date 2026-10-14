# Changelog

All notable changes to cfabric-browser will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Standalone program: output and errors go to `~/.cfabric/logs/cfabric-browser.log`, with a message on Windows when it cannot start; without a window toolkit it opens a web browser instead. `--no-window` runs it as a plain server. Built and smoke-tested on Windows, macOS and Linux by the `browser-desktop.yml` workflow (`packaging/smoke-test.sh`)
- `--window`: the browser in a window of its own (pywebview) instead of a web browser; closing it stops the server. The window keeps its web storage (settings, saved workspaces) in `~/.cfabric/window`, and takes the first free port from 8107 so that it keeps the same address. The standalone program opens this window, without a console
- Start without corpora: `cfabric-browser` on its own opens an empty browser that offers the corpora on this computer (*Open a corpus*) straight away; the first corpus opened goes into the first pane. With `--no-library`, a corpus is still required
- Standalone program: `packaging/cfabric-browser.spec` builds `cfabric-browser` with PyInstaller into a program that needs no Python installation (`pip install .[desktop]`); `python -m cfabric_browser` also starts the browser
- `cfabric-browser` command: serve one or more corpora given by name (`bhsa`), repository or data directory
- JSON API: corpora, features, section tree, passages and nodes as render trees, node inspector, template check, search with live status events, paging, results in context, TSV export
- Searches in worker processes: parallel, cancellable, with time limits and a result cap; results stream in and are put in canonical order when done
- TSV export of all results: features mentioned in the template become columns per result node (`sp*` adds a feature without narrowing the search, as in Text-Fabric); `?excel=true` gives a UTF-16 file with byte-order mark that Excel opens correctly
- Several corpora side by side: up to four panes, each with its own corpus, view, passage and search, all kept in the URL
- Synchronised reading: opening a passage in one pane opens the corresponding passage in the others, through a concordance of book names (`1_Samuel`, `1Sam`, `Samuel_1`); where a verse is missing in a corpus, its chapter is shown with a note; reading panes without a passage (a short link such as `?c0=bhsa&ref0=Genesis+1&c1=sp&sync=1`, or a new pane) open the passage of the first pane that has one
- Search all corpora: run a template in every open corpus and compare the counts; each opens in its own pane
- Choose a corpus version with `@`: `DT-UCPH/sp@7.2.4`, or `@latest` for the newest on this computer (by default the version named in the corpus' app config); the corpus card reports the version actually loaded
- Reading by biblical book in a corpus of manuscripts (DSS), whose sections are scroll, fragment and line: an index of the words' `book`/`chapter`/`verse` gives per verse the text of every scroll that contains it (`/books`, `/witnesses?ref=Genesis 1`); synchronised reading aligns it with MT and SP (`align?view=books`). Reconstructed text is flagged (`dim`), also in the scroll view, and the DSS placeholders for lost text (`ε`) and illegible letters (`#`, `?`) are shown as in the editions, `[…]` and `◦`, while the transcription's morpheme breaks (`׳`, as in `עמ׳י`) are left out; in comparisons, an illegible letter counts as unknown; text the DSS also lists as non-biblical (`biblical=2`) is labelled. With Show differences, each scroll is compared with the MT or SP pane: reconstructed letters don't count, nor does what is lost at the edges of a fragment or next to a reconstruction; the MT/SP pane marks a word where any scroll differs, with a tooltip naming the scrolls (`compare?bview=books`)
- Three-way comparison: with MT, SP and a DSS pane in book view open and Show differences on, each place where MT and SP differ shows which scrolls side with MT, which with SP and which with neither, read only where a scroll preserves the text; as a line per place under each verse in the DSS pane, and in tooltips on the marked words (`/api/agreement`)
- Show differences: where reading panes show the same passage of two Hebrew corpora (e.g. MT/BHSA and SP), the words in which the texts differ are marked, compared on their consonants verse by verse. Vowels, accents and word division don't count; a difference in matres lectionis only is marked as spelling, any other as a variant; a plus is marked in the text that has it. Kept in the URL (`diff=1`)
- Clicking a word in the text opens the word in the node inspector, also where slots are signs (SP, DSS): signs within a word are no longer inspected one by one, and words shown as one unit in the pretty view are clickable
- Local display settings in `~/.cfabric/display/<org>/<repo>[@<version>].yaml`, layered over a corpus' app config
- Cite: a citation to copy for the passage being read or for a search (template and number of results), with the corpus' title, version and DOI and the software versions and date; `/api/health` reports the context-fabric version
- Query editor: completes feature values after `feature=` (also after `|`), most frequent first with their counts; hovering a feature shows its description, the object types it is on and its most frequent values; hovering an object type shows how many there are
- Saved workspaces: save the current panes, passages and templates under a name in this browser, open or delete them, and export or import them as files; importing a Text-Fabric browser job opens its query (and runs it) and its first passage
- A pane whose search the server no longer has (after a restart, or from an old link) searches again with its template
- Download dialog: choose extra feature columns per part of the results, besides those in the template, and TSV or TSV for Excel (`export.tsv?extra=3:lex,gloss`)
- Statistics on search results: how often each value of a feature occurs on one part of the results (by default a feature the template leaves open, such as `lex*`), and how the results spread over books or chapters; with a table view, and links to read each section
- Library list: copies of one corpus (same title, e.g. a fork) are listed once, keeping the newest data; each entry shows the version that opens; corpora can be hidden with `library: hide:` in `~/.cfabric/browser.yaml` or `--hide`
- Corpus library: open other corpora found on this computer while the browser runs (`--no-library` switches this off)
- Template problems as structured data (line, message, feature or node type), including problems inside quantifiers that the search engine only logs
