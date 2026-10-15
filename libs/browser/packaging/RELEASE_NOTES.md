**A test version.** Context-Fabric as a program of its own, for reading, searching and comparing the Hebrew Bible (BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls side by side. You don't need to install Python. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Before you start: the three corpora

This test is about three corpora only:

| Corpus | Version | Folder in your home folder |
|---|---|---|
| BHSA (Hebrew Bible) | 2021 | `text-fabric-data/github/ETCBC/bhsa/tf/2021` |
| Samaritan Pentateuch | 7.2.4 | `text-fabric-data/github/DT-UCPH/sp/tf/7.2.4` |
| Dead Sea Scrolls | 2.0.1 | `text-fabric-data/github/ETCBC/dss/tf/2.0.1` |

The program reads them where [Text-Fabric](https://annotation.github.io/text-fabric/) has downloaded them. If you have used Text-Fabric with these corpora, they are probably there already. Other corpora on your computer are not offered in this test. The program can't download these corpora itself yet.

**New in this version: an English translation.** The program can add the JPS 1917 (*The Holy Scriptures: A New Translation*, Jewish Publication Society, 1917) itself; see *Use*.

## Download

| Computer | File |
|---|---|
| Windows 10 or 11 | `cfabric-browser-windows.zip` |
| Mac with Apple silicon (M1 and later) | `Context-Fabric-macos-arm64.zip` |
| Mac with Intel processor | `Context-Fabric-macos-x64.zip` |
| Linux | `cfabric-browser-linux.tar.gz` |

## Start

The program isn't signed yet, so your computer warns about it the first time.

**Windows:** unpack the zip, open the folder `cfabric-browser` and double-click `cfabric-browser.exe`. If Windows says "Windows protected your PC", click *More info* and then *Run anyway*. Keep the folder together: the program needs the `_internal` folder next to it. If the window doesn't open, right-click the downloaded zip before unpacking it, choose *Properties*, tick *Unblock*, and unpack it again.

**macOS:** unpack the zip and move `Context-Fabric.app` to *Applications*. The first time, right-click it, choose *Open*, and then *Open* again. If macOS only offers to move it to the bin, go to *System Settings → Privacy & Security* and click *Open Anyway*.

**Linux:** unpack (`tar -xzf cfabric-browser-linux.tar.gz`) and start `cfabric-browser/cfabric-browser`. The window needs GTK with WebKit2GTK, or Qt with WebEngine. Without them, the program opens your web browser instead.

## Use

The program opens in **Parallel**, laid out after [Parabible](https://parabible.com): a column per corpus and a row per verse, so the BHSA, the SP and the scrolls line up verse by verse. The menu at the top right offers four other ways of working, and takes the passage, the texts and the search along when you switch:

- **Panes:** corpora side by side, each with its own passage and view.
- **Reader** (after [Sefaria](https://www.sefaria.org)): one text in a quiet column, verse by verse or as running text, chapter after chapter as you scroll. Click a verse for a panel with its parallels, words, lexicon entries, a search and your own notes.
- **Interlinear** (after STEPBible): each word with lines under it (transliteration, lexeme, gloss, morphology, or any feature), another text word by word under it, phrases and clauses, colours, and a practice mode that hides a line.
- **Concordance** (after Sketch Engine): search results as lines with the hit in the middle, sorted by the words before or after it, filtered by book, stem or tense, with the words most typical near the hit (*Collocations*) and a download of the lines.

*Choose which to offer…* in that menu hides the ones you don't use. The program remembers the one you used last.

- **The JPS 1917 translation:** in *Texts → Add a text…*, under *Translations*, click *Add*. The program downloads it from Sefaria (about 5 MB, once), prepares it in a few seconds and shows it as a column next to the Hebrew. Its verses are numbered as in the Hebrew Bible, so they line up with the BHSA. In the Interlinear, choose it under *Under* for a line of English under each verse; in the Concordance, search it in English, for instance `word lower=lord`. It is in the public domain in the United States; elsewhere that depends on the translators' dates. *Remove* deletes it again.

- **The first time:** the BHSA opens by itself, and *Open a corpus* offers the three corpora. Choose the SP there, and add the DSS with *Texts → Add a text…*. The program remembers your choice. Opening a large corpus (such as the BHSA) the first time takes a little longer, because it is prepared once; after that it opens in about a second.
- **Reading:** click the chapter name at the top for the books and chapters; `‹` and `›` (or the arrow keys) go to the previous and next chapter. *Texts* turns corpora on and off and changes their order. The Dead Sea Scrolls show, per verse, the text of every scroll that has it; dimmed letters are reconstructed by the editors.
- **Differences** marks the words where the texts differ from the first column. With the BHSA, the SP and the DSS shown, the DSS column also says which scrolls side with the BHSA and which with the SP.
- **Words:** hover over a word to see its gloss, lexeme, part of speech and morphology. Click it for all its features, in a panel on the left. Words in the Dead Sea Scrolls get their gloss from the BHSA.
- **Search:** click a word, then *Add as search term*, and tick the features that should count. Add more terms, choose whether they must be in one verse, clause or phrase, and choose books if you like. Or write a search template under *Template*. The results show each verse in all the texts, with statistics and a download as a spreadsheet.
- **Lexicon** (BHSA): all lexemes of the Hebrew Bible, by language and first letter, with gloss, part of speech and frequency. Choose a lexeme to see every verse it occurs in; click a verse to read it in all the texts.
- **Panes:** each pane has its own corpus, passage and view (*Read*, *Search*, *Lexicon*). *Sync passages* keeps them at the same passage.
- **Notes** in the Reader stay on your computer. They go along when you export a workspace.
- *Display*, *Workspaces* (save and reopen what you are looking at) and *Cite* are in the top bar. Settings and saved workspaces are kept between sessions.
- If the program doesn't start, look in `.cfabric/logs/cfabric-browser.log` in your home folder and send that file to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Known limitations

- Fixed since test.2: switching from *Panes* to *Parallel* could give an empty page.
- Fixed since test.3: on Windows, the program could fail to start when its zip was unpacked with Explorer.
- New since test.4: the JPS 1917 translation.

- On macOS and Linux, the window hasn't been tried by anyone yet. You're the first.
- The BHSA, the SP and the DSS can't be downloaded from the program yet; only the translation can.
- In three places the 1917 print joins verses that the BHSA divides (the Ten Commandments in Exodus 20 and Deuteronomy 5, and Numbers 25:19 with 26:1). The program divides them as the BHSA does and shows the print's number next to those verses.
