# Printing and PDF

`Ctrl+P`, or 🖶 in the status bar, opens the **Print** panel. After it comes your system's print dialog, where you choose a printer or save a PDF.

## The Print panel

**Colors:**

- **Theme colors** prints the page as it looks on screen, the page color included, all the way to the edges. It suits a PDF meant for reading on screen.
- **White paper** keeps the theme's fonts and layout but uses Scrald Light's colors on a white page, which saves ink.

Dark themes start at White paper, and other themes at Theme colors.

**Show link addresses** prints each web or mail link's address after its text, for readers who can't click. Links whose text is already the address, links within the document, and links to files get none.

The next print from the same window starts with the same choices.

## Saving a PDF

In the print dialog, choose **Save as PDF** as the printer. It suggests the document's title as the file name, and links in the PDF stay clickable.

**Microsoft Print to PDF** works too, but it always starts with an empty file name.

## What's printed

- Only the document: no sidebars, status bar, or properties.
- The theme's own text size. Zoom and fit to window are for the screen.
- The document's title at the top of every page except the first, and the page number ("3 / 12") at the bottom.
- Folded callouts and `<details>` sections opened, and every image, formula, diagram, and piece of music drawn.
- Code that wraps instead of running off the page, and wide tables shown whole. Code blocks, figures, diagrams, callouts, and table rows aren't split across pages when they fit on one, and a heading stays with what follows it.

When the print dialog closes, the window goes back to how it was: colors, folded callouts, and your place in the document.
