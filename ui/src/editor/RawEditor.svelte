<script lang="ts">
  // Raw mode (DESIGN.md §9.2): the whole document, front matter included,
  // in CodeMirror 6, with Markdown highlighting in the theme's colors, search
  // and replace (Ctrl+F), and soft wrap at the theme's measure. Loaded only
  // when raw mode is first opened.
  import { onDestroy, onMount } from "svelte";
  import type { EditorView } from "@codemirror/view";
  import type { LineEnding } from "../lib/types";
  import { byteLength, byteToIndex, indexToEditorPos } from "./offsets";

  interface Props {
    text: string;
    lineEnding: LineEnding;
    /** Byte offset (in `text`) to show at the top when opening. */
    startByte: number;
    /** Changes when `text` was replaced from outside (a reload): show it. */
    resetKey: number;
    /** Reading font size in px; the editor uses a little less. */
    fontSize: number | null;
    /**
     * Called after the text changes. The caller debounces and then reads
     * `currentText()`, so the whole text isn't built on every keystroke.
     */
    onchange: () => void;
  }

  let { text, lineEnding, startByte, resetKey, fontSize, onchange }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let failed = $state<string | null>(null);
  let shownResetKey = -1;

  // Rust-side text uses the file's line breaks. CodeMirror stores lines
  // separately: `state.sliceDoc()` joins them with the separator set below
  // (so CRLF stays CRLF), while `doc.toString()` would always use "\n" and
  // must not be used for the file's text. Positions count a line break as
  // one, however long the separator.
  const separator = $derived<"\n" | "\r\n">(lineEnding === "crlf" ? "\r\n" : "\n");

  /** The editor's text now, exactly as the file would hold it; null before it has loaded. */
  export function currentText(): string | null {
    return view ? view.state.sliceDoc() : null;
  }

  /** Byte offset (in the text) of the first line visible at the top. */
  export function topByte(): number {
    if (!view) return startByte;
    const top = view.lineBlockAtHeight(view.scrollDOM.scrollTop);
    return byteLength(view.state.sliceDoc(0, top.from));
  }

  /** The editor position for byte offset `byte` in `fileText`. */
  function positionFor(fileText: string, byte: number): number {
    return indexToEditorPos(fileText, byteToIndex(fileText, byte), separator);
  }

  /** Scrolls so the line holding byte offset `byte` is at the top. */
  export async function scrollToByte(byte: number): Promise<void> {
    if (!view) return;
    const { EditorView } = await import("@codemirror/view");
    const index = positionFor(view.state.sliceDoc(), byte);
    view.dispatch({ effects: EditorView.scrollIntoView(index, { y: "start" }), selection: { anchor: index } });
    view.focus();
  }

  export function focus(): void {
    view?.focus();
  }

  onMount(() => {
    void (async () => {
      try {
        const [{ basicSetup }, { markdown }, { yamlFrontmatter }, { EditorView }, { EditorState }, language, { tags }] =
          await Promise.all([
          import("codemirror"),
          import("@codemirror/lang-markdown"),
          import("@codemirror/lang-yaml"),
          import("@codemirror/view"),
          import("@codemirror/state"),
          import("@codemirror/language"),
          import("@lezer/highlight"),
        ]);
        if (!host) return;
        // Markdown highlighting from the theme's variables, so every theme
        // (and a theme switch) colors raw mode without code changes.
        const highlight = language.HighlightStyle.define([
          { tag: tags.heading, color: "var(--sk-color-accent)", fontWeight: "bold" },
          { tag: tags.emphasis, fontStyle: "italic" },
          { tag: tags.strong, fontWeight: "bold" },
          { tag: tags.strikethrough, textDecoration: "line-through" },
          { tag: [tags.link, tags.url], color: "var(--sk-color-link)" },
          { tag: tags.monospace, color: "var(--sk-palette-green, var(--sk-color-accent))" },
          { tag: tags.quote, color: "var(--sk-color-muted)", fontStyle: "italic" },
          {
            tag: [tags.processingInstruction, tags.meta, tags.contentSeparator, tags.comment],
            color: "var(--sk-color-muted)",
          },
          // Front matter keys and values.
          { tag: [tags.propertyName, tags.definition(tags.propertyName)], color: "var(--sk-color-link)" },
          { tag: [tags.string, tags.number, tags.bool, tags.null], color: "var(--sk-color-foreground)" },
        ]);
        const state = EditorState.create({
          doc: text,
          extensions: [
            // Line breaks exactly as the file has them, so the text that
            // comes back is byte-for-byte the file's (no CRLF -> LF).
            EditorState.lineSeparator.of(separator),
            basicSetup,
            // Front matter is YAML, not a setext heading.
            yamlFrontmatter({ content: markdown() }),
            language.syntaxHighlighting(highlight),
            EditorView.lineWrapping,
            EditorView.updateListener.of((update) => {
              if (update.docChanged) onchange();
            }),
            EditorView.theme({
              "&": {
                height: "100%",
                color: "var(--sk-color-foreground)",
                backgroundColor: "var(--sk-color-background)",
              },
              ".cm-scroller": { fontFamily: "var(--sk-font-mono)", lineHeight: "1.55" },
              ".cm-content": {
                maxWidth: "calc(var(--sk-measure) * 1ch)",
                caretColor: "var(--sk-color-accent)",
              },
              ".cm-cursor": { borderLeftColor: "var(--sk-color-accent)" },
              ".cm-gutters": {
                backgroundColor: "var(--sk-color-background)",
                color: "var(--sk-color-muted)",
                border: "none",
              },
              ".cm-activeLine, .cm-activeLineGutter": {
                backgroundColor: "color-mix(in srgb, var(--sk-color-foreground) 5%, transparent)",
              },
              "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
                backgroundColor: "var(--sk-color-selection)",
              },
              // The search panel: CodeMirror's own fields and buttons are
              // light-themed (a pale gradient), unreadable on dark themes.
              ".cm-panels": {
                backgroundColor: "var(--sk-color-ui-background, var(--sk-color-background))",
                color: "var(--sk-color-foreground)",
                fontFamily: "var(--sk-font-ui)",
              },
              ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--sk-color-border)" },
              ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--sk-color-border)" },
              ".cm-panel.cm-search": { padding: "0.4em 2em 0.5em 0.6em" },
              ".cm-textfield": {
                color: "var(--sk-color-foreground)",
                backgroundColor: "var(--sk-color-background)",
                border: "1px solid var(--sk-color-border)",
                borderRadius: "4px",
                padding: "0.15em 0.4em",
              },
              ".cm-textfield:focus": {
                outline: "none",
                borderColor: "var(--sk-color-accent)",
              },
              ".cm-textfield::placeholder": { color: "var(--sk-color-muted)" },
              ".cm-button": {
                color: "var(--sk-color-foreground)",
                backgroundColor: "var(--sk-color-background)",
                backgroundImage: "none",
                border: "1px solid var(--sk-color-border)",
                borderRadius: "4px",
                padding: "0.15em 0.6em",
                cursor: "pointer",
              },
              ".cm-button:hover": { borderColor: "var(--sk-color-accent)" },
              ".cm-button:active": {
                backgroundImage: "none",
                backgroundColor: "var(--sk-color-selection)",
              },
              ".cm-panel.cm-search label": { color: "var(--sk-color-foreground)" },
              ".cm-panel.cm-search input[type=checkbox]": { accentColor: "var(--sk-color-accent)" },
              ".cm-panel.cm-search [name=close]": {
                color: "var(--sk-color-muted)",
                fontSize: "1.2em",
                cursor: "pointer",
              },
              ".cm-panel.cm-search [name=close]:hover": { color: "var(--sk-color-foreground)" },
              ".cm-searchMatch": {
                backgroundColor: "var(--sk-color-highlight-bg)",
              },
              // After .cm-searchMatch, so the current match wins.
              ".cm-searchMatch.cm-searchMatch-selected": {
                backgroundColor: "var(--sk-color-selection)",
                outline: "1px solid var(--sk-color-accent)",
              },
              "&.cm-focused": { outline: "none" },
            }),
          ],
        });
        view = new EditorView({ parent: host, state });
        shownResetKey = resetKey;
        const index = positionFor(text, startByte);
        view.dispatch({ effects: EditorView.scrollIntoView(index, { y: "start" }), selection: { anchor: index } });
        view.focus();
      } catch (e) {
        failed = `The editor couldn't load: ${String(e)}`;
      }
    })();
  });

  // The text was replaced from outside (a reload from disk): show it.
  $effect(() => {
    const key = resetKey;
    const newText = text;
    if (view && key !== shownResetKey) {
      shownResetKey = key;
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: newText } });
    }
  });

  onDestroy(() => view?.destroy());
</script>

<div class="sk-raw" bind:this={host} style:font-size={fontSize === null ? null : `${fontSize * 0.85}px`}>
  {#if failed}<p class="sk-raw-error">{failed}</p>{/if}
</div>

<style>
  .sk-raw {
    height: 100%;
    overflow: hidden;
  }

  .sk-raw :global(.cm-editor) {
    height: 100%;
  }

  .sk-raw :global(.cm-scroller) {
    padding: 1.5rem 0 40vh;
  }

  .sk-raw-error {
    margin: 2rem;
    color: var(--sk-color-muted);
  }
</style>
