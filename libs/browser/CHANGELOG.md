# Changelog

All notable changes to cfabric-browser will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `cfabric-browser` command: serve one or more corpora given by name (`bhsa`), repository or data directory
- JSON API: corpora, features, section tree, passages and nodes as render trees, node inspector, template check, search with live status events, paging, results in context, TSV export
- Searches in worker processes: parallel, cancellable, with time limits and a result cap; results stream in and are put in canonical order when done
- TSV export of all results: features mentioned in the template become columns per result node (`sp*` adds a feature without narrowing the search, as in Text-Fabric); `?excel=true` gives a UTF-16 file with byte-order mark that Excel opens correctly
- Several corpora side by side: up to four panes, each with its own corpus, view, passage and search, all kept in the URL
- Synchronised reading: opening a passage in one pane opens the corresponding passage in the others, through a concordance of book names (`1_Samuel`, `1Sam`, `Samuel_1`); where a verse is missing in a corpus, its chapter is shown with a note
- Search all corpora: run a template in every open corpus and compare the counts; each opens in its own pane
- Choose a corpus version with `@`: `DT-UCPH/sp@7.2.4`, or `@latest` for the newest on this computer (by default the version named in the corpus' app config); the corpus card reports the version actually loaded
- Corpus library: open other corpora found on this computer while the browser runs (`--no-library` switches this off)
- Template problems as structured data (line, message, feature or node type), including problems inside quantifiers that the search engine only logs
