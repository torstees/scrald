<script lang="ts">
  // The block editor (DESIGN.md §9.1): a small CodeMirror editor holding one
  // block's Markdown source in place of its rendered HTML. Clicking outside,
  // Escape, or Ctrl+Enter commits. CodeMirror is loaded only when a block is
  // first edited.
  import { onDestroy, onMount } from "svelte";
  import type { EditorView } from "@codemirror/view";

  interface Props {
    source: string;
    /** Called once, with the edited text (unchanged text included). */
    oncommit: (text: string) => void;
  }

  let { source, oncommit }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let committed = false;
  let failed = $state<string | null>(null);

  function commit(): boolean {
    if (committed) return true;
    committed = true;
    oncommit(view ? view.state.doc.toString() : source);
    return true;
  }

  onMount(() => {
    void (async () => {
      try {
        const [{ minimalSetup }, { markdown }, { EditorView, keymap }, { EditorState }] = await Promise.all([
          import("codemirror"),
          import("@codemirror/lang-markdown"),
          import("@codemirror/view"),
          import("@codemirror/state"),
        ]);
        if (!host || committed) return;
        view = new EditorView({
          parent: host,
          state: EditorState.create({
            doc: source,
            extensions: [
              // Highest precedence, so these win over CodeMirror's own keys.
              keymap.of([
                { key: "Mod-Enter", run: commit },
                { key: "Escape", run: commit },
              ]),
              minimalSetup,
              markdown(),
              EditorView.lineWrapping,
              EditorView.domEventHandlers({ blur: () => void commit() }),
              EditorView.theme({
                "&": { color: "var(--sk-color-foreground)", backgroundColor: "var(--sk-color-code-bg)" },
                ".cm-content": { fontFamily: "var(--sk-font-mono)", caretColor: "var(--sk-color-accent)" },
                ".cm-cursor": { borderLeftColor: "var(--sk-color-accent)" },
                "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
                  backgroundColor: "var(--sk-color-selection)",
                },
                "&.cm-focused": { outline: "none" },
              }),
            ],
          }),
        });
        view.focus();
        view.dispatch({ selection: { anchor: view.state.doc.length } });
      } catch (e) {
        failed = `The editor couldn't load: ${String(e)}`;
      }
    })();
  });

  onDestroy(() => view?.destroy());
</script>

<div class="sk-block-editor" bind:this={host} aria-label="Edit block (Ctrl+Enter or Escape to finish)">
  {#if failed}<p class="sk-editor-error">{failed}</p>{/if}
</div>

<style>
  .sk-block-editor {
    border: 1px solid var(--sk-color-accent);
    border-radius: 6px;
    overflow: hidden;
    font-size: 0.85em;
    line-height: 1.5;
  }

  .sk-block-editor :global(.cm-editor) {
    padding: 0.5em 0.7em;
  }

  .sk-editor-error {
    margin: 0.5em;
    color: var(--sk-color-muted);
  }
</style>
