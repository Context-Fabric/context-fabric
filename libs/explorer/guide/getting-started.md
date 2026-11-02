# Getting started

CACCHT Explorer is a program for reading, searching and comparing the Hebrew Bible (the BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls, side by side. It runs on your own computer, needs no internet connection once the texts are there, and you don't need to install Python or anything else for it. It reads the same data as [Text-Fabric](https://annotation.github.io/text-fabric/), through Context-Fabric, the engine underneath.

The Explorer is a test version. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Download

The program is on the [releases page](https://github.com/Context-Fabric/context-fabric/releases) of Context-Fabric on GitHub. Take the newest release whose name begins with a version number (such as *0.1.0-test.13*), and under *Assets* the file for your computer:

| Computer | File |
|---|---|
| Windows 10 or 11 | `CACCHT-Explorer-windows.zip` |
| Mac with Apple silicon (M1 and later) | `CACCHT-Explorer-macos-arm64.zip` |
| Mac with an Intel processor | `CACCHT-Explorer-macos-x64.zip` |
| Linux | `CACCHT-Explorer-linux.tar.gz` |

On a Mac, *About This Mac* in the Apple menu says which processor you have: *Chip Apple M…* is Apple silicon.

The release also has `CACCHT-Explorer.mcpb`, which you need only for [asking Claude](asking-claude.md) in Claude Desktop.

## Starting the program

The program isn't signed yet, so your computer warns about it the first time.

**Windows.** Unpack the zip, open the folder `cfabric-browser` and double-click `cfabric-browser.exe`. If Windows says *Windows protected your PC*, click *More info* and then *Run anyway*. Keep the folder together: the program needs the `_internal` folder next to it. If the window doesn't open, right-click the downloaded zip before unpacking it, choose *Properties*, tick *Unblock*, and unpack it again.

**macOS.** Unpack the zip and move `CACCHT Explorer.app` to *Applications*. The first time, right-click it, choose *Open*, and then *Open* again. If macOS only offers to move it to the bin, go to *System Settings → Privacy & Security* and click *Open Anyway*.

**Linux.** Unpack it (`tar -xzf CACCHT-Explorer-linux.tar.gz`) and start `cfabric-browser/cfabric-browser`. The window needs GTK with WebKit2GTK, or Qt with WebEngine. Without them, the program opens your web browser instead, which works just as well.

## The first start

If the texts are not on your computer yet, the program shows *Getting the corpora*, with the three it offers:

| Corpus | Version | Download |
|---|---|---|
| BHSA (Hebrew Bible) | 2021 | 34 MB |
| Samaritan Pentateuch | 7.2.4 | 4 MB |
| Dead Sea Scrolls | 2.0.1 | 30 MB |

Click *Download (68 MB)*. Each corpus is downloaded from its own release on GitHub, unpacked and then prepared once, so that it opens quickly afterwards. You see the progress of each. The download takes less than a minute on a fast connection; preparing takes a few minutes in all, the BHSA longest. Then the program opens in [Parallel](interfaces.md#parallel), with the three texts next to each other.

*Not now* skips this. You can download the corpora later in *Texts → Add a text…*, under *Can be downloaded*. Without an internet connection, the program says so and offers *Try again*.

If you have used Text-Fabric with these corpora, the program finds them and downloads nothing; it only prepares them the first time it opens them.

[The datasets](datasets.md) says what each corpus is and what the program shows of it.

## Where the data is kept

Everything the program keeps is in your home folder (on Windows `C:\Users\<you>`, on a Mac `/Users/<you>`):

| Folder | What is in it |
|---|---|
| `text-fabric-data/github` | The corpora, where Text-Fabric keeps them too, and the translations you add. Next to each corpus a folder `.cfr` holds what the program prepared (for the BHSA about 600 MB). |
| `.cfabric/dictionaries` | The dictionaries you add (BDB). |
| `.cfabric/window` | The program's settings, your notes and your saved workspaces. |
| `.cfabric/logs` | The log, `cfabric-browser.log`, to send along when something goes wrong. |
| `.cfabric/downloads`, `.cfabric/translations` | The downloaded files, kept so that a download need not be repeated. They may be deleted. |

Folders whose name begins with a dot are hidden. On a Mac, press *Cmd+Shift+.* in the Finder to see them; in the Windows Explorer, choose *View → Show → Hidden items*.

## Updating and removing

**Updating.** Download the new version and put it where the old one was: on Windows, replace the folder `cfabric-browser`; on a Mac, replace `CACCHT Explorer.app` in *Applications*. The corpora, your settings, notes and workspaces stay where they are, so the new version finds them. Sometimes a new version has to prepare the corpora once more; the release notes say so.

**Removing.** Delete the folder `cfabric-browser` (Windows, Linux) or `CACCHT Explorer.app` (Mac). That leaves the data. To remove that too, delete the folder `.cfabric` in your home folder, and in `text-fabric-data/github` the folders of the corpora (`ETCBC/bhsa`, `ETCBC/dss`, `DT-UCPH/sp` and `Context-Fabric`). If you also use Text-Fabric, it uses the same corpora, so keep them.
