# Editing and saving

Scrald is a reader first, but small changes don't need another program.

## Editing a block

Double-click a paragraph, heading, list, table, or other block to edit its Markdown in place. `Ctrl+Enter`, `Esc`, or clicking elsewhere finishes, and the block is shown rendered again.

A block is a whole top-level piece of the document: editing one item of a list edits the whole list, and editing inside a quote edits the whole quote.

## Raw mode

`Ctrl+E` switches the whole document to its Markdown source, front matter included, and back. You stay at the same place in the document both ways.

Raw mode has line numbers, Markdown coloring in the theme's colors, and its own find and replace on `Ctrl+F`. Clicking a heading in the contents jumps to it in the source.

## Properties

Click a property's value to change it. Lists such as tags are chips: × removes one, and the box after them adds more, separated by commas. Theme and flavor are chosen from a list, and **assets** has a **Choose folder…** button. **+ Add property** adds the common properties a document doesn't have yet.

Scrald changes only the line of the property you edit, so comments, order, and quoting in the front matter stay as they were. A property too complex to edit safely is marked "read-only", with **Edit in raw mode** to change it there.

## Undo

`Ctrl+Z` undoes, and `Ctrl+Y` or `Ctrl+Shift+Z` redoes, across every kind of edit: blocks, properties, and raw mode. Undoing back to the saved text clears the unsaved mark.

## Saving

While there are unsaved changes, the window title starts with "• ". `Ctrl+S` saves.

Closing the window, opening another document, or going back asks whether to **Save**, **Don't save**, or **Cancel**.

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
