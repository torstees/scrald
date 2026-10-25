# Front matter

Front matter is a block of YAML at the very top of a document, between `---` lines:

```yaml
---
title: The Long Winter
author: A. Writer
summary: A family survives a hard season on the northern coast.
tags: [novel, draft, norse]
cover: images/cover.jpg
scrald-theme: sepia
---
```

Scrald shows it as the document's properties (see [Reading](reading.md)), and you can edit it there (see [Editing and saving](editing.md)).

## Properties Scrald understands

Scrald reads the keys that Obsidian, Hugo, Jekyll, and Pandoc use, so the same document works in each.

| Key | Also accepted | Meaning |
|---|---|---|
| `title` | | The title, shown in the window title, the start screen, and printed pages |
| `author` | `authors` | One name or a list |
| `summary` | `synopsis`, `description`, `abstract`, `excerpt` | A short description, shown on the start screen |
| `tags` | `tag`, `keywords` | One tag or a list. A leading `#` is dropped. |
| `notes` | | Notes to yourself |
| `source` | `url` | Where the text came from |
| `assets` | `asset_dir`, `attachments` | A folder to look in for images, relative to the document |
| `scrald-theme` | | The document's theme, by id (see [Themes](themes.md)) |
| `scrald-flavor` | | `gfm`, `obsidian`, or `pandoc` (see [Markdown flavors](markdown.md)) |

Every other key is kept and shown in the properties too.

## Cover images

A cover image is shown above the document's text (see "The cover and the title" in [Reading](reading.md)), and in place of the theme swatch on the start screen. Scrald takes the first of these keys that's set: `cover`, `cover_image`, `cover-image`, `coverImage`, `image`, `banner`, `feature_image`, `featured_image`, `featureImage`, `featuredImage`, `hero_image`, `heroImage`, `hero`, `thumbnail`, `images`.

The value can be a path, a list (the first item is used), a map with `image`, `src`, `url`, or `path` (as Hugo themes write it), or Obsidian's `"[[cover.jpg]]"`. It's found the way the document's images are. Only images on your computer are used, not ones from the web.

## How Scrald edits front matter

Many programs rewrite the whole block when one property changes, losing comments and changing how lists are written. Scrald changes only the property you edit:

- comments, blank lines, and the order of keys stay as they are;
- a list written `[a, b]` stays that way, and so does one written with `-` on each line;
- text is quoted only when YAML needs it.

Properties Scrald can't change safely, such as nested maps, are marked "read-only", with a link to edit them in raw mode.

Scrald writes its own settings as `scrald-theme` and `scrald-flavor`. It also reads the nested form some tools use (`scrald: { theme: sepia }`), but never writes it.
