# Text-Fabric Usage Research Index

**Purpose:** Research index for informing the `cfabric-skill` PRD. Documents how researchers use Text-Fabric (TF) in practice, based on analysis of GitHub notebooks, SHEBANQ queries, academic publications, and community patterns.

**Date:** 2026-03-11

---

## 1. Researchers & Their Notebooks

### 1.1 Cody Kingham (codykingham)

| Repository | Corpus | Task | Key APIs & Libraries |
|-----------|--------|------|---------------------|
| `verb_semantics` | BHSA + custom semantic domains | Semantic clustering of Hebrew verbs by argument structure | `F`, `E`, `L`, `S`, `Bhsa`, pandas, sklearn (KMeans), matplotlib |
| `verb_semantics/data_validation` | BHSA | Validating verb frame experiments, complex search templates | `S.search()` with `/with/`, `/without/`, quantifiers; `E.mother.f()` |
| `tfNotebooks/vocabulary_gen` | BHSA | Export rare Hebrew vocab (hapax, freq<=5) to CSV | `F.freq_lex`, `F.voc_lex_utf8`, `F.gloss`, `L.u()`, csv export |
| `tfNotebooks/plain_texts/generate_txts` | BHSA, Quran, Tischendorf GNT, NENA | Generate plain text files from multiple TF corpora | `use()` app loading, `F.otype.s()`, `L.d()`, `T.text()` |
| `ETCBC/sbl_berlin17_textfabric` | BHSA + phono | SBL 2017 presentation: TF data model, features, text formats | `Fabric()`, multi-module loading, custom display functions, `T.formats` |

**Additional notebooks found (60+ total across 6 repos):**

| Repository | Corpus | Task | Key APIs |
|-----------|--------|------|---------|
| `tfNotebooks/semantic_patterns` | BHSA | Simile constructions with Hebrew particle "K" | `S.study()`, `S.fetch()`, `S.glean()` |
| `tfNotebooks/DSS_Info` | DSS | Scroll inventory, word counts by scroll type | `use('dss')`, seaborn visualization |
| `tfNotebooks/HB_person_distribution` | BHSA | Grammatical person distribution across narrative vs. quotation | `F.ps`, `F.prs_ps`, `F.domain`, seaborn |
| `tfNotebooks/Pentateuch_formulaic_language` | BHSA | Formulaic verse detection via Levenshtein distance | `T.text(verse, fmt='lex-orig-plain')`, heatmap |
| `tfNotebooks/Psalms_parallels` | BHSA | Lexical parallels between Psalm 1 and other Psalms | Set intersection on lexeme sets |
| `tfNotebooks/timeSpans` | BHSA | Time span tracking via clause chains | `E.mother.f()` recursive tree climbing |
| `tfNotebooks/conditional_clauses` | BHSA | Hebrew conditional clauses (>M) | `E.mother.t()`, regex clause type matching |
| `tfNotebooks/word_vectors` | BHSA | Word2Vec distributional semantics for Hebrew | Co-occurrence matrices, Gephi export, dendrograms |
| `tfNotebooks/שכבValency` | BHSA | Full valency analysis of verb shkb | Multi-source semantic classification, CSV export |
| `tfNotebooks/4Q246_Participants` | BHSA | Participant tracking in Dead Sea Scroll 4Q246 | PGN matching, `E.mother.f/t()`, CSV annotation |
| `tfNotebooks/linguistic_dating` | BHSA | Early vs. Late Biblical Hebrew feature export | ML feature extraction pipeline |
| `tfNotebooks/tf_corpus_tutorial` | Custom | Building a TF corpus from scratch | `CV` walker converter, `TF.save()` |
| `Verb_in_Biblical_Hebrew` | BHSA + phono | Time phrase / verb tense association analysis | Custom `is_weqt()`, R export pipeline |
| `catss_lxx` | CATSS LXX | Greek Septuagint TF corpus creation | `TF.save()`, beta-code conversion, cross-corpus alignment |
| `pyling` Ch.23 | Syriac NT | Teaching tutorial on TF concepts | `N()`, `A.pretty()`, `.tf` file inspection |

**Notable patterns:**
- Heavy use of **pandas DataFrames** for organizing TF query results
- **sklearn integration** for clustering and dimensionality reduction on linguistic data
- **Custom experiment frameworks** (`ExperimentFrame`, `SemSpace`) built on top of TF
- **Complex search templates** with quantifiers (`/with/`, `/without/`, `/or/`)
- **Multi-corpus workflows** loading TF data from different sources in same session
- **CSV/pickle export** for sharing results and caching expensive computations
- **Corpus creation** (`TF.save()`, walker converter `CV`) for LXX and custom corpora
- **Word2Vec / distributional semantics** built on TF-extracted co-occurrence data
- **Cross-corpus linking** (Hebrew BHSA ↔ Greek LXX alignment)

**Recurring pain points (from detailed notebook analysis):**
1. **Lexeme dereferencing** - `L.u(word, otype='lex')[0]` repeated dozens of times; always needs `[0]` indexing
2. **No weqetal detection** - ETCBC marks weqetal as regular qatal; custom `is_weqt()` function needed in multiple repos
3. **Linear lexeme lookup** - `next((l for l in F.otype.s('lex') if F.lex.v(l) == lex), None)` - no built-in index
4. **Tree traversal** - No built-in mother-daughter traversal; manual recursive generators everywhere
5. **Edge feature awkwardness** - `E.mother.f(node)` returns tuple that may be empty, requiring guards
6. **Feature loading verbosity** - Long multi-line feature list strings repeated in every notebook

### 1.2 Dirk Roorda (annotation org / ETCBC)

| Repository | Corpus | Task | Key APIs |
|-----------|--------|------|---------|
| `ETCBC/bhsa/tutorial/cookbook/edges` | BHSA | Edge traversal patterns, mother relationships | `E.mother.f()`, `E.mother.freqList()`, `N.walk()` |
| `ETCBC/bhsa/tutorial/cookbook/namedEntity` | BHSA | Named entity recognition using `nametype` feature | `F.nametype.freqList()`, `A.search()`, `A.show()` |
| `ETCBC/bhsa/tutorial/cookbook/text-fabric-api` | BHSA | Comprehensive API overview | `A.plain()`, `A.pretty()`, `A.nodeFromSectionStr()`, `T.formats`, `L.d()` |
| `ETCBC/bhsa/tutorial/cookbook/export` | BHSA | Data export patterns | Export to various formats |
| `ETCBC/bhsa/tutorial/cookbook/lexemes` | BHSA | Lexeme access and frequency analysis | `L.u(word, 'lex')`, lexeme features |
| `ETCBC/bhsa/tutorial/searchSets` | BHSA | Custom node sets for search | `S.search()` with custom sets |
| `ETCBC/bhsa/tutorial/searchFromMQL` | BHSA | MQL to TF query migration | MQL vs TF search syntax comparison |
| `Nino-cunei/uruk/tutorial/` | Proto-cuneiform | Sign analysis, quad structure, lineart | `A.atfFromSign()`, `E.sub.f()`, `E.op.f()`, `A.lineart()` |
| `Nino-cunei/oldassyrian/tutorial/` | Old Assyrian | Search, display, export to Excel, similar lines | `A.search()`, Excel export |

**BHSA cookbook notebooks (25 total):**
`accentPatterns`, `accents`, `books`, `chapters`, `condenseType`, `edges`, `export`, `featureValues`, `firstOcc`, `lexemes`, `loadextrafeatures`, `namedEntity`, `nametype`, `nametype2`, `negative`, `nerByTheBook`, `phraseTranslations`, `roots`, `text-fabric-api`, `text`, `tfVsAlpino`, `wordPatterns`

**Notable patterns:**
- **A (App) object** heavily used: `A.pretty()`, `A.show()`, `A.table()`, `A.search()`, `A.lineart()`
- **Recursive edge traversal** for complex structures (cuneiform quads)
- **Custom display/presentation functions** built on TF
- **Excel export** for non-programmer collaborators
- **MQL-to-TF migration guide** (critical for SHEBANQ users)

### 1.3 Martijn Naaijer

| Repository | Corpus | Task | Key APIs |
|-----------|--------|------|---------|
| `MartijnNaaiworker/...` | BHSA | Clause type analysis (WXIm, etc.) | `S.search()`, clause features |
| SHEBANQ queries | BHSA | Various clause type searches | MQL queries on clause_atom types |
| PhD research | BHSA | Clause structure variation, sequence analysis | `F`, `L`, `S`, LSTM neural networks, TensorFlow |

**Notable patterns:**
- **Machine learning on TF data**: LSTM networks for sequence modeling of Hebrew clause structure
- **TF as data extraction pipeline** feeding into sklearn/TensorFlow
- **GPU-accelerated deep learning** on linguistic features extracted from TF

### 1.4 Oliver Glanz (Andrews University)

| Source | Corpus | Task | Key Features |
|--------|--------|------|-------------|
| SHEBANQ queries | BHSA | Jussive verb form identification | Morphological regex: `vt`, `vs`, `g_word`, `g_prs` |
| SHEBANQ queries | BHSA | KPR lexeme with objects | `lex`, `function` (Objc, PreO) |
| SHEBANQ queries | BHSA | Numbers in Lev 26 | `ls=card`, `prs=absent`, phrase function ordering |
| SHEBANQ queries | BHSA | Genesis 4:7 analysis | Infinitive construct identification |

**Notable patterns:**
- **Complex morphological queries** combining verb form, stem, graphical word form
- **Word order permutation searches** (all orderings of Pred/Obj/Cmpl)
- **Regex on Hebrew graphemes** for vowel pattern detection
- **Theological research questions** driving corpus queries

### 1.5 Tony Jurg (tonyjurg)

| Repository | Corpus | Task | Key APIs |
|-----------|--------|------|---------|
| `Parashot` | BHSA + BHSaddons | Parashah statistics (verse/sentence/clause/word counts) | `A.search()`, `F.domain.v()`, `F.trailer.freqList()`, pandas |
| `Nestle1904LFT` | Greek NT (Nestle 1904) | Word study (monogenes) | `F.lemma.v()`, `F.otype.s()`, `T.sectionFromNode()` |
| `Nestle1904GBI` | Greek NT | Syntax tree visualization | Syntax tree display, HTML rendering |
| `TF_Knowledge_Graph` | N1904-TF | Knowledge graph generation + Cytoscape visualization | JSON export, Cytoscape.js interactive HTML |

**Notable patterns:**
- **Non-Hebrew corpus** (Greek NT) with TF
- **Interactive visualization** via Cytoscape.js knowledge graphs
- **Parashah/liturgical division** analysis extending TF with custom features
- **Cross-version comparison** of query results

### 1.6 Other Researchers

| Researcher | Repository | Corpus | Task |
|-----------|-----------|--------|------|
| Lucie Perez | `lucieperez/notebooks` | BHSA + DSS (1QIsa) | Annotation tools, dataset generation, textual criticism, MT-DSS alignment |
| Christiaan Erwich | `cmerwich/participant-analysis` | BHSA (Psalms) | Coreference resolution, participant tracking |
| Janet Dyk | ETCBC/valence | BHSA | Verbal valence patterns, SYNVAR project flowcharts |
| Wido van Peursen | ETCBC publications | BHSA | Syntactic variation, language change detection |
| Saulo Cantanhede | `saulocantanhede/tfgreek2` | Greek NT | TF conversion, Greek text analysis |
| James Cuenod | `jcuenod/tf-accent-data` | BHSA | Hebrew accent/cantillation data generation |
| ETCBC/ssi_morphology | BHSA + Syriac | Morphological analysis | Neural networks (attention models), bidirectional LSTMs |
| DT-UCPH | `DT-UCPH/cuc` | Ugaritic | Cuneiform Ugaritic corpus in TF |
| BYUIDSS | `BYUIDSS/text-fabric-cookbook` | Various | Cookbook: word counts, searching, linguistic analysis, textual criticism |
| Yonatan Lou | `yonatanlou/QumranNLP` | DSS | Hebrew BERT + Graph Neural Networks for authorial clustering of Dead Sea Scrolls |
| Albert De La Fuente | Blog post | BHSA | Literate programming with Doom Emacs + org-mode + TF |
| CLARIAH/wp6-missieven | General Missives | Dutch 17th-century letters in TF |
| Folgert Karsdorp | `fbkarsdorp/python-course` | Various | Python for Humanities course with TF appendix by Roorda |

---

## 2. Corpora Used with Text-Fabric

| Corpus | Repository | Language | Annotation Level |
|--------|-----------|----------|-----------------|
| **BHSA** | ETCBC/bhsa | Biblical Hebrew | Full morpho-syntactic (word→phrase→clause→sentence→verse→chapter→book) |
| **Dead Sea Scrolls** | ETCBC/dss | Hebrew/Aramaic | Morphological + textual |
| **Peshitta** | ETCBC/peshitta | Syriac | Morpho-syntactic |
| **Syriac NT** | ETCBC/syrnt | Syriac | Morpho-syntactic |
| **Nestle 1904 GNT** (LFT) | tonyjurg/Nestle1904LFT | Greek | Syntactic trees |
| **Nestle 1904 GNT** (GBI) | tonyjurg/Nestle1904GBI | Greek | Syntax diagrams |
| **Tischendorf GNT** | codykingham/tischendorf_tf | Greek | Morphological |
| **Quran** | q-ran/quran | Arabic | Morphological + syntactic |
| **Fusus al-Hikam** | among/fusus | Arabic | OCR + text alignment |
| **Proto-cuneiform Uruk** | Nino-cunei/uruk | Sumerian | Sign/grapheme level |
| **Old Assyrian** | Nino-cunei/oldbabylonian | Akkadian | Cuneiform tablets |
| **Old Babylonian** | Nino-cunei/oldbabylonian | Akkadian | Cuneiform tablets |
| **Ugaritic** | DT-UCPH/cuc | Ugaritic | 278 tablets from KTU |
| **NENA** | CambridgeSemiticsLab/nena_tf | Neo-Aramaic | Oral texts |
| **Greek Literature** | pthu/greek_literature | Ancient Greek | Perseus Digital Library + OGL |
| **Mondriaan Letters** | annotation/mondriaan | Dutch | TEI conversion |
| **Moby Dick** | (built-in) | English | Literary annotation |
| **General Missives** | CLARIAH/wp6-missieven | Dutch | Historical letters |
| **Extrabiblical** | ETCBC/extrabiblical | Hebrew | Non-masoretic texts |
| **Samaritan Pentateuch** | DT-UCPH/sp | Hebrew | Word-level linguistic annotations |
| **CATSS LXX** | codykingham/catss_lxx | Greek | Morphologically tagged Septuagint |
| **SBLGNT** | CenterBLC/SBLGNT | Greek | SBL Greek NT with enriched features |
| **Nineveh Medical** | Nino-cunei/ninmed | Akkadian | Medical texts from Nineveh |
| **Sanskrit** | dirkroorda/text-fabric-data | Sanskrit | Experimental |

---

## 3. SHEBANQ Query Patterns

SHEBANQ uses MQL (Monads Query Language / Emdros) rather than TF search templates, but the *types of queries* reveal what researchers need.

### 3.1 Query Categories Found

| Category | Example Queries | Authors |
|----------|----------------|---------|
| **Verbal morphology** | Jussive forms by morphological pattern, verb stem distributions | Oliver Glanz |
| **Clause type analysis** | WXIm clauses, clause atom types | Martijn Naaijer |
| **Lexeme collocations** | KPR with objects, NTN with objects+complements, C<N <L JD | Glanz, Dyk, van Peursen |
| **Word order** | All permutations of Pred/Obj/Cmpl in clauses | Janet Dyk |
| **Language identification** | Aramaic sections of Hebrew Bible | Dirk Roorda |
| **Syntactic incongruency** | Imperative plural + singular vocative | Dirk Roorda |
| **Theological vocabulary** | BRK blessing patterns by stem (active vs. passive) | Seungho Park |
| **Number/quantity** | Cardinals in Leviticus 26 with specific functions | Oliver Glanz |
| **Specific verse analysis** | Genesis 4:7 "a lifting" - infinitive construct identification | Oliver Glanz |

### 3.2 Common MQL Features Used

- `lex` (lexeme), `vs` (verbal stem), `vt` (verbal tense), `sp` (part of speech)
- `function` (phrase function: Pred, Objc, Cmpl, Adju, Subj, Voct, etc.)
- `typ` (clause type), `domain` (narrative, quotation, discursive)
- `g_word` (graphical word form), `g_prs` (pronominal suffix form)
- `language` / `languageISO` (Hebrew vs. Aramaic)
- `ls` (lexical set: card, nmpr, gntl)
- `ps` (person), `nu` (number), `gn` (gender)
- `prs` (pronominal suffix)
- `nametype` (pers, topo, gens, mens, god)
- Regex matching on graphemes (`g_word ~`)
- `NOTEXIST` / negative constraints
- Distance operators (`..`) for word/phrase ordering

---

## 4. Analysis Workflow Patterns

### 4.1 Data Extraction Pipeline

```
TF Corpus → Feature Access (F/E/L/T/S) → Python Data Structures → Analysis
```

Common pipeline stages observed:
1. **Load corpus** (`Fabric()` or `use()`)
2. **Query/filter** (`S.search()`, `F.feature.s()`, iteration)
3. **Extract features** into lists/dicts (`F.feature.v()`, `L.d()`, `L.u()`)
4. **Transform** to pandas DataFrame
5. **Analyze** with statistical/ML tools
6. **Export** results (CSV, JSON, pickle, Excel)

### 4.2 Frequency & Distribution Analysis

Observed in nearly every notebook:
- `F.feature.freqList()` for top-N distributions
- `collections.Counter` for custom counting
- Cross-tabulation of features (e.g., verb stem x book)
- Hapax legomena identification (`freq_lex == 1`)

### 4.3 Search-Based Analysis

- Simple feature filtering: `F.sp.s('verb')` to find all verbs
- Template search: `S.search()` with multi-level structural patterns
- Complex quantifiers: `/with/`, `/without/`, `/or/` in search templates
- Result iteration with feature extraction
- Custom node sets for search scope restriction

### 4.4 Hierarchical Navigation

- **Downward**: `L.d(verse, 'word')` - get all words in a verse
- **Upward**: `L.u(word, 'verse')` - get verse containing a word
- **Siblings**: `L.n(node)` / `L.p(node)` - next/previous
- **Cross-type**: Navigate clause→phrase→word, with feature extraction at each level
- **Edge traversal**: `E.mother.f(node)` for syntactic dependencies

### 4.5 Text Extraction & Display

- `T.text(nodes, fmt='...')` with various format strings
- `T.sectionFromNode(node)` for human-readable references
- `A.pretty()` / `A.show()` for rich Jupyter display
- Custom HTML/CSS rendering for presentations
- Multiple text formats (Hebrew, transliteration, phonetic)

### 4.6 Batch/Corpus-Wide Analysis

- `F.feature.items()` for iterating all (node, value) pairs
- `N.walk()` for walking all nodes
- `F.otype.s(type)` for all nodes of a type
- Large-scale feature extraction into DataFrames
- Caching results with pickle/dill for expensive computations

---

## 5. Integration Patterns

### 5.1 Data Science Stack

| Library | Usage with TF | Observed In |
|---------|--------------|-------------|
| **pandas** | DataFrames for feature tables, cross-tabulation | Kingham, Naaijer, Jurg, Perez |
| **matplotlib** | Frequency plots, histograms, scatter plots | Kingham, Naaijer |
| **sklearn** | KMeans clustering, dimensionality reduction | Kingham (verb_semantics) |
| **TensorFlow/Keras** | LSTM sequence models for clause analysis | Naaijer (PhD) |
| **numpy** | Numerical operations on feature matrices | Kingham, Naaijer |
| **collections.Counter** | Custom frequency counting | Nearly universal |
| **csv** | Simple tabular export | Kingham, Roorda |
| **pickle/dill** | Caching expensive computation results | Kingham |

### 5.2 Visualization

| Tool | Usage | Observed In |
|------|-------|-------------|
| **A.pretty() / A.show()** | Rich Jupyter display of search results | Universal |
| **A.table()** | Tabular result display | Roorda tutorials |
| **A.lineart()** | Cuneiform sign visualization | Roorda (Uruk) |
| **Cytoscape.js** | Interactive knowledge graphs from TF data | Jurg |
| **matplotlib** | Charts and plots | Kingham, Naaijer |
| **Custom HTML/CSS** | Presentation-quality displays | Kingham (SBL) |
| **pandas .style** | Styled DataFrame output | Various |

### 5.3 Export Formats

| Format | Usage | Observed In |
|--------|-------|-------------|
| **CSV/TSV** | Feature tables, vocabulary lists | Kingham, Roorda |
| **JSON** | Structured data, knowledge graphs | Jurg |
| **Excel** | For non-programmer collaborators | Roorda (Old Assyrian) |
| **Pickle/Dill** | Python object serialization for caching | Kingham |
| **Parquet** | Efficient columnar storage via TF's pandas module | Roorda (tf.convert.pandas) |
| **Plain text** | Generated text files from corpora | Kingham |
| **HTML** | Interactive visualizations | Jurg, Kingham |

### 5.4 External Annotation Round-Trips

TF includes a `Recorder` module (`tf.convert.recorder`) for:
1. Exporting corpus text to plain text with position-to-node mappings
2. Annotating in external tools
3. Re-importing annotations as new TF features

This pattern is used by: Lucie Perez (DSS annotation), Erwich (coreference annotation)

---

## 6. Academic Research Workflows

### 6.1 Morphological Analysis
- Part-of-speech distribution across books/sections
- Verb stem (binyan) frequency analysis
- Verbal tense/aspect distribution
- Pronominal suffix analysis
- Morphological pattern matching with regex

### 6.2 Syntactic Analysis
- Clause type classification and distribution
- Word order variation analysis (VSO, SVO permutations)
- Phrase function analysis (Pred, Obj, Cmpl, Adju)
- Subordinate clause identification
- Syntactic embedding depth analysis

### 6.3 Lexical/Semantic Analysis
- Lexeme frequency and distribution
- Semantic domain clustering
- Verbal valence frame analysis (what arguments a verb takes)
- Collocation analysis (which words appear together)
- Named entity recognition and classification
- Hapax legomena study

### 6.4 Textual Criticism
- MT (Masoretic Text) vs. DSS comparison
- Parallel passage identification and comparison
- Textual variant cataloging
- Cross-version alignment

### 6.5 Diachronic/Sociolinguistic Analysis
- "Early" vs. "Late" Biblical Hebrew feature comparison
- Genre-conditioned variation (prose vs. poetry, narrative vs. speech)
- Syntactic variation as proxy for language change
- Statistical modeling of linguistic change probability

### 6.6 Discourse Analysis
- Participant tracking/coreference resolution
- Domain analysis (narrative, quotation, discursive)
- Clause connection patterns
- Text segmentation by discourse features

### 6.7 Pedagogical/Reference
- Vocabulary list generation (by frequency, by book)
- Text generation for reading/study
- Accent/cantillation analysis
- Parashah (liturgical reading) statistics

---

## 7. PRD Gap Analysis

Based on this research, the following gaps and refinements should be considered for the `cfabric-skill` PRD:

### 7.1 Missing Workflow Support

| Gap | Evidence | PRD Impact |
|-----|----------|-----------|
| **Pandas integration** | Used in nearly every research notebook | Add `pandas_export.py` script; document DataFrame patterns in SKILL.md |
| **ML pipeline support** | Kingham (sklearn), Naaijer (TF/LSTM) | Add `ml_prep.py` script for feature matrix extraction |
| **Multi-corpus loading** | Kingham loads BHSA + custom modules together | Document multi-source loading pattern |
| **Caching expensive results** | pickle/dill used widely | Add caching guidance in reference docs |
| **Custom feature creation** | Valence project, annotation round-trips | Document `TF.save()` and feature writing patterns |
| **Visualization guidance** | matplotlib, Cytoscape common | Add `visualization_patterns.md` reference |

### 7.2 Missing Reference Content

| Gap | Evidence | PRD Impact |
|-----|----------|-----------|
| **MQL-to-CF migration guide** | Roorda's `searchFromMQL` notebook; SHEBANQ users | Add `references/mql-migration.md` |
| **Common BHSA features reference** | Every notebook loads specific features | Add corpus-specific feature catalogs |
| **Search template cookbook** | Complex patterns with quantifiers widespread | Expand `search-syntax.md` with real examples |
| **Edge feature patterns** | Cuneiform quads, mother edges heavily used | Expand `navigation-patterns.md` with edge examples |
| **Text format reference** | Multiple formats (Hebrew, transliteration, phonetic) | Add text format documentation |

### 7.3 Missing User Stories

| User Story | Evidence |
|-----------|----------|
| **Textual critic comparing versions** | Perez DSS work, ETCBC/parallels |
| **ML researcher building feature matrices** | Kingham, Naaijer |
| **Teacher generating study materials** | Kingham vocabulary lists, BYUIDSS cookbook |
| **Non-Hebrew corpus user** | Greek NT, Quran, cuneiform, Syriac, Ugaritic users |
| **Annotation workflow user** | Perez, Erwich using Recorder for external tools |
| **SHEBANQ migrant** | Users transitioning from MQL to TF search |

### 7.4 Script Additions

| Script | Purpose | Evidence |
|--------|---------|---------|
| `pandas_export.py` | Export node features to pandas DataFrame | Universal pattern |
| `frequency_analysis.py` | Feature distribution, cross-tabulation, hapax | Most common analysis type |
| `collocation.py` | Co-occurrence analysis for lexemes/features | Kingham, Dyk, SHEBANQ queries |
| `text_comparison.py` | Compare parallel passages or versions | Perez, ETCBC/parallels |
| `section_filter.py` | Filter results by book/chapter/section | Universal need |
| `ml_prep.py` | Extract feature matrices for ML pipelines | Kingham, Naaijer |

### 7.5 Missing from Helper Scripts

| Gap | Evidence | Recommendation |
|-----|----------|----------------|
| **Corpus creation** | Kingham's catss_lxx uses `TF.save()` + walker `CV` | Not needed in v1 but document the pattern |
| **Annotation round-trip** | Perez, Erwich use `Recorder` module extensively | Add reference doc on `Recorder` workflow |
| **Cross-corpus alignment** | Kingham links Hebrew BHSA ↔ Greek LXX; Perez aligns MT ↔ DSS | Future: `alignment.py` script |
| **Distributional semantics** | Kingham builds Word2Vec, co-occurrence matrices, PMI | Future: `distributional.py` for semantic space construction |

### 7.6 Key Pain Points to Address in Skill Design

| Pain Point | Frequency | Skill Response |
|-----------|-----------|---------------|
| `L.u(word, 'lex')[0]` verbose lexeme access | Every notebook | Document shorthand patterns; consider helper |
| No built-in tree traversal for clause chains | ~30% of notebooks | Add `navigation.py` with `climb_tree()` helper |
| Linear lexeme lookup (no index) | Multiple notebooks | Document `F.lex.s(value)` as the indexed alternative |
| Template search + manual iteration tension | ~25% of notebooks | Document when to use each approach in reference docs |
| Feature loading verbosity | Every notebook | Skill can suggest `loadAll()` or common feature sets |
| Edge feature empty-tuple guards | ~30% of notebooks | Document safe access patterns |

### 7.7 Architecture Considerations

| Consideration | Evidence | Recommendation |
|--------------|----------|----------------|
| **A (App) object** is primary entry point | Roorda tutorials use `A = use(...)` not `Fabric()` | Support both `Fabric()` and `use()` patterns in SKILL.md |
| **`makeAvailableIn(globals())`** is universal | Every notebook uses this | Make this the default in quick-start |
| **Rich display methods** (`A.pretty`, `A.show`) | Used extensively but require Jupyter | Note Jupyter-only features; provide CLI alternatives |
| **Custom features loaded from multiple paths** | Kingham loads BHSA + heads + sdbh | Document multi-path loading |
| **Session caching** | Users pickle results to avoid recomputation | Consider session state in future skill versions |

---

## 8. Source Index

### GitHub Repositories (by stars)
- `annotation/text-fabric` (78 stars) - Core library
- `ETCBC/bhsa` (65 stars) - Hebrew Bible dataset
- `ETCBC/shebanq` (37 stars) - Web query interface
- `ETCBC/peshitta` (27 stars) - Syriac OT
- `ETCBC/syrnt` (13 stars) - Syriac NT
- `codykingham/tfNotebooks` (8 stars) - Misc TF notebooks
- `DT-UCPH/cuc` (5 stars) - Ugaritic corpus
- `CLARIAH/wp6-missieven` (5 stars) - Dutch historical letters
- `Context-Fabric/context-fabric` (4 stars) - CF (TF successor)
- `Nino-cunei/oldbabylonian` (4 stars) - Old Babylonian
- `Nino-cunei/uruk` (3 stars) - Proto-cuneiform
- `kungsik/kimsbible` (3 stars) - Korean Bible study app
- `codykingham/tischendorf_tf` (3 stars) - Greek NT
- `jcuenod/tf-accent-data` (3 stars) - Hebrew accents

### SHEBANQ Query Authors
- Oliver Glanz (Andrews University) - Sanctuary theology, morphological queries
- Janet Dyk (ETCBC/VU) - Verbal valence, word order
- Dirk Roorda (DANS) - Aramaic identification, syntactic tests
- Martijn Naaijer - Clause type analysis
- Willem van Peursen (ETCBC/VU) - Lexeme collocations
- Seungho Park (Andrews) - Blessing theology

### Academic Publications
- Roorda, D. (2014). "LAF-Fabric: a data analysis tool for Linguistic Annotation Framework" (arXiv:1410.0286)
- Roorda, D. (2018). "Coding the Hebrew Bible." Research Data Journal for the Humanities and Social Sciences 3(1)
- Erwich, C. - "Who is Who in the Psalms?" PhD, VU Amsterdam (coreference resolution)
- Erwich, C. & Kingham, C. (2017). "Text Fabric: What, How, and Why." SBL International Meeting, Berlin
- Cantanhede, S. (2025). "Coding the Greek New Testament in text-fabric" Thesis, Andrews University
- Van Peursen, W. (2019). "A Computational Approach to Syntactic Diversity in the Hebrew Bible"
- Dyk, J.W. (2014). "Analysing Valence Patterns in Biblical Hebrew" (JNSL)
- Kingham, C. & van Peursen, W. (2018). "The ETCBC Database"
- Kingham, C. (2021). "Parts of Speech in Biblical Hebrew Time Phrases: A Cognitive-Statistical Analysis" (Open Book Publishers)
- Samaritan Pentateuch TF Dataset (2024). Research Data Journal for the Humanities and Social Sciences 9(1)

### Workshops & Events
- Lorentz Center, Leiden (2020): "Processing Ancient Text Corpora" - 25 scholars, hands-on with DSS, Old Babylonian, Quran via TF
- SBL International Meeting, Berlin (2017): "Text Fabric: What, How, and Why" by Erwich & Kingham
- VU Amsterdam MA Program: "Biblical Studies and Digital Humanities" specialization uses TF as core tool

---

*End of Research Index*
