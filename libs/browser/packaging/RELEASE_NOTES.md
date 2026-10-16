**A test version.** Context-Fabric as a program of its own, for reading, searching and comparing the Hebrew Bible (BHSA), the Samaritan Pentateuch and the Dead Sea Scrolls side by side. You don't need to install Python. Please report what goes wrong, and what you would like to see, in an [issue](https://github.com/Context-Fabric/context-fabric/issues) or by email to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## The corpora

This test is about three corpora, all under the licence CC BY-NC 4.0:

| Corpus | Version | Download |
|---|---|---|
| BHSA (Hebrew Bible) | 2021 | 34 MB |
| Samaritan Pentateuch | 7.2.4 | 4 MB |
| Dead Sea Scrolls | 2.0.1 | 30 MB |

**New in this version: you don't need them on your computer first.** When they are missing, the program offers to download them the first time it starts, from their own releases on GitHub (the same files [Text-Fabric](https://annotation.github.io/text-fabric/) uses). It puts them where Text-Fabric keeps its downloads (`text-fabric-data/github` in your home folder), so if you have used Text-Fabric with these corpora, the program finds them there and downloads nothing. Other corpora on your computer are not offered in this test.

**New since test.9: faster searching.** The program now runs on the new core of Context-Fabric, written in Rust: searches take a fraction of the time they took. The first time you open a corpus after updating, the program prepares it once more (about a minute for the BHSA); the files the earlier versions prepared (`.cfm` folders next to the corpora) are no longer used, and you may delete them.

**New since test.7: asking in your own words.** With your own AI app (Claude Desktop or Claude Code, with your own subscription) you can ask for a search in plain language, such as *all cases of >LHJM/ in the BHSA*; the search appears in the program for you to run. See *Asking Claude for a search*.

**New since test.6: the BDB dictionary.** The lexicon can show *A Hebrew and English Lexicon of the Old Testament* by Brown, Driver and Briggs (1906), with every lexeme of the BHSA linked to its entry; see *Use*. And the BHSA's text can be shown with or without vowels and accents (*Display → Text*).

**Also new since test.4: an English translation.** The program can add the JPS 1917 (*The Holy Scriptures: A New Translation*, Jewish Publication Society, 1917) itself; see *Use*.

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

- **BDB:** in *Texts → Add a text…*, under *Dictionaries*, click *Add* (about 5 MB, once). A lexeme's page in the *Lexicon* then has a tab *BDB* with its entry: its verses open the passage, its references open other entries, BDB's abbreviations are explained when you hover over them, long entries have their stems folded, and the root family links to the other lexemes of the root. The lexicon's *Find* also searches BDB's definitions (*perish* finds אבד), and it can be ordered *By root (BDB)*. Hovering over a word shows its BDB entry, also for words in the Dead Sea Scrolls (through the BHSA); the Interlinear offers a line *BDB*. The text is from Sefaria, the ids and short definitions from Open Scriptures; the file is CC BY-NC 4.0 ([Context-Fabric/bdb](https://github.com/Context-Fabric/bdb)).
- **Asking in your own words:** see the section *Asking Claude for a search* below.
- **Vocalized or unvocalized:** *Display → Text* chooses for the BHSA between *Vocalized* (vowels and accents, and the qere where it differs) and *Unvocalized* (the consonants of the ketiv). The other corpora have one text, so the choice isn't shown for them.
- **The first time:** if the corpora are not on your computer, the program shows *Getting the corpora*: click *Download (68 MB)*. Each corpus is downloaded, unpacked and prepared once, which takes a few minutes in all (the download itself less than a minute on a fast connection); you see the progress of each. Then Parallel opens with the three. *Not now* skips this; the corpora can then be downloaded later in *Texts → Add a text…*, under *Can be downloaded*. Without an internet connection the program says so and offers *Try again*.
- If the corpora are there already, the BHSA opens by itself, and *Open a corpus* offers the others. Opening a corpus the first time takes a little longer, because it is prepared once; after that it opens in a few seconds.
- **Reading:** click the chapter name at the top for the books and chapters; `‹` and `›` (or the arrow keys) go to the previous and next chapter. *Texts* turns corpora on and off and changes their order. The Dead Sea Scrolls show, per verse, the text of every scroll that has it; dimmed letters are reconstructed by the editors.
- **Differences** marks the words where the texts differ from the first column. With the BHSA, the SP and the DSS shown, the DSS column also says which scrolls side with the BHSA and which with the SP.
- **Words:** hover over a word to see its gloss, lexeme, part of speech and morphology. Click it for all its features, in a panel on the left. Words in the Dead Sea Scrolls get their gloss from the BHSA.
- **Search:** click a word, then *Add as search term*, and tick the features that should count. Add more terms, choose whether they must be in one verse, clause or phrase, and choose books if you like. Or write a search template under *Template*. The results show each verse in all the texts, with statistics and a download as a spreadsheet.
- **Lexicon** (BHSA): all lexemes of the Hebrew Bible, by language and first letter, with gloss, part of speech and frequency. Choose a lexeme to see every verse it occurs in; click a verse to read it in all the texts.
- **Panes:** each pane has its own corpus, passage and view (*Read*, *Search*, *Lexicon*). *Sync passages* keeps them at the same passage.
- **Notes** in the Reader stay on your computer. They go along when you export a workspace.
- *Display*, *Workspaces* (save and reopen what you are looking at) and *Cite* are in the top bar. Settings and saved workspaces are kept between sessions.
- If the program doesn't start, look in `.cfabric/logs/cfabric-browser.log` in your home folder and send that file to [martijn.naaijer@uzh.ch](mailto:martijn.naaijer@uzh.ch).

## Asking Claude for a search

You can ask Claude, in your own words, for a search: for instance *find all cases of >LHJM/ in the BHSA*, or *clauses where the object comes before the verb*. Claude writes a search template, tries it, and sends it to the program. You decide whether it runs. This uses your own Claude subscription, in Claude Desktop or Claude Code.

**Always first:** start Context-Fabric and keep it running. Claude talks to the running program.

### In Claude Desktop

1. Download `Context-Fabric.mcpb` from this release (in the list of files below).
2. Install it, in one of these ways:
   - double-click the downloaded file: Claude Desktop opens it and offers to install it; click *Install*;
   - or, in Claude Desktop, go to *Settings → Extensions*, and choose *Install Extension…* (under *Advanced settings*), then the file;
   - or drag the file into the *Settings → Extensions* window.
3. Check that *Context-Fabric browser* is switched on under *Settings → Extensions*.
4. Start a new chat. The tools menu (the button below the message box) now lists *Context-Fabric browser*.

You need a recent Claude Desktop. The extension needs nothing else: no Python and no Node.js of your own. If your Claude Desktop names its menus differently, look under *Extensions* for the button that installs one.

**Without the extension** (if installing it doesn't work): in Claude Desktop, *Settings → Developer → Edit Config* opens `claude_desktop_config.json`. Add the program, with its path on your computer, and restart Claude Desktop:

```json
{
  "mcpServers": {
    "context-fabric": {
      "command": "C:\\path\\to\\cfabric-browser\\cfabric-browser.exe",
      "args": ["mcp"]
    }
  }
}
```

On a Mac the command is `/Applications/Context-Fabric.app/Contents/MacOS/cfabric-browser`; on Linux, the path of `cfabric-browser/cfabric-browser`.

### In Claude Code

In a terminal, once:

```
claude mcp add --transport http context-fabric http://127.0.0.1:8107/mcp
```

(8107 is the program's usual port. If another program had taken it, the address in the program's log, `.cfabric/logs/cfabric-browser.log` in your home folder, gives the port.)

### Asking

Ask Claude, for instance:

- *Find all cases of >LHJM/ in the BHSA.*
- *All infinitives construct of HLK[.*
- *Clauses in Genesis where the subject comes before the verb.*

The first time Claude uses a tool, it asks your permission. Claude reads a guide to the BHSA, checks its template and tries it, and tells you it has sent a search. In the program, it appears above the search form (in Parallel: click *Search* in the top bar), with your request, the template, Claude's explanation and how many results its trial found:

- *Run* runs it, as if you had typed it;
- *Edit* puts it in the editor, to change it first;
- *Dismiss* drops it.

Nothing runs by itself, and the results always come from the program's own search, never from Claude's text, so they are the same as a search you would have typed. The search keeps where it came from (*Proposed by Claude, … for: …*), in its link, in a saved workspace and when you cite it.

What you ask goes to Anthropic, as anything you ask Claude. The texts stay on your computer, apart from the few results Claude sees when it tries a template.

## Known limitations

- Fixed since test.2: switching from *Panes* to *Parallel* could give an empty page.
- Fixed since test.3: on Windows, the program could fail to start when its zip was unpacked with Explorer.
- New since test.4: the JPS 1917 translation.
- New since test.5: downloading the three corpora from the program.
- New since test.6: BDB, and the choice between vocalized and unvocalized text; the page of a frequent lexeme (הלך) opens at once instead of after about 20 seconds.
- New since test.7 (test.8 was not published: its macOS test needed a command macOS lacks): asking for a search in your own words, through Claude Desktop or Claude Code. It has been tried with test clients; you are the first to try it with Claude itself. If the extension doesn't connect, the program must be running first.

- On macOS and Linux, the window hasn't been tried by anyone yet. You're the first.
- In three places the 1917 print joins verses that the BHSA divides (the Ten Commandments in Exodus 20 and Deuteronomy 5, and Numbers 25:19 with 26:1). The program divides them as the BHSA does and shows the print's number next to those verses.
