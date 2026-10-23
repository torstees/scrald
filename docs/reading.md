# Reading

## The window

The document fills the middle of the window. Around it:

- **Contents** at the left: every heading, with the current one highlighted as you scroll. Click one to jump there.
- **Properties**: the document's front matter (title, tags, summary, and so on). See [Front matter](front-matter.md).
- **The status bar** at the bottom:
  - ☰ shows or hides the contents, and ‹ › go back and forward;
  - the current section;
  - the theme, the text size and zoom, and the Markdown flavor, each of which opens a menu;
  - the word count and how far you've read ("42% · 7 min left"), also shown as a thin line along the top of the status bar;
  - 🖶 to print, ⚙ for settings, and ? for keyboard shortcuts.

`Ctrl+\` shows or hides the contents, and `Ctrl+Shift+\` the properties.

## The contents

The **Depth** menu at the top of the contents chooses how many heading levels it lists, from one to all. For a novel, one or two levels keep the list short. Depth is one setting for every document.

Headings with others under them can be folded with the arrow beside them.

## Properties

The properties can sit in one of three places, chosen from the **Show** menu in the panel or in Settings:

- **At the top** of the document (the default), scrolling with it.
- **At the left**, as a tab beside Contents.
- **At the right**, in a sidebar of their own, so the contents and properties can both be open.

Whether they're shown is remembered too. In a narrow window, only one sidebar fits, so opening one closes the other until the window is wide again.

## Text size and zoom

Each theme sets a text size and a line length (66 characters by default). When the window is narrower than the line, the text rewraps.

The text-size menu in the status bar offers two ways to size text:

- **Fixed:** the theme's size, times the zoom. A wider window adds margin.
- **Fit to window:** the text grows or shrinks so a line fills the window, within limits the theme sets. It suits a large monitor, where you want bigger text without zooming.

**Zoom** with `Ctrl+=` and `Ctrl+-`, `Ctrl+wheel`, or a trackpad pinch, and reset it with `Ctrl+0`. The text under the pointer stays in place.

Scrald remembers the text size and zoom for each document. **Make this the default** uses the current ones for documents you haven't changed. **Fill window**, in the same menu, lets the text run the window's full width, ignoring the line length; it applies to every document.

Only the document's text changes size: the contents, properties sidebar, and status bar stay as they are.

## Finding text

`Ctrl+F` opens the find bar. It searches the text as you see it, not the Markdown behind it, so `**Ragnar** Lothbrok` is found by "Ragnar Lothbrok", and a phrase is found across a line break.

- `Enter` or `F3` goes to the next match, `Shift+Enter` or `Shift+F3` to the previous one.
- **Match case** and **Whole word** narrow the search.
- A match inside a folded callout opens it.
- Footnotes and code are searched. Math, diagrams, music, and the properties aren't.
- `Esc` closes the bar.

## Footnotes

Hover over a footnote number, or tab to it, to see the note in a popover. Clicking the number jumps to the notes at the end of the document, and ↩ after a note jumps back.

## Links

- **Web and mail links** open in your browser or mail program.
- **Links to other Markdown files** open in the same window. `Alt+Left` and `Alt+Right`, the mouse's back and forward buttons, or ‹ › in the status bar move through the documents you've visited, each at the place you left it.
- **Links to other files** don't open, so a link can never run a program. The status bar shows the file's path instead.
- **Obsidian's `[[wikilinks]]`** are found within the vault (the folder holding `.obsidian`), or the document's folder outside a vault.

## Images

Images next to the document, in a folder named by the `assets` property, or anywhere in an Obsidian vault are shown. A missing image shows its file name instead; hover over it to see where Scrald looked.

**Images from the web are blocked**, so a document can't tell anyone that you opened it. The status bar says how many were blocked, and **Load** shows them for that document. Scrald remembers the choice, and **Block** reverses it.

## Reading position

Scrald remembers where you were in each document, and opens it there next time. The position is kept by paragraph rather than by pixel, so it survives a new theme, zoom, a resized window, or edits made elsewhere.

When a document changes on disk while it's open, and you haven't changed it in Scrald, it reloads in place, at the same spot.

## Settings

`Ctrl+,` or ⚙ opens the settings:

- **Appearance:** the default theme, or following the system's light or dark mode.
- **Reading:** text size and zoom for documents without their own, **Fill window**, the contents depth, and where the properties go.
- **Editing:** autosave. See [Editing and saving](editing.md).
- **New documents:** the folders to remember.

Changes apply at once, to every open window.
