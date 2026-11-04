# Getting started

## Installing

<img src="art/getting-started.svg" alt="The Skald waves hello" width="200" align="right" class="right">

Scrald doesn't have a published release yet, so for now it's built from source, on Windows 11, macOS, or Linux (checked on Ubuntu 24.04). You'll need:

- [Rust](https://rustup.rs) (stable; on Windows, with the MSVC toolchain);
- [Node.js](https://nodejs.org) (LTS);
- on Windows, Visual Studio Build Tools with the C++ workload; on macOS, the Xcode Command Line Tools (`xcode-select --install`); on Ubuntu or Debian, the WebKitGTK and build packages:

  ```
  sudo apt install build-essential curl file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

- [just](https://github.com/casey/just) (`winget install Casey.Just` on Windows, `brew install just` on macOS, `sudo apt install just` on Ubuntu).

After installing Rust, open a new terminal, so that `cargo` is on your path.

From a copy of the [repository](https://github.com/torstees/scrald), install the frontend's packages:

```
just install
```

Then build the installer:

```
just bundle
```

- **On Windows,** it's written to `target/release/bundle/nsis/`, named like `Scrald_0.1.0_x64-setup.exe`. It installs Scrald for your user account only, so it doesn't need administrator rights.
- **On macOS,** you get `Scrald.app` in `target/release/bundle/macos/` and a disk image, named like `Scrald_0.1.0_aarch64.dmg`, in `target/release/bundle/dmg/`. Open the disk image and drag Scrald to Applications. The app isn't signed yet, so the first time, macOS may refuse to open it: Control-click it, choose **Open**, then **Open** again.
- **On Linux,** you get a Debian package in `target/release/bundle/deb/`, named like `Scrald_0.1.0_amd64.deb`, and an AppImage in `target/release/bundle/appimage/`. Install the package with `sudo apt install ./Scrald_0.1.0_amd64.deb` (with your file's name): that adds Scrald to the applications menu and to **Open With** for Markdown files. The AppImage runs from anywhere without installing, but your file manager won't offer it for Markdown files.
- **If Scrald's window stays blank on Linux,** which happens in virtual machines and with some graphics drivers, start it with `WEBKIT_DISABLE_DMABUF_RENDERER=1 scrald`.

## Opening documents

Scrald reads `.md`, `.markdown`, and `.mdown` files, plus `.mkd`, `.mkdn`, and `.txt` when you open them yourself.

- **From File Explorer (Windows):** right-click a Markdown file and choose **Open with** > **Scrald**. The installer adds Scrald to that list but doesn't make it the default. To make it the default, choose **Always** in the **Open with** prompt, or set it in Windows Settings > Apps > Default apps.
- **From Finder (macOS):** Control-click a Markdown file and choose **Open With** > **Scrald**, or drop it on Scrald's icon in the Dock. If you've never picked an app for Markdown files, macOS may make Scrald the one a double-click uses. To choose another, or to make Scrald the default, select a file, choose **File** > **Get Info**, pick the app under **Open with**, and click **Change All…**.
- **From the file manager (Linux):** right-click a Markdown file and choose **Open With…** > **Scrald**. As on a Mac, if no other app claims Markdown files, Scrald becomes the one a double-click uses. To choose another, use the same dialog's **Always use for this file type**, or run `xdg-mime default org.gnome.TextEditor.desktop text/markdown` with the app you want.
- **From Scrald:** `Ctrl+O`, or **Open…** on the start screen. The file dialog starts in the open document's folder.
- **By dropping it:** drop a Markdown file anywhere on a Scrald window.
- **From a terminal:** `scrald path/to/file.md`.

Opening another file while Scrald is running opens it in a new window. If the file is already open, its window comes to the front instead, so a document is never open twice.

With a document open, `Ctrl+O` and dropping a file open the new one in the same window. Scrald asks first if there are unsaved changes, and `Alt+Left` (or the ‹ button in the status bar) goes back.

## The start screen

Starting Scrald without a file shows the start screen:

- **Pinned** and **Recent** documents, each with its title, folder, when you last opened it, and a swatch of its theme. A document with a cover image (see [Front matter](front-matter.md)) shows the cover instead, and one with a summary shows it under the title.
- ☆ pins a document to the top, and × takes it off the list, which also forgets what Scrald remembered about it (theme, position, zoom).
- A file that has moved or been deleted is dimmed and marked "Not found".
- **New document…**, **Open…**, **Settings**, and **Keyboard shortcuts**.
- **About Scrald**, beside the name: the version, where the name comes from, and a link to the source. Settings has it too, and on a Mac so does the **Scrald** menu at the top left of the screen.

If Scrald stopped while a document had unsaved changes, the start screen lists those changes first, so you can restore them. See "Crash recovery" in [Editing and saving](editing.md).

## Creating a document

`Ctrl+N`, or **New document…** on the start screen, asks for:

- a **title** (optional), which also suggests the file name;
- a **template**: Plain, Obsidian note, Pandoc document, or one of your own;
- the **file name** (`.md` is added unless you type another Markdown extension);
- the **folder**: folders you've told Scrald to remember come first, then the folders of your recent documents, or **Choose folder…**.

The new file opens in raw mode, ready to type. Scrald never replaces an existing file: if the name is taken, the dialog says so.

**Your own templates** are the `.md` files in Scrald's templates folder; the dialog's **Templates folder** link opens it. A template can have front matter and text. `{{date}}` becomes today's date anywhere in it, and `{{title}}` becomes the title in the text below the front matter. Scrald puts a `Sample.md` there the first time it runs, which you can edit or delete.

Each remembered folder starts with the template you last used in it.
