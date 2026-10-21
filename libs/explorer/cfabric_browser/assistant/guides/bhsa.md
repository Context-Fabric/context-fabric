# Searching the BHSA

The BHSA (Biblia Hebraica Stuttgartensia Amstelodamensis, ETCBC, version 2021) is the Hebrew Bible with linguistic annotations on words, phrases, clauses and sentences. A search template describes objects, one per line; indentation means "inside". The browser runs it; you only write it.

## Object types (how many), from large to small

book (39) > chapter (929) > verse (23,213) > half_verse (45,179) > sentence (63,717) > clause (88,131) > phrase (253,203) > word (426,590).

Also: `sentence_atom`, `clause_atom`, `phrase_atom`, `subphrase` (finer divisions), and `lex` (9,230 lexemes). A `lex` node contains all the words of that lexeme, so `lex gloss=create` with `word` indented under it finds the words of lexemes glossed "create".

## The main features

**Words** (`word`):

- `lex`: the lexeme in ETCBC transliteration (see *Conventions*): `>LHJM/`, `HLK[`, `JHWH/`.
- `sp`: part of speech: `subs` noun, `verb`, `prep` preposition, `conj` conjunction, `nmpr` proper noun, `art` article, `adjv` adjective, `nega` negation, `prps` personal pronoun, `advb` adverb, `prde` demonstrative pronoun, `intj` interjection, `inrg` interrogative particle, `prin` interrogative pronoun.
- `vs`: verbal stem: `qal`, `nif` niphal, `piel`, `pual`, `hit` hitpael, `hif` hiphil, `hof` hophal, and rarer ones (`hsht`, `poel`, ...); Aramaic `peal`, `pael`, `haf`, `afel`, `peil`, `htpe`, `htpa`, ... `NA` for words that aren't verbs.
- `vt`: verbal tense: `perf` perfect (qatal), `impf` imperfect (yiqtol), `wayq` wayyiqtol, `impv` imperative, `infc` infinitive construct, `infa` infinitive absolute, `ptca` active participle, `ptcp` passive participle.
- `ps` person (`p1`, `p2`, `p3`), `gn` gender (`m`, `f`), `nu` number (`sg`, `pl`, `du`); `unknown` where it can't be told (common gender, for instance), `NA` where it doesn't apply.
- `st`: state: `a` absolute, `c` construct, `e` emphatic (Aramaic).
- Pronominal suffix: `prs` its consonants (`W`, `K`, `HM`, ...; `absent` without one), with `prs_ps`, `prs_gn`, `prs_nu`.
- `language`: `Hebrew` or `Aramaic` (6,106 Aramaic words, in Daniel, Ezra and a few verses elsewhere).
- `nametype` of proper nouns: `pers` person, `topo` place, `gens` people, `ppde`, `mens`, and combinations such as `pers,gens,topo`: use `nametype~topo` to include those.
- `ls`: lexical set: `card` cardinal, `ordn` ordinal, `gntl` gentilic, `quot` quotation verb (אמר), `ques`, `nmdi` distributive noun, ...
- Text: `g_cons` (consonants in transliteration, without vowels: `>LHJM`), `g_word_utf8` (the word in Hebrew with vowels), `g_lex_utf8`.
- `gloss` (English) is on `lex` nodes, not on words.

**Phrases** (`phrase`):

- `function`: what the phrase does in its clause: `Pred` predicate, `Subj` subject, `Objc` object, `Cmpl` complement, `Adju` adjunct, `Time`, `Loca` location, `Modi` modifier, `Conj` conjunction, `Nega` negation, `Ques` question, `Rela` relative, `Voct` vocative, `Intj`, `PreC` predicate complement, `PreO` predicate with object suffix, `PreS` predicate with subject suffix, `Frnt` fronted element, `Supp` supplementary constituent, ...
- `typ`: phrase type: `VP` verbal, `NP` nominal, `PP` prepositional, `CP` conjunctive, `PrNP` proper-noun phrase, `AdvP`, `NegP`, `InrP`, `AdjP`, `PPrP` personal pronoun, `DPrP` demonstrative pronoun, ...
- `det`: determination: `det`, `und` (undetermined).

**Clauses** (`clause`):

- `typ`: clause type, by the form of its verb and what comes before it: `Way0` wayyiqtol first, `WayX` wayyiqtol after something, `WQt0` weqatal first, `xQt0` qatal after a word, `xQtX` qatal after a constituent, `XQtl` qatal after a non-subject, `ZQt0`, `ZYq0` (no conjunction), `xYq0`, `WYq0` (yiqtol), `ZIm0` imperative, `NmCl` nominal clause, `AjCl` adjectival, `InfC` infinitive construct clause, `Ptcp` participle clause, `Ellp` ellipsis, `Voct`, `Defc` defective, `CPen` casus pendens, `MSyn` macrosyntactic sign, ...
- `rela`: relation to another clause: `Objc` object clause, `Subj` subject clause, `Attr` attributive (relative), `Adju` adjunct, `Cmpl`, `PreC`, `RgRc` regens/rectum, `Resu` resumptive, `Coor` coordinated, `Spec`, `Appo`, `Link`, ...; `NA` for main clauses.
- `kind`: `VC` verbal clause, `NC` nominal clause, `WP` without predicate.
- `domain`: `N` narrative, `Q` quotation (direct speech), `D` discursive, `?` unknown.
- `txt`: text type by levels of embedding (`N`, `Q`, `NQ`, `NQQ`, ...).

**Sections**: `book book=Genesis`, `chapter chapter=1`, `verse verse=3`, indented in that order.

## Conventions

- **Lexemes** are written in ETCBC transliteration: `>` א, `B` ב, `G` ג, `D` ד, `H` ה, `W` ו, `Z` ז, `X` ח, `V` ט, `J` י, `K` כ, `L` ל, `M` מ, `N` נ, `S` ס, `<` ע, `P` פ, `Y` צ, `Q` ק, `R` ר, `F` שׂ, `C` שׁ, `T` ת (final forms the same).
- A verb's lexeme ends in `[` (`HLK[` to go, `BR>[` to create), a noun's or adjective's in `/` (`>LHJM/` god(s), `MLK/` king, `VWB/` good), particles have neither (`L` to, `>CR` which, `KJ` that, `W` and, `H` the article, `H=` the interrogative he).
- `=` marks homonyms: `DBR[` speak and `DBR/` word; `>MR[` say; `H` the article but `H=` the interrogative; `QW=/`, `<RB===/`. When a lexeme seems to give too few results, check its homonyms with `describe_feature` on `lex`, or look for its gloss: `lex gloss=word`.
- **Book names** in the `book` feature are Latin, with Roman numerals after the name: Genesis, Exodus, Leviticus, Numeri, Deuteronomium, Josua, Judices, Samuel_I, Samuel_II, Reges_I, Reges_II, Jesaia, Jeremia, Ezechiel, Hosea, Joel, Amos, Obadia, Jona, Micha, Nahum, Habakuk, Zephania, Haggai, Sacharia, Maleachi, Psalmi, Iob, Proverbia, Ruth, Canticum, Ecclesiastes, Threni, Esther, Daniel, Esra, Nehemia, Chronica_I, Chronica_II. So Psalms is `book book=Psalmi`, 1 Kings is `book book=Reges_I`.
- Values are exact: `vt=infc`, not `vt=inf` or `vt=infinitive`. Several values with `|`: `prs_ps=p1|p2`. Not a value with `#`: `prs#absent` (with a suffix). A pattern with `~`: `nametype~topo`, `gloss~^king` (starting with "king").
- Hebrew and Aramaic words have the same features; add `language=Aramaic` (or `Hebrew`) to keep to one.

## Templates in short

- One object per line; an indented line lies inside the object above it.
- Features after the type: `word sp=verb vs=hif`.
- Two objects at the same level in no particular order, unless related: name them and add a relation on a line of its own: `p1:phrase function=Pred`, `p2:phrase function=Subj`, then `p1 < p2` (p1 before p2). `<:` means "right before" (adjacent).
- Quantifiers, each closed with `/-/`: `/with/` (has this inside), `/without/` (none of these inside), `/where/ … /have/` (every object of the `/where/` part also fits the `/have/` part). In `/have/`, name the object of `/where/` and refer to it: `w:word` under `/where/`, `w sp=nmpr` under `/have/`. A new line such as `word sp=nmpr` under `/have/` is another object, so it only asks that there is one. `/where/` also lets through what has no such object at all; "only proper names" is shorter as `/without/` with `word sp#nmpr`.
- Results are tuples, one node per line of the template; the count is the number of tuples. Ask for the object the user wants counted at the top, or as the only line.

## Worked examples (with their number of results in the BHSA)

| Request | Template | Results |
|---|---|---|
| all cases of >LHJM/ | `word lex=>LHJM/` | 2,601 |
| the verb HLK[ in all its forms | `word lex=HLK[` | 1,554 |
| HLK[ in the qal | `word lex=HLK[ vs=qal` | 1,412 |
| infinitives construct of HLK[ | `word lex=HLK[ vt=infc` | 144 |
| active participles of >MR[ | `word lex=>MR[ vt=ptca` | 138 |
| the niphal of DBR[ | `word lex=DBR[ vs=nif` | 4 |
| hiphil imperfects | `word vs=hif vt=impf` | 2,264 |
| perfects, second person feminine singular | `word vt=perf ps=p2 gn=f nu=sg` | 222 |
| the name YHWH | `word lex=JHWH/` | 6,828 |
| Aramaic words | `word language=Aramaic` | 6,106 |
| place names | `word sp=nmpr nametype=topo` | 6,371 |
| place names, also those that are also personal names | `word sp=nmpr nametype~topo` | 10,916 |
| feminine plural nouns | `word sp=subs gn=f nu=pl` | 5,992 |
| nouns in the construct state | `word sp=subs st=c` | 36,679 |
| verbs with a suffix of the first person | `word sp=verb prs_ps=p1` | 1,649 |
| the interrogative he | `word lex=H=` | 743 |
| words with the consonants >LHJM, of any lexeme | `word g_cons=>LHJM` | 1,177 |
| the words of lexemes glossed "create" | `lex gloss=create` / `  word` | 54 |
| the words of lexemes whose gloss starts with "king" | `lex gloss~^king` / `  word` | 3,001 |
| wayyiqtol forms in Genesis | `book book=Genesis` / `  word vt=wayq` | 2,106 |
| imperatives in the Psalms | `book book=Psalmi` / `  word vt=impv` | 701 |
| >LHJM/ in Genesis 1 | `book book=Genesis` / `  chapter chapter=1` / `    word lex=>LHJM/` | 32 |
| phrases that are the subject | `phrase function=Subj` | 31,979 |
| determined noun phrases | `phrase typ=NP det=det` | 20,476 |
| clauses with a predicate and a subject | `clause` / `  phrase function=Pred` / `  phrase function=Subj` | 18,966 |
| clauses with the predicate before the subject | `clause` / `  p1:phrase function=Pred` / `  p2:phrase function=Subj` / `p1 < p2` | 13,938 |
| clauses with the subject before the predicate | `clause` / `  p1:phrase function=Subj` / `  p2:phrase function=Pred` / `p1 < p2` | 5,028 |
| clauses whose subject is >LHJM/, with a verb as predicate | `clause` / `  phrase function=Pred` / `    word sp=verb` / `  phrase function=Subj` / `    word lex=>LHJM/` | 614 |
| clauses where >LHJM/ is the subject of >MR[ | `clause` / `  phrase function=Pred` / `    word lex=>MR[` / `  phrase function=Subj` / `    word lex=>LHJM/` | 147 |
| clauses with HJH[ as predicate | `clause` / `  phrase function=Pred` / `    word lex=HJH[` | 3,521 |
| nominal clauses | `clause typ=NmCl` | 11,377 |
| wayyiqtol clauses with nothing before the verb | `clause typ=Way0` | 9,038 |
| qatal after a constituent (x-qatal-X) | `clause typ=xQtX` | 2,797 |
| object clauses | `clause rela=Objc` | 1,327 |
| relative clauses with >CR | `clause` / `  phrase function=Rela` / `    word lex=>CR` | 4,738 |
| negated clauses | `clause` / `  phrase function=Nega` | 6,044 |
| clauses in direct speech | `clause domain=Q` | 52,937 |
| an infinitive construct right after the preposition L | `phrase` / `  w1:word lex=L` / `  w2:word vt=infc` / `w1 <: w2` | 4,572 |
| a noun followed right away by an adjective | `phrase typ=NP` / `  w1:word sp=subs` / `  w2:word sp=adjv` / `w1 <: w2` | 1,579 |
| >MR[ followed in its clause by L with a suffix | `clause` / `  w1:word lex=>MR[` / `  w2:word lex=L prs#absent` / `w1 < w2` | 347 |
| verses with both >LHJM/ and BR>[ (21 combinations of a verse and the two words, in 12 verses) | `verse` / `  word lex=>LHJM/` / `  word lex=BR>[` | 21 |
| verses without a verb | `verse` / `/without/` / `  word sp=verb` / `/-/` | 1,607 |
| subjects that are only proper names | `phrase function=Subj` / `/where/` / `  w:word` / `/have/` / `  w sp=nmpr` / `/-/` | 5,893 |
| the same, shorter | `phrase function=Subj` / `/without/` / `  word sp#nmpr` / `/-/` | 5,893 |
| subjects with at least one proper name | `phrase function=Subj` / `/with/` / `  word sp=nmpr` / `/-/` | 10,484 |
| verses with both >HB[ and JHWH/, each verse once | `verse` / `/with/` / `  word lex=>HB[` / `/-/` / `/with/` / `  word lex=JHWH/` / `/-/` | 60 |

(`/` separates the lines of a template here; write each on its own line, with the indentation shown.)

## Common mistakes

- English or Hebrew values: `sp=noun`, `vt=infinitive`, `vs=hiphil` find nothing; use the codes above (`subs`, `infc`, `hif`).
- A lexeme without its ending: `lex=HLK` finds nothing; it is `HLK[`. `try_search` names a value that a feature never has.
- English book names: `book=Psalms` finds nothing; it is `Psalmi`.
- `gloss` on a word: it is on `lex` nodes (`lex gloss=create`, with `word` under it).
- `function` on a word or a clause: it is a feature of phrases.
- The article `H` for the interrogative `H=`, `DBR[` (speak) for `DBR/` (word).
- `/where/` with a new object under `/have/` (`/where/` / `  word` / `/have/` / `  word sp=nmpr`): that finds subjects with at least one proper name (10,484), not subjects of proper names only (5,893). Name the object and refer to it, or use `/without/`.
- Counting: `verse` with two words under it gives a result for every pair of matching words in the verse, not one per verse. To count verses, keep the verse alone with its conditions inside a quantifier, or tell the user that the count is of combinations.
