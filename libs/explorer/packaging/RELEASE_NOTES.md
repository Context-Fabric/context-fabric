**A test version.** CACCHT Explorer, a program of its own for reading, searching and comparing the Hebrew Bible (BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls side by side. You don't need to install Python. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

**How to use it** is in the [guide to CACCHT Explorer](https://caccht.github.io/explorer/), also inside the program: *Guide to …* at the bottom of the menu at the top right. Start with [Getting started](https://caccht.github.io/explorer/getting-started.html).

## What is new

- **Since test.14: asking ChatGPT.** Besides Claude, you can now ask ChatGPT for a search, in the ChatGPT desktop app or in Codex, with your own ChatGPT subscription: see [In ChatGPT or Codex](https://caccht.github.io/explorer/asking-claude.html#in-chatgpt-or-codex). Not yet tried with ChatGPT itself, so please tell us how it goes. A search shows which app proposed it, also when the model doesn't say.
- Since test.14: the guide's example of "only proper names" in [Searching](https://caccht.github.io/explorer/searching.html#without-and-for-all) was wrong (it found subjects with at least one proper name); corrected, and the same in what Claude and ChatGPT are told about searching.
- Since test.14: asking from Claude Desktop and ChatGPT no longer fails when the Explorer still has to start; a trial search that takes too long says how to make it faster.
- Since test.13: the guide is written, with screenshots: getting started, the datasets, the five ways of working, reading and comparing, words, searching (with a short course in search templates), the Concordance, asking Claude, adding texts, notes, workspaces and citing, and questions and problems. The instructions for asking Claude moved from these notes to the guide.
- Since test.12: a new name. The program is now called CACCHT Explorer; Context-Fabric remains the name of the engine underneath. On a Mac, the app is `CACCHT Explorer.app`: you may delete the old `Context-Fabric.app`. In Claude Desktop, install `CACCHT-Explorer.mcpb`; it takes the place of the old extension.
- Since test.11: in the Samaritan Pentateuch, hovering over a word shows the word, not a single letter.
- Since test.10: *Search* opens on *Template* instead of *Search terms*.
- Since test.9: faster searching, on the new core of Context-Fabric, written in Rust. The first time you open a corpus after updating from test.9 or earlier, the program prepares it once more; the `.cfm` folders next to the corpora are no longer used, and you may delete them.

Earlier: asking Claude for a search in your own words (test.9), the BDB dictionary and the choice between vocalized and unvocalized text (test.7), downloading the corpora in the program (test.6), the JPS 1917 translation (test.5), and the five ways of working (test.3).

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

- On macOS and Linux, the window hasn't been tried by many people yet.
- Only the BHSA, the Samaritan Pentateuch and the Dead Sea Scrolls are offered; other corpora on your computer are not.
- Asking Claude has been tried with test clients and by few people with Claude itself; asking ChatGPT not yet with ChatGPT itself.
