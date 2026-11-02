# The datasets

The Explorer reads annotated corpora: texts in which every word carries its lexeme, part of speech, morphology and more, and in which words are grouped into phrases, clauses and verses. It offers three corpora, which it downloads at the [first start](getting-started.md#the-first-start), and a translation and a dictionary that you can add.

| Dataset | Version | In the Explorer | Licence |
|---|---|---|---|
| [BHSA](#the-bhsa) (Hebrew Bible) | 2021 | downloaded at the first start | CC BY-NC 4.0 |
| [Samaritan Pentateuch](#the-samaritan-pentateuch) | 7.2.4 | downloaded at the first start | CC BY-NC 4.0 |
| [Dead Sea Scrolls](#the-dead-sea-scrolls) | 2.0.1 | downloaded at the first start | CC BY-NC 4.0 |
| [JPS 1917](#the-jps-1917-translation) (English) | 1917 | can be added | public domain in the US |
| [BDB](#the-bdb-dictionary) (dictionary) | 1.0 | can be added | CC BY-NC 4.0 |

The Samaritan Pentateuch and the Dead Sea Scrolls are CACCHT datasets: their cards on [caccht.github.io](https://caccht.github.io/#corpora) give the repository, the DOI of each release and the licence, and the [data-use statement](https://caccht.github.io/data-use.html) says how they may be used. All three corpora are free for research and teaching; CC BY-NC 4.0 means you may share and adapt them, not commercially, if you name the source.

*Cite* in the top bar gives a citation of what you are looking at, with the corpus, its version and its DOI. Cite the version you actually used.

## The BHSA

The *Biblia Hebraica Stuttgartensia Amstelodamensis*, the Masoretic Text of the Hebrew Bible as annotated by the [ETCBC](https://etcbc.nl) in Amsterdam ([its documentation](https://etcbc.github.io/bhsa/)). The CACCHT datasets follow its conventions, so its features have the same names and meanings in the other corpora.

In the Explorer the BHSA is the reference text:

- Books, chapters and verses are numbered as in the BHSA, and the other texts are lined up with it.
- *Display → Text* chooses between *Vocalized* (with vowels and accents; where the Masoretes read a word otherwise than it is written, the qere) and *Unvocalized* (the consonants as written, the ketiv).
- The *Lexicon* lists all its lexemes, with gloss, part of speech, frequency and every verse they occur in; see [Words](words.md).
- Words in the Dead Sea Scrolls get their gloss from the BHSA's lexemes.
- The Interlinear opens on the BHSA, with transliteration, gloss and morphology under each word.

## The Samaritan Pentateuch

The Pentateuch as the Samaritans hand it down, annotated word by word in the style of the BHSA, with the boundaries of phrases and clauses. Its card: [The CACCHT Samaritan Pentateuch](https://caccht.github.io/#corpora) (in progress).

- The text is shown in Hebrew square script, without vowels: the Samaritan Pentateuch has one text, so *Display* offers no choice of text.
- Hovering over a word shows its lexeme, gloss and morphology, as for the BHSA.
- It is lined up with the BHSA verse by verse; *Differences* marks where its words differ.

## The Dead Sea Scrolls

The biblical and non-biblical scrolls from the Judaean Desert, transcribed and annotated word by word, after Martin Abegg's transcriptions. Its card: [The CACCHT Dead Sea Scrolls](https://caccht.github.io/#corpora) (released).

The scrolls are manuscripts, not a book, so the Explorer shows them in two ways:

- **By biblical book.** In Parallel, the Reader and the Interlinear, a verse shows the text of every scroll that has it, one line per scroll, named by the scroll.
- **By scroll.** In Panes, choose *Scrolls* (instead of *Bible books*) to read a scroll as it is, line by line in its columns or fragments, including the non-biblical scrolls.

What the signs mean:

- **Dimmed letters** are reconstructed by the editors: they are not on the scroll. More than half of the letters in the biblical scrolls are reconstructed. They never count when texts are compared.
- **`[…]`** marks text that is lost.
- **`◦`** marks a letter that can't be read.
- **"also non-biblical"** is said of the few lines that the edition lists both as biblical and as non-biblical text.

*Differences* compares the scrolls with the first column. With the BHSA, the Samaritan Pentateuch and the scrolls shown together, it also says which scrolls side with the BHSA and which with the Samaritan Pentateuch.

## The JPS 1917 translation

*The Holy Scriptures: A New Translation* of the Jewish Publication Society of America (1917), in English. Add it in *Texts → Add a text…*, under *Translations*: the program downloads it from [Sefaria](https://www.sefaria.org/), about 5 MB, and prepares it in a few seconds. See [Adding texts](adding-texts.md).

- Its verses are numbered as in the Hebrew Bible, so it lines up with the BHSA.
- In three places the 1917 print joins verses that the BHSA divides (the Ten Commandments in Exodus 20 and Deuteronomy 5, and Numbers 25:19 with 26:1). The Explorer divides them as the BHSA does and shows the print's number next to those verses.
- It is left out of *Differences* and of the hover card: it is a translation, not a witness.
- It is in the public domain in the United States; elsewhere that depends on the translators' dates.

## The BDB dictionary

*A Hebrew and English Lexicon of the Old Testament* by Brown, Driver and Briggs (1906), with every lexeme of the BHSA linked to its entry. Add it in *Texts → Add a text…*, under *Dictionaries* (about 5 MB). Its entries then appear in the *Lexicon*, in the hover card and in the Interlinear; see [Words](words.md).

The text is from Sefaria, the ids and short definitions from [Open Scriptures](https://github.com/openscriptures/HebrewLexicon), and the links to the BHSA's lexemes were made for the Explorer. The file is published as [Context-Fabric/bdb](https://github.com/Context-Fabric/bdb), under CC BY-NC 4.0.

## Not yet in the Explorer

CACCHT publishes more datasets than the Explorer offers so far. They can be used with Text-Fabric; see their cards on [caccht.github.io](https://caccht.github.io/#corpora):

- the Aleppo Codex;
- the ETCBC Syriac Corpus;
- the ETCBC Targum Corpus;
- the Copenhagen Ugaritic Corpus;
- the Septuagint (in development at CenterBLC).
