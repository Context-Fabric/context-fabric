# Search examples

Fifty search templates for the BHSA, from a single word to whole clauses, each with the number of results it finds. They go from words (1–18) to phrases (19–34) and clauses (35–50); [Searching](searching.md#templates) explains how templates work, and the [summary](#summary) below sums up what the examples use.

To try one, open the BHSA, choose *Search* and *Template*, paste the template and click *Search*. Lines that start with `%` are comments. Change a value and search again to see what else is there; *Statistics* and the [Concordance](concordance.md) show the results in other ways.

The numbers are those of the BHSA, version 2021, checked with both cores of Context-Fabric (Python and Rust). A result is one way to match the whole template, so a template with a clause and a phrase in it gives a result for every such pair ([What is counted](searching.md#what-is-counted)).

## Summary

**Conditions** on an object's line, all of which must hold:

| Written | Means |
|---|---|
| `sp=verb` | the feature has this value |
| `vt=wayq\|impf` | one of these values |
| `sp#verb` | not this value |
| `lex~^>L` | matches a regular expression |
| `freq_lex>1000` | more than (also `<`) |
| `qere` | the feature has a value, whatever it is |

**Relations** between two named objects (`w1:word`), on a line of their own (`w1 < w2`), or at the start of a line, relating that object to the one on the line above:

| Written | Means |
|---|---|
| `<`, `>` | before, after (anywhere) |
| `<:`, `:>` | right before, right after |
| `=:` | starts at the same word |
| `:=` | ends at the same word |
| `==` | consists of the same words |
| `[[`, `]]` | contains, lies in |
| `-mother>` | has the other as its mother (an edge) |
| `.lex.` | has the same value of a feature; `.nu#nu.`: different values |

**Quantifiers**, each ending with `/-/`, directly under the object they belong to, at its indentation:

| Written | Means |
|---|---|
| `/without/` … `/-/` | the object doesn't contain … |
| `/where/` A `/have/` B `/-/` | every A in the object is also B |
| `/with/` A `/or/` B `/-/` | the object contains A or B |

## Words

### 1. Every verb

The simplest query: every word whose part of speech is "verb".

```
word sp=verb
```

73,710 results in the BHSA.

### 2. One lexeme: BR>[ “create”

All occurrences of the lexeme BR\>\[ ("to create"). In BHSA lexemes are written in ETCBC transliteration; the "\[" marks a verb, "/" a noun.

```
word lex=BR>[
```

48 results in the BHSA.

### 3. Several features: AND

Two conditions on one line are combined with AND: verbs in the Hifil stem that are also in the wayyiqtol form.

```
word sp=verb vs=hif vt=wayq
```

1,752 results in the BHSA.

### 4. One of several values: OR

The pipe \| means OR within one feature: every word that is either an imperative OR an infinitive construct.

```
word vt=impv|infc
```

10,913 results in the BHSA.

### 5. Not this value

Negation of a value with \#: every word that is NOT Hebrew, i.e. the Aramaic words (Daniel, Ezra, Jer 10:11, Gen 31:47).

```
word language#Hebrew
```

6,106 results in the BHSA.

### 6. Person, gender and number

Person, gender and number together: 2nd person feminine singular verbs.

```
word sp=verb ps=p2 gn=f nu=sg
```

778 results in the BHSA.

### 7. Dual nouns in the construct state

Nouns in the construct state that are dual.

```
word sp=subs st=c nu=du
```

809 results in the BHSA.

### 8. Verbs with a suffix “me”

Verbs that carry a pronominal suffix of the 1st person singular ("me", "my"). prs\_ps / prs\_nu / prs\_gn describe the suffix.

```
word sp=verb prs_ps=p1 prs_nu=sg
```

1,372 results in the BHSA.

### 9. A regular expression on the lexeme

Regular expressions with \~ : all lexemes that start with the letter aleph ("\>" in ETCBC transliteration).

```
word lex~^>
```

62,568 results in the BHSA.

### 10. Lexemes by their gloss

Regular expression on the gloss (English translation of the lexeme). In BHSA the gloss lives on the "lex" node type (one node per lexeme, linked to all its occurrences), so we look for lexemes whose gloss contains "king" and then for the words inside those lexemes.

```
lex gloss~king
  word
```

3,529 results in the BHSA.

### 11. Hapax legomena

Numeric comparison: rare words. freq\_lex is the number of times the lexeme occurs in the whole Bible; here: occurs exactly once (hapax).

```
word freq_lex=1
```

3,071 results in the BHSA.

### 12. Frequent nouns

Numeric comparison with \> : very frequent nouns (more than 1000 times).

```
word sp=subs freq_lex>1000
```

33,889 results in the BHSA.

### 13. The he locale

Proper nouns with the "he locale" ending (uvf = univalent final), the directional -ah as in MYRJMH "to Egypt".

```
word sp=nmpr uvf=H
```

280 results in the BHSA.

### 14. Inside a book

Embedding in a section: all words of the lexeme \>LHJM/ ("God") in the book of Genesis. Indentation means: the word lies inside the book.

```
book book=Genesis
  word lex=>LHJM/
```

219 results in the BHSA.

### 15. Two adjacent words

Two words in sequence. \<: means "immediately followed by": the article directly followed by an adjective.

```
word sp=art
<: word sp=adjv
```

3,282 results in the BHSA.

### 16. Named words in one verse

Naming nodes and using a relation later. Within one verse, the lexeme \>MR\[ ("say") comes somewhere before the lexeme DBR\[ ("speak").

```
verse
  w1:word lex=>MR[
  w2:word lex=DBR[
w1 < w2
```

293 results in the BHSA.

### 17. The same lexeme twice in a row

Comparing features of two nodes with .f. : two adjacent words in the same verse that have the same lexeme (repetitions like "holy, holy").

```
verse
  w1:word
  w2:word
w1 <: w2
w1 .lex. w2
```

1,472 results in the BHSA.

### 18. Three words in a row: “and he said” + preposition

Three adjacent words: the classic sequence "and he said" WJ\>MR (conjunction + wayyiqtol of \>MR\[) followed by a preposition.

```
word lex=W
<: word lex=>MR[ vt=wayq
<: word sp=prep
```

819 results in the BHSA.

## Phrases

### 19. Subject phrases

All phrases that function as the subject of their clause.

```
phrase function=Subj
```

31,979 results in the BHSA.

### 20. Prepositional phrases

Phrase type instead of function: all prepositional phrases.

```
phrase typ=PP
```

57,483 results in the BHSA.

### 21. Objects with the object marker

A phrase with a word inside it: object phrases that contain the object marker \>T ("et").

```
phrase function=Objc
  word lex=>T
```

10,636 results in the BHSA.

### 22. Objects without the object marker

Object phrases WITHOUT the object marker, using the /without/ quantifier.

```
phrase function=Objc
/without/
  word lex=>T
/-/
```

13,640 results in the BHSA.

### 23. Time phrases with “day”

Time phrases (function=Time) that contain the noun JWM/ ("day").

```
phrase function=Time
  word lex=JWM/
```

1,678 results in the BHSA.

### 24. Locatives with a proper noun

Location phrases that contain a proper noun.

```
phrase function=Loca
  word sp=nmpr
```

990 results in the BHSA.

### 25. The first word of a phrase

A phrase whose FIRST word is a preposition. =: means "starts at the same slot", so the word is at the very beginning of the phrase.

```
p:phrase typ=PP
  w:word sp=prep
p =: w
```

55,734 results in the BHSA.

### 26. The last word of a phrase

A phrase whose LAST word is a noun with a pronominal suffix of the third person. := means “ends at the same slot”.

```
p:phrase function=Subj
  w:word sp=subs prs_ps=p3
p := w
```

2,717 results in the BHSA.

### 27. One-word phrases

Single-word phrases: the phrase occupies exactly the same slots as one word (==). Here: subject phrases consisting of only a personal pronoun.

```
p:phrase function=Subj
  w:word sp=prps
p == w
```

3,766 results in the BHSA.

### 28. Nifal predicates

Predicate phrases that contain a verb in the Nifal stem.

```
phrase function=Pred
  word sp=verb vs=nif
```

3,250 results in the BHSA.

### 29. Predicates with an object suffix

Predicate with object suffix (function=PreO), e.g. "he saw him".

```
phrase function=PreO
  word vt=wayq
```

1,028 results in the BHSA.

### 30. Determined subjects

Determined noun phrases (det=det) that are subjects.

```
phrase typ=NP function=Subj det=det
```

11,797 results in the BHSA.

### 31. Negation followed by a predicate

Two phrases in a row: a negation phrase immediately followed by a predicate phrase ("not + verb").

```
phrase function=Nega
<: phrase function=Pred
```

4,832 results in the BHSA.

### 32. Addressing YHWH

Vocative phrases (direct address) that contain the name of God YHWH.

```
phrase function=Voct
  word lex=JHWH/
```

390 results in the BHSA.

### 33. Either of two prepositions

/with/ ... /or/: complement phrases that contain either the preposition \>L ("to") or the preposition L ("to/for").

```
phrase function=Cmpl
/with/
  word lex=>L
/or/
  word lex=L
/-/
```

13,377 results in the BHSA.

### 34. Every noun plural

"Every" via /without/: noun phrases in which EVERY noun is plural. Trick: "every noun is plural" = "there is no noun that is not plural".

```
phrase typ=NP
/without/
  word sp=subs nu#pl
/-/
```

9,436 results in the BHSA.

## Clauses

### 35. Nominal clauses

Nominal clauses: clauses without a finite verb (clause type NmCl).

```
clause typ=NmCl
```

11,377 results in the BHSA.

### 36. Wayyiqtol clauses without an explicit subject

Wayyiqtol clauses without an explicit subject (Way0; WayX has one), in narrative (domain=N).

```
clause typ=Way0 domain=N
```

9,027 results in the BHSA.

### 37. Subject before predicate

Word order within a clause: Subject before Predicate. Phrases indented under the clause lie inside it; \< gives the order.

```
clause
  s:phrase function=Subj
  p:phrase function=Pred
s < p
```

5,028 results in the BHSA.

### 38. Verb – subject – object

The "classic" verb-subject-object order (V-S-O). Each line that starts with \< relates the node on that line to the node on the line above it.

```
clause
  phrase function=Pred
  < phrase function=Subj
  < phrase function=Objc
```

2,211 results in the BHSA.

### 39. The object first

Fronted object: the object is the very first phrase of the clause (=: = starts at the same slot as the clause).

```
c:clause
  o:phrase function=Objc
  phrase function=Pred
c =: o
```

1,141 results in the BHSA.

### 40. Clauses without a predicate

Clauses WITHOUT any predicate phrase (no Pred, PreO, PreS or PreC).

```
clause
/without/
  phrase function=Pred|PreO|PreS|PreC
/-/
```

7,641 results in the BHSA.

### 41. Relative clauses with אשׁר

Relative clauses: clauses that are attributive (rela=Attr) and start with the relative particle \>CR ("asher").

```
c:clause rela=Attr
  w:word lex=>CR
c =: w
```

3,765 results in the BHSA.

### 42. Object clauses with כי

Object clauses (a whole clause functions as object of another clause), e.g. "he saw THAT it was good".

```
clause rela=Objc
  word lex=KJ
```

540 results in the BHSA.

### 43. Imperatives in direct speech

Clauses inside direct speech (domain=Q) that contain an imperative.

```
clause domain=Q
  word vt=impv
```

4,301 results in the BHSA.

### 44. Participle clauses in the Psalms

Participle clauses (typ=Ptcp) in the book of Psalms.

```
book book=Psalmi
  clause typ=Ptcp
```

430 results in the BHSA.

### 45. A clause and its mother

Using an edge feature: a clause and its mother clause. -mother\> follows the "mother" edge from the daughter to the mother. Here: infinitive construct clauses whose mother is a wayyiqtol clause.

```
c1:clause typ=InfC
c2:clause typ=Way0|WayX
c1 -mother> c2
```

1,494 results in the BHSA.

### 46. Saying, then speech

Two clauses in a row: a clause with the lexeme \>MR\[ ("say") immediately followed by a clause in the quotation domain (the start of speech).

```
c1:clause
  word lex=>MR[
c2:clause domain=Q
c1 <: c2
```

5,070 results in the BHSA.

### 47. Three levels at once

Combining all three levels: a clause that contains a subject phrase containing a proper noun, and a predicate phrase with a qal perfect verb.

```
clause
  phrase function=Subj
    word sp=nmpr
  phrase function=Pred
    word vs=qal vt=perf
```

2,533 results in the BHSA.

### 48. Disagreement in number

Agreement with feature comparison: within one clause, a verb in the predicate and a noun in the subject differ in number (.nu\#nu. means “nu of the left node is not equal to nu of the right node”). Useful for collective nouns.

```
clause
  phrase function=Pred
    v:word sp=verb nu=sg|pl
  phrase function=Subj
    s:word sp=subs nu=sg|pl
v .nu#nu. s
```

3,317 results in the BHSA.

### 49. Clause order in a sentence, Genesis 1–11

Sentence level + clause level: sentences that consist of at least two clauses where a wayyiqtol clause is followed (not necessarily directly) by a nominal clause, in Genesis 1-11.

```
book book=Genesis
  chapter chapter<12
    sentence
      clause typ=Way0|WayX
      < clause typ=NmCl
```

9 results in the BHSA.

### 50. Putting it together

A showcase that uses almost everything: in Genesis, a verbal clause (kind=VC) in narrative that starts with the conjunction, directly followed by a wayyiqtol predicate, has a subject containing a proper name, and has NO object phrase. The /without/ block sits directly under the clause line, at the clause’s indentation, so it belongs to clause c; the phrase lines after it are ordinary children of c. In the BHSA the wa- of a wayyiqtol is a phrase of its own (function=Conj), so such a clause never starts with its predicate.

```
book book=Genesis
  c:clause kind=VC domain=N
  /without/
    phrase function=Objc
  /-/
    j:phrase function=Conj
    p:phrase function=Pred
      word vt=wayq
    phrase function=Subj
      word sp=nmpr
c =: j
j <: p
```

454 results in the BHSA.

## Tips

- Lexemes are in the ETCBC's transliteration ([Searching](searching.md#one-line-words-with-features) has the letters). A verb ends in `[`, a noun in `/`, and `=` tells apart lexemes written alike: `>L` is the preposition "to", `>L=` the negation "not". Not sure how a lexeme is written? Look it up in the [lexicon](words.md#the-lexicon), or use a regular expression (`word lex~^MLK`).
- The gloss is a feature of `lex` objects (one per lexeme, with its occurrences inside), not of words; see example 10.
- The `domain` of a clause is `N` (narrative), `Q` (quotation, direct speech) or `D` (discourse).
- A quantifier goes directly under the object it belongs to, at the same indentation; see example 50.
- If a template finds nothing, [When a template is wrong](searching.md#when-a-template-is-wrong) lists the usual causes.
