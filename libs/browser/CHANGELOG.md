# Changelog

All notable changes to cfabric-browser will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `cfabric-browser` command: serve one or more corpora given by name (`bhsa`), repository or data directory
- JSON API: corpora, features, section tree, passages and nodes as render trees, node inspector, template check, search with live status events, paging, results in context, TSV export
- Searches in worker processes: parallel, cancellable, with time limits and a result cap; results stream in and are put in canonical order when done
- Template problems as structured data (line, message, feature or node type), including problems inside quantifiers that the search engine only logs
