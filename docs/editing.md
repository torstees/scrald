# Editing and saving

<img src="art/editing.svg" alt="The Skald writes with his quill-horn" width="200" align="right" class="right">

Scrald is a reader first, but small changes don't need another program.

## Editing a block

Double-click a paragraph, heading, list, table, or other block to edit its Markdown in place. `Ctrl+Enter`, `Esc`, or clicking elsewhere finishes, and the block is shown rendered again.

In a list, double-clicking an item edits just that item, with the lists inside it. In a quote, it edits just the paragraph, list, or other part you clicked, with its `>` markers. The rest of the list or quote stays in view around the editor. Emptying an item removes it, along with the blank line beside it.

To edit a whole list or quote at once, hold `Alt` (`Option` on a Mac) while double-clicking. Callouts, and quotes with HTML in them, are always edited whole.

## Task lists

Click the checkbox of a task item (`- [ ] Buy milk`) to check or uncheck it, or tab to it and press `Space`. Only the one character between the brackets changes: an `x` when checking (or `X`, if most of the document's checked items use it), a space when unchecking. Like other edits, it's unsaved until you save, and `Ctrl+Z` undoes it.

Checkboxes in a note shown inside another stay as they are, since the note is read-only there. To hide the checked items, see [Reading](reading.md).

## Raw mode

`Ctrl+E` switches the whole document to its Markdown source, front matter included, and back. You stay at the same place in the document both ways.

Raw mode has line numbers, Markdown coloring in the theme's colors, and its own find and replace on `Ctrl+F`. Clicking a heading in the contents jumps to it in the source.

## Line endings

Scrald keeps a file's line endings (Windows `CRLF` or Unix `LF`) and its byte order mark (BOM) exactly as they are. The status bar shows which the document uses: `LF`, `CRLF`, or `Mixed` when it has both, with `· BOM` after it when the file starts with one.

Click it to **Convert to LF** or **Convert to CRLF**, or to **Remove BOM**. Each is an ordinary change: `Ctrl+Z` undoes it, and the file changes only when you save.

## Tidying hard-wrapped text

Text pasted from a PDF, an email, or a plain-text book often has a line break at the end of every line, and no blank line between paragraphs, so the paragraphs run together. **Tidy hard-wrapped text** (`Alt+Q`, `Option+Q` on a Mac, also in the line-ending menu in the status bar and, on a Mac, in the Edit menu) fixes that:

- Lines broken in the middle of a paragraph are joined.
- A blank line goes where a paragraph seems to end: a short line that ends a sentence, followed by one that starts like a new sentence.
- A word broken with a hyphen at the end of a line (`exam-` / `ple`) is joined without it, unless the document writes it with a hyphen elsewhere (`well-known`).

Only ordinary paragraphs are tidied. Headings, lists, quotes, tables, code, and front matter stay as they are, and so do line breaks you made on purpose (two spaces or `\` at the end of a line). In raw mode, only the selected text is tidied, if you've selected some.

These are guesses, so Scrald shows the result in raw mode beside the text as it was. The arrows between the two sides put back a change you don't want. **Done** keeps the tidied text as unsaved changes, and **Undo tidy** puts the text back as it was.

## Properties

Click a property's value to change it. Lists such as tags are chips: × removes one, and the box after them adds more, separated by commas. Theme and flavor are chosen from a list, and **assets** has a **Choose folder…** button. **+ Add property** adds the common properties a document doesn't have yet.

Scrald changes only the line of the property you edit, so comments, order, and quoting in the front matter stay as they were. A property too complex to edit safely is marked "read-only", with **Edit in raw mode** to change it there.

## Undo

`Ctrl+Z` undoes, and `Ctrl+Y` or `Ctrl+Shift+Z` redoes, across every kind of edit: blocks, properties, and raw mode. Undoing back to the saved text clears the unsaved mark.

## Saving

While there are unsaved changes, the window title starts with "• ". `Ctrl+S` saves.

Closing the window, opening another document, or going back asks whether to **Save**, **Don't save**, or **Cancel**.

On a Mac, **Quit Scrald** (`Cmd+Q`) closes each window the same way: windows without unsaved changes close, and each one with changes asks. Cancelling keeps Scrald open with that window. Quitting from the Dock, logging out, and shutting down ask the same way; cancelling there also stops the logout or shutdown, as other Mac apps do.

Saving writes the whole file to a temporary file next to it and then swaps it in, so a failed save never leaves half a file. Only what you changed differs: line endings (Windows or Unix) and a byte order mark are kept as they were.

### Autosave

Autosave is off unless you turn it on in Settings. When it's on, the status bar says "Autosave", and a document saves itself 5 seconds after your last change. Closing saves instead of asking.

## When the file changes elsewhere

If another program changes the file while it's open:

- **With no unsaved changes in Scrald**, the document reloads at the same place.
- **With unsaved changes**, a banner offers:
  - **Keep mine:** your version stays, and `Ctrl+S` replaces the file.
  - **Load disk version:** the file's version replaces yours. `Ctrl+Z` brings yours back.
  - **Compare:** the two side by side, the file's on the left and yours on the right, with arrows to copy changes across. **Done: keep my version** or **Use the disk version** finishes.

Saving always checks the file first, so Scrald never overwrites a change it hasn't shown you. If the file is moved or deleted, the banner says so, and `Ctrl+S` writes it again.

## Crash recovery

While a document has unsaved changes, Scrald copies them every few seconds to a recovery file in its own data folder, never next to the document. The copy is deleted when you save or discard the changes.

If Scrald or Windows stops before that, opening the document again shows a banner:

- **Restore** brings the changes back, as unsaved changes you can undo.
- **Compare** shows the file on disk next to the recovered text, so you can take parts of each.
- **Discard** deletes the recovered changes.

Starting Scrald without a document lists every set of recovered changes on the start screen. **Copy text** copies the changes of a document that no longer exists.
