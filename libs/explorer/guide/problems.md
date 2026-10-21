# Questions and problems

## The program doesn't start

- **Windows says *Windows protected your PC*, or macOS won't open it.** The program isn't signed yet; see [Starting the program](getting-started.md#starting-the-program) for how to open it anyway.
- **On Windows, nothing happens, or the window stays empty.** Right-click the downloaded zip, choose *Properties*, tick *Unblock*, and unpack it again. Keep the folder `cfabric-browser` together: the program needs its `_internal` folder.
- **A message says *CACCHT Explorer could not start*.** The log says why; see [The log](#the-log).
- **On Linux, a web browser opens instead of a window.** The window needs GTK with WebKit2GTK, or Qt with WebEngine. In the web browser, the program works the same.

## It is slow

- **The first time a corpus opens**, the program prepares it once, which takes from half a minute to a few minutes, the BHSA longest. After that it opens in a few seconds. A new version of the program sometimes prepares the corpora once more.
- **A search that takes long** can be stopped with *Stop*. A search stops by itself after 60 seconds. Searches with many objects at the same level, or without anything to narrow them down, take longest: put the objects inside one another where you can, and add features. See [Searching](searching.md#limits).

## Something doesn't work

- **A corpus doesn't open** (*Could not open it: …*). Close the program and start it again. If it still fails, delete the corpus' `.cfr` folder (in `text-fabric-data/github/<organisation>/<corpus>/tf/<version>` in your home folder): the program then prepares it again. If that doesn't help, download it again from the library.
- **A download fails** (*Could not download it: …*). Check the internet connection and click *Try again*; what was downloaded already is kept.
- **The page says *The browser server cannot be reached*.** The program has stopped, or was closed while the page was still open in a web browser. Start the program again and reload the page.
- **Hovering over a word shows a single letter**, or the text looks otherwise wrong. If you have your own display settings in `.cfabric/display` in your home folder, move them away and start the program again: they take precedence over the program's.
- **Claude can't reach the program.** The program must be running first. See [Asking Claude](asking-claude.md#setting-it-up).

## The log

The program writes what it does, and any error, to a log: `.cfabric/logs/cfabric-browser.log` in your home folder. It starts afresh each time the program starts. The log also gives the address the program uses, such as `http://localhost:8107/`: 8107 is the usual port, and if another program has taken it, the Explorer takes the next free one.

## Reporting a problem

Please report what goes wrong, and what you would like to see, in an [issue on GitHub](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch). Say which computer you use (Windows, Mac with Apple silicon or Intel, Linux), which version of the Explorer (in *Cite*), what you did and what happened, and send the log along.
