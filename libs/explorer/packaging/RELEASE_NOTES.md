**CACCHT Explorer**, a program of its own for reading, searching and comparing the Hebrew Bible (BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls side by side. You don't need to install Python. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

**How to use it** is in the [guide to CACCHT Explorer](https://caccht.github.io/explorer/), also inside the program: *Guide to …* at the bottom of the menu at the top right. Start with [Getting started](https://caccht.github.io/explorer/getting-started.html).

## New in 0.1.2

- **The Statenvertaling** (1637, in the text of the 1888 edition) can be added as a text next to the Hebrew: *Texts → Add a text…*, under *Translations* (about 30 MB). Its verses are lined up with the Hebrew, with its own number shown where it counts otherwise (Malachi 3:19 is 4:1); the words the translators added are in italics, and their notes, the *kanttekeningen*, are under each verse. See [The Statenvertaling](https://caccht.github.io/explorer/datasets.html#the-statenvertaling).
- After adding a translation or dictionary, the library sometimes kept showing it as not added until the page was reloaded; fixed.

## What it does

- **Five ways of working:** Parallel (verses of several texts next to each other), Panes, Reader, Interlinear and Concordance, with one menu to switch between them at the same passage.
- **Comparing:** differences between the Masoretic text, the Samaritan Pentateuch and the scrolls marked word by word, and the scrolls read by biblical book.
- **Words:** hover over a word for its gloss, lexeme and morphology; a lexicon of the BHSA with the BDB dictionary (added from the library).
- **Searching** with search templates, with a short course and fifty examples in the guide, and results to inspect, cite and download.
- **Asking in your own words:** Claude or ChatGPT proposes a search with your own subscription, and the Explorer runs it.
- **Translations** as optional texts: the JPS 1917 (English) and the Statenvertaling (Dutch); and notes and workspaces that stay on your computer.

## Updating

Put the new version where the old one was; the corpora, your settings, notes and workspaces stay. Installed with pip: `pip install -U cfabric-browser`. If you come from a test version of 0.1.0, test.12 or earlier: on a Mac you may delete the old `Context-Fabric.app`, and in Claude Desktop install `CACCHT-Explorer.mcpb` in place of the old extension. From test.9 or earlier, the program prepares each corpus once more, and you may delete the `.cfm` folders next to the corpora.

## Download

| Computer | File |
|---|---|
| Windows 10 or 11 | `CACCHT-Explorer-windows.zip` |
| Mac with Apple silicon (M1 and later) | `CACCHT-Explorer-macos-arm64.zip` |
| Mac with Intel processor | `CACCHT-Explorer-macos-x64.zip` |
| Linux | `CACCHT-Explorer-linux.tar.gz` |

`CACCHT-Explorer.mcpb` is for [asking Claude](https://caccht.github.io/explorer/asking-claude.html) in Claude Desktop.

## Start

The program isn't signed yet, so your computer warns about it the first time.

- **Windows:** unpack the zip, open the folder `cfabric-browser` and double-click `cfabric-browser.exe`. If Windows says "Windows protected your PC", click *More info* and then *Run anyway*.
- **macOS:** unpack the zip and move `CACCHT Explorer.app` to *Applications*. The first time, right-click it, choose *Open*, and then *Open* again. If macOS only offers to move it to the bin, go to *System Settings → Privacy & Security* and click *Open Anyway*.
- **Linux:** unpack it (`tar -xzf CACCHT-Explorer-linux.tar.gz`) and start `cfabric-browser/cfabric-browser`.

The first time, the program offers to download the three corpora (68 MB, all under CC BY-NC 4.0) and prepares them, which takes a few minutes. If you have used Text-Fabric with them, it finds them and downloads nothing.

If the program doesn't start, see [Questions and problems](https://caccht.github.io/explorer/problems.html), and send the log (`.cfabric/logs/cfabric-browser.log` in your home folder) to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Known limitations

- Only the BHSA, the Samaritan Pentateuch and the Dead Sea Scrolls are offered; other corpora on your computer are not.
- On macOS and Linux, the window has been tried by fewer people than on Windows.
- Asking ChatGPT has been tried by few people with ChatGPT itself.
