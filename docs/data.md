# Your data

## What Scrald keeps, and where

Scrald keeps everything in your user profile, never next to your documents:

| What | Where |
|---|---|
| What it remembers about each document (theme, flavor, position, zoom, window size, title, summary), recent and pinned documents, and settings | `%APPDATA%\io.github.torstees.scrald\scrald.db` |
| Unsaved changes, for crash recovery (deleted once saved or discarded) | `%APPDATA%\io.github.torstees.scrald\recovery\` |
| Your themes | `%APPDATA%\io.github.torstees.scrald\themes\` |
| Your templates for new documents | `%APPDATA%\io.github.torstees.scrald\templates\` |
| Downloaded Google fonts | `%LOCALAPPDATA%\io.github.torstees.scrald\fonts\` |

The database holds facts about your documents, not their text. The only copies of document text Scrald makes are the recovery files, while there are unsaved changes.

Removing a document from the start screen's list (×) deletes what Scrald remembered about it.

## Moved and renamed files

Scrald recognizes a document by its path and, as a backup, by its size and first few bytes. If you move or rename a file, the next time you open it Scrald finds its old entry and keeps its theme and position. A copy, where the original is still in place, starts fresh.

## Privacy

Scrald contacts a server by itself only to download a Google font that a theme asks for, once per font.

Otherwise, nothing leaves your computer unless you ask:

- images from the web are blocked until you choose **Load** for a document;
- web links open in your browser only when you click them.

Documents can't run code: scripts and anything else that could run in the page are removed, and a link to a program on your computer never opens it.
