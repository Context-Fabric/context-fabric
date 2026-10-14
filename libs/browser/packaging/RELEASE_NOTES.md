**A test version.** Context-Fabric as a program of its own, for reading, searching and comparing the Hebrew Bible (BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls side by side. You don't need to install Python. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Before you start: the three corpora

This test is about three corpora only:

| Corpus | Version | Folder in your home folder |
|---|---|---|
| BHSA (Hebrew Bible) | 2021 | `text-fabric-data/github/ETCBC/bhsa/tf/2021` |
| Samaritan Pentateuch | 7.2.4 | `text-fabric-data/github/DT-UCPH/sp/tf/7.2.4` |
| Dead Sea Scrolls | 2.0.1 | `text-fabric-data/github/ETCBC/dss/tf/2.0.1` |

The program reads them where [Text-Fabric](https://annotation.github.io/text-fabric/) has downloaded them. If you have used Text-Fabric with these corpora, they are probably there already. Other corpora on your computer are not offered in this test. The program can't download corpora itself yet.

## Download

| Computer | File |
|---|---|
| Windows 10 or 11 | `cfabric-browser-windows.zip` |
| Mac with Apple silicon (M1 and later) | `Context-Fabric-macos-arm64.zip` |
| Mac with Intel processor | `Context-Fabric-macos-x64.zip` |
| Linux | `cfabric-browser-linux.tar.gz` |

## Start

The program isn't signed yet, so your computer warns about it the first time.

**Windows:** unpack the zip, open the folder `cfabric-browser` and double-click `cfabric-browser.exe`. If Windows says "Windows protected your PC", click *More info* and then *Run anyway*. Keep the folder together: the program needs the `_internal` folder next to it.

**macOS:** unpack the zip and move `Context-Fabric.app` to *Applications*. The first time, right-click it, choose *Open*, and then *Open* again. If macOS only offers to move it to the bin, go to *System Settings → Privacy & Security* and click *Open Anyway*.

**Linux:** unpack (`tar -xzf cfabric-browser-linux.tar.gz`) and start `cfabric-browser/cfabric-browser`. The window needs GTK with WebKit2GTK, or Qt with WebEngine. Without them, the program opens your web browser instead.

## Use

- The program opens with *Open a corpus*: the three corpora, if they are on your computer. Choose one to start.
- *Add a pane* shows another corpus, or another passage, side by side. *Sync passages* keeps the panes at the same passage, and *Show differences* marks where the texts differ. With the BHSA, the SP and the DSS each in a pane of its own (the DSS read by biblical book), *Show differences* also shows which scrolls side with the BHSA and which with the SP.
- The first time you open a large corpus (such as the BHSA), it takes a little longer, because it is prepared once. After that it opens in about a second.
- **Lexicon** (BHSA): next to *Read* and *Search*. All lexemes of the Hebrew Bible, by language (Hebrew, Aramaic) and first letter, with their gloss, part of speech and frequency. *Find* looks in all letters at once: type Hebrew (ברא), a transliteration (BR>[) or a gloss (create). Choose a lexeme to see every verse it occurs in, with the word marked, and how often it occurs per book; choose a book to see only its verses. Click a verse to read it; with *Sync passages* on, the other panes go to the same verse. Its occurrences can be downloaded as a spreadsheet.
- Settings and saved workspaces are kept between sessions.
- If the program doesn't start, look in `.cfabric/logs/cfabric-browser.log` in your home folder and send that file to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Known limitations

- On macOS and Linux, the window hasn't been tried by anyone yet. You're the first.
- Corpora can't be downloaded from the program yet.
