<script lang="ts">
  // Theme editor (DESIGN.md §7.2): a panel beside the document that edits a
  // user theme's colors, fonts, layout, and element styles. Every change is
  // previewed on the document at once; nothing is written until Save, which
  // changes only the edited keys in theme.toml (comments stay).
  import { restoreFocus } from "../lib/focus";
  import { onMount } from "svelte";
  import type { ThemeChange, ThemeDetails, ThemeEditorValues, ThemeFontValue } from "../lib/types";
  import {
    changeList,
    colorFieldValue,
    colorPickerValue,
    parseList,
    parseWeights,
    recordChange,
    withAlphaOf,
  } from "./editor";

  interface Props {
    details: ThemeDetails;
    /** Installed font families, suggested in the font fields. */
    fonts: string[];
    /** Preview `changes`; resolves to the edited values, or rejects with the problems. */
    onpreview: (changes: ThemeChange[]) => Promise<ThemeEditorValues>;
    onsave: (changes: ThemeChange[]) => Promise<void>;
    oncancel: () => void;
  }

  let { details, fonts, onpreview, onsave, oncancel }: Props = $props();

  const COLOR_LABELS: Record<string, string> = {
    background: "Page",
    foreground: "Text",
    muted: "Muted text",
    accent: "Accent",
    link: "Links",
    selection: "Selection",
    border: "Borders",
    code_bg: "Code background",
    quote_bar: "Quote bar",
    highlight_bg: "Highlight",
    ui_background: "Sidebar and status bar",
  };
  const DEFAULT_GLYPH = "❦";
  const FONT_LABELS: Record<ThemeFontValue["slot"], string> = {
    body: "Body",
    heading: "Headings",
    mono: "Code",
    ui: "Interface",
  };

  // The panel edits its own copy; `details` is only where it starts.
  // svelte-ignore state_referenced_locally
  const start = $state.snapshot(details.values);
  let values = $state<ThemeEditorValues>(start);
  let colorText = $state<Record<string, string>>(Object.fromEntries(start.colors.map((c) => [c.key, c.value ?? ""])));
  let changes = $state(new Map<string, ThemeChange["value"]>());
  let problem = $state<string | null>(null);
  let saving = $state(false);
  let panel: HTMLElement | undefined = $state();

  onMount(() => panel?.querySelector<HTMLInputElement>("input")?.focus());

  let previewTimer: number | undefined;
  let previewSeq = 0;

  function set(key: string, value: ThemeChange["value"]): void {
    changes = recordChange(changes, key, value);
    window.clearTimeout(previewTimer);
    previewTimer = window.setTimeout(() => void preview(), 150);
  }

  async function preview(): Promise<void> {
    const seq = ++previewSeq;
    try {
      const edited = await onpreview(changeList(changes));
      if (seq !== previewSeq) return;
      problem = null;
      // Derived colors follow the edited ones; the text fields stay as typed.
      values.colors = edited.colors;
    } catch (e) {
      if (seq === previewSeq) problem = String(e);
    }
  }

  function resolved(key: string): string {
    return values.colors.find((c) => c.key === key)?.resolved ?? "#000000";
  }

  function onColorText(key: string, text: string): void {
    colorText[key] = text;
    const value = colorFieldValue(text);
    if (value !== undefined) set(`colors.${key}`, value);
  }

  function onColorPicked(key: string, picked: string): void {
    const value = withAlphaOf(picked, colorFieldValue(colorText[key] ?? "") ?? null);
    colorText[key] = value;
    set(`colors.${key}`, value);
  }

  function font(slot: ThemeFontValue["slot"]): ThemeFontValue {
    // Every slot is listed by the app, so this always finds one.
    return values.fonts.find((f) => f.slot === slot) ?? values.fonts[0]!;
  }

  function onFamily(slot: ThemeFontValue["slot"], text: string): void {
    const family = text.trim();
    font(slot).family = family === "" ? null : family;
    // An empty family leaves the slot to the default: remove it entirely.
    set(family === "" ? `fonts.${slot}` : `fonts.${slot}.family`, family === "" ? null : family);
  }

  /** The ornament divider's character; empty means the usual ❦. */
  function setGlyph(text: string): void {
    values.hrGlyph = text.trim() || DEFAULT_GLYPH;
    set("elements.hr.glyph", values.hrGlyph);
  }

  async function save(): Promise<void> {
    if (changes.size === 0) {
      oncancel();
      return;
    }
    saving = true;
    try {
      await onsave(changeList(changes));
    } catch (e) {
      problem = String(e);
    } finally {
      saving = false;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      oncancel();
    } else if (event.key === "Enter" && event.ctrlKey) {
      event.preventDefault();
      void save();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside bind:this={panel} use:restoreFocus class="sk-theme-editor" aria-label="Edit theme" onkeydown={onKeydown}>
  <header>
    <h2>Edit theme</h2>
    <p class="hint" title={details.folder ?? undefined}>Changes show on the document as you make them. Save writes theme.toml.</p>
  </header>

  <datalist id="sk-system-fonts">
    {#each fonts as family (family)}<option value={family}></option>{/each}
  </datalist>

  <section>
    <h3>Theme</h3>
    <label class="row">
      <span>Name</span>
      <input
        type="text"
        value={values.name}
        onchange={(e) => {
          const name = e.currentTarget.value.trim();
          if (name !== "") set("meta.name", name);
        }}
      />
    </label>
    <label class="row">
      <span>Appearance</span>
      <select value={values.appearance} onchange={(e) => set("meta.appearance", e.currentTarget.value)}>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
  </section>

  <section>
    <h3>Colors</h3>
    <p class="hint">Leave a color empty to derive it from the others.</p>
    {#each values.colors as color (color.key)}
      <div class="row color">
        <label for={`sk-color-${color.key}`}>{COLOR_LABELS[color.key] ?? color.key}</label>
        <input
          type="color"
          aria-label={`${COLOR_LABELS[color.key] ?? color.key} color picker`}
          value={colorPickerValue(colorFieldValue(colorText[color.key] ?? "") ?? resolved(color.key))}
          oninput={(e) => onColorPicked(color.key, e.currentTarget.value)}
        />
        <input
          id={`sk-color-${color.key}`}
          type="text"
          spellcheck="false"
          placeholder={`${resolved(color.key)} (derived)`}
          value={colorText[color.key] ?? ""}
          oninput={(e) => onColorText(color.key, e.currentTarget.value)}
        />
        {#if (colorText[color.key] ?? "") !== ""}
          <button
            class="x"
            title="Derive this color instead"
            aria-label={`Derive ${COLOR_LABELS[color.key] ?? color.key}`}
            onclick={() => onColorText(color.key, "")}>×</button
          >
        {/if}
      </div>
    {/each}
  </section>

  <section>
    <h3>Fonts</h3>
    {#each values.fonts as f (f.slot)}
      <fieldset>
        <legend>{FONT_LABELS[f.slot]}</legend>
        <label class="row">
          <span>Family</span>
          <input
            type="text"
            list="sk-system-fonts"
            placeholder="Theme default"
            value={f.family ?? ""}
            onchange={(e) => onFamily(f.slot, e.currentTarget.value)}
          />
        </label>
        {#if f.source === "theme"}
          <p class="hint">Uses font files from the theme's folder; edit theme.toml to change them.</p>
        {:else if f.family !== null}
          <label class="row">
            <span>Source</span>
            <select
              value={f.source}
              onchange={(e) => {
                f.source = e.currentTarget.value as ThemeFontValue["source"];
                set(`fonts.${f.slot}.source`, f.source);
              }}
            >
              <option value="system">Installed on this computer</option>
              <option value="google">Google Fonts (downloaded once)</option>
            </select>
          </label>
          {#if f.source === "google"}
            <label class="row">
              <span>Weights</span>
              <input
                type="text"
                placeholder="400, 700"
                value={f.weights.join(", ")}
                onchange={(e) => set(`fonts.${f.slot}.weights`, parseWeights(e.currentTarget.value))}
              />
            </label>
            <label class="row check">
              <input
                type="checkbox"
                checked={f.italic}
                onchange={(e) => set(`fonts.${f.slot}.italic`, e.currentTarget.checked)}
              />
              <span>Italics</span>
            </label>
          {/if}
          <label class="row">
            <span>Fallbacks</span>
            <input
              type="text"
              list="sk-system-fonts"
              placeholder="Georgia, serif"
              value={f.fallback.join(", ")}
              onchange={(e) => set(`fonts.${f.slot}.fallback`, parseList(e.currentTarget.value))}
            />
          </label>
        {/if}
      </fieldset>
    {/each}
  </section>

  <section>
    <h3>Layout</h3>
    <label class="row">
      <span>Line length</span>
      <input
        type="number"
        min="20"
        max="200"
        aria-label="Line length in characters"
        value={values.measure}
        onchange={(e) => set("layout.measure", Number(e.currentTarget.value))}
      />
      <span class="unit">characters</span>
    </label>
    <label class="row">
      <span>Text size</span>
      <input
        type="number"
        min="8"
        max="72"
        aria-label="Text size in pixels"
        step="0.5"
        value={values.fontSize}
        onchange={(e) => set("layout.font_size", Number(e.currentTarget.value))}
      />
      <span class="unit">px</span>
    </label>
    <label class="row">
      <span>Line height</span>
      <input
        type="number"
        min="1"
        max="3"
        step="0.05"
        value={values.lineHeight}
        onchange={(e) => set("layout.line_height", Number(e.currentTarget.value))}
      />
    </label>
    <label class="row">
      <span>Paragraph gap</span>
      <input
        type="number"
        min="0"
        max="4"
        aria-label="Paragraph gap in em"
        step="0.1"
        value={values.paragraphSpacing}
        onchange={(e) => set("layout.paragraph_spacing", Number(e.currentTarget.value))}
      />
      <span class="unit">em</span>
    </label>
    <label class="row">
      <span>Text sizing</span>
      <select value={values.textSizing} onchange={(e) => set("layout.text_sizing", e.currentTarget.value)}>
        <option value="fixed">Fixed</option>
        <option value="fit">Fit the window</option>
      </select>
    </label>
  </section>

  <section>
    <h3>Elements</h3>
    <label class="row">
      <span>Quotes</span>
      <select value={values.blockquoteStyle} onchange={(e) => set("elements.blockquote.style", e.currentTarget.value)}>
        <option value="bar">Bar</option>
        <option value="indent">Indent</option>
        <option value="boxed">Boxed</option>
        <option value="pull-quote">Pull quote</option>
      </select>
    </label>
    <label class="row check">
      <input
        type="checkbox"
        checked={values.blockquoteItalic}
        onchange={(e) => set("elements.blockquote.italic", e.currentTarget.checked)}
      />
      <span>Quotes in italics</span>
    </label>
    {#if values.hrStyle !== "image"}
      <label class="row">
        <span>Dividers</span>
        <select
          value={values.hrStyle}
          onchange={(e) => {
            values.hrStyle = e.currentTarget.value as ThemeEditorValues["hrStyle"];
            set("elements.hr.style", values.hrStyle);
            // An ornament divider needs its character; start with the usual one.
            if (values.hrStyle === "ornament" && !values.hrGlyph) setGlyph("");
          }}
        >
          <option value="line">Line</option>
          <option value="dots">Dots</option>
          <option value="ornament">Ornament</option>
        </select>
      </label>
      {#if values.hrStyle === "ornament"}
        <label class="row">
          <span>Ornament</span>
          <input
            type="text"
            placeholder={DEFAULT_GLYPH}
            value={values.hrGlyph ?? ""}
            onchange={(e) => setGlyph(e.currentTarget.value)}
          />
        </label>
      {/if}
    {:else}
      <p class="hint">Dividers use an image from the theme's folder; edit theme.toml to change them.</p>
    {/if}
    <label class="row check">
      <input
        type="checkbox"
        checked={values.numbering}
        onchange={(e) => set("elements.headings.numbering", e.currentTarget.checked)}
      />
      <span>Number headings</span>
    </label>
    <label class="row check">
      <input
        type="checkbox"
        checked={values.h1Rule}
        onchange={(e) => set("elements.headings.h1_rule", e.currentTarget.checked)}
      />
      <span>Rule under the title</span>
    </label>
  </section>

  <footer>
    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    <div class="actions">
      <button class="primary" disabled={saving || problem !== null} onclick={() => void save()}>Save</button>
      <button onclick={oncancel}>Cancel</button>
      <span class="hint">{changes.size === 0 ? "No changes" : `${changes.size} change${changes.size === 1 ? "" : "s"}`}</span>
    </div>
  </footer>
</aside>

<style>
  .sk-theme-editor {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 2.2rem;
    width: min(23rem, 92vw);
    display: flex;
    flex-direction: column;
    overflow-x: hidden;
    overflow-y: auto;
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
    border-left: 1px solid var(--sk-color-border);
    box-shadow: -8px 0 24px color-mix(in srgb, var(--sk-color-foreground) 12%, transparent);
    font-family: var(--sk-font-ui);
    font-size: 0.82rem;
    z-index: 9;
  }

  header,
  section,
  footer {
    padding: 0.6rem 1rem;
  }

  section {
    border-top: 1px solid var(--sk-color-border);
  }

  footer {
    position: sticky;
    bottom: 0;
    margin-top: auto;
    background: var(--sk-color-ui-background);
    border-top: 1px solid var(--sk-color-border);
  }

  h2,
  h3 {
    font-family: var(--sk-font-ui);
    margin: 0 0 0.4rem;
  }

  h2 {
    font-size: 1rem;
  }

  h3 {
    font-size: 0.85rem;
  }

  .hint {
    margin: 0.2rem 0 0.4rem;
    color: var(--sk-color-muted);
    font-size: 0.75rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0.3rem 0;
  }

  .row > span:first-child,
  .row > label:first-child {
    flex: 0 0 7.5rem;
  }

  .row.check > span {
    flex: 1;
  }

  .unit {
    color: var(--sk-color-muted);
  }

  fieldset {
    /* Fieldsets are otherwise at least as wide as their contents. */
    min-width: 0;
    border: 1px solid var(--sk-color-border);
    border-radius: 6px;
    margin: 0.4rem 0;
    padding: 0.2rem 0.6rem 0.4rem;
  }

  legend {
    padding: 0 0.3rem;
    font-weight: 600;
  }

  input[type="text"],
  input[type="number"],
  select {
    flex: 1;
    min-width: 0;
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
  }

  input[type="number"] {
    flex: 0 0 5rem;
  }

  input[type="color"] {
    flex: 0 0 2rem;
    height: 1.6rem;
    padding: 0;
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    background: none;
  }

  input[type="checkbox"] {
    accent-color: var(--sk-color-accent);
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.25rem 0.7rem;
    cursor: pointer;
  }

  /* In the text color: the accent may be the very color being edited. */
  button.primary {
    border-color: var(--sk-color-accent);
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  button.x {
    padding: 0 0.4rem;
    border-color: transparent;
    background: none;
    color: var(--sk-color-muted);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .problem {
    margin: 0 0 0.4rem;
    color: var(--sk-palette-red, var(--sk-color-accent));
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
