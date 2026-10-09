// Rich content rendered in the frontend (DESIGN.md §4): math (KaTeX),
// diagrams (Mermaid), and music (abcjs). Each library is loaded with a
// dynamic import() only when the document uses it, and a block is rendered
// only when it comes near the viewport.
//
// These libraries build their own markup (KaTeX HTML, SVG) from the source
// text in the block; that output is the one exception to "only insert core's
// sanitized HTML" (AGENTS.md). Mermaid runs with securityLevel "strict" and
// KaTeX with trust: false, so documents can't inject links or scripts
// through them.

import { parseAbcBlock } from "./abc-options";

/** Selectors core's HTML uses for each kind of rich content. */
const MATH = "[data-math-style]:not([data-sk-rendered])";
const MERMAID = "pre > code.language-mermaid";
const ABC = "pre > code.language-abc";

/** Whether a block's HTML contains anything to render here (cheap string test). */
export function needsRichRendering(html: string): boolean {
  return html.includes("data-math-style") || html.includes("language-mermaid") || html.includes("language-abc");
}

// Render a block once it is within a screen's height of the viewport.
let observer: IntersectionObserver | null = null;

function getObserver(): IntersectionObserver {
  observer ??= new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer?.unobserve(entry.target);
        void renderRich(entry.target as HTMLElement);
      }
    },
    { rootMargin: "100% 0px" },
  );
  return observer;
}

/**
 * Svelte action: render a block's rich content as it nears the viewport.
 * Takes the block's HTML so it runs again when that changes: on a live
 * reload Svelte keeps the element (same block id) and swaps in fresh,
 * unrendered HTML, which must be rendered again.
 */
export function richContent(node: HTMLElement, html: string) {
  const watch = (current: string) => {
    if (needsRichRendering(current)) getObserver().observe(node);
  };
  watch(html);
  return {
    update: watch,
    destroy() {
      observer?.unobserve(node);
    },
  };
}

/** Renders every kind of rich content inside `root`. */
export async function renderRich(root: HTMLElement): Promise<void> {
  const tasks: Promise<void>[] = [];
  if (root.querySelector(MATH)) tasks.push(renderMath(root));
  if (root.querySelector(MERMAID)) tasks.push(renderMermaid(root));
  if (root.querySelector(ABC)) tasks.push(renderAbc(root));
  await Promise.all(tasks);
}

/** Redraws diagrams and music in the current theme's colors (after a theme change). */
export async function refreshRichForTheme(root: ParentNode = document): Promise<void> {
  const diagrams = Array.from(root.querySelectorAll<HTMLElement>(".sk-mermaid[data-sk-source]"));
  const scores = Array.from(root.querySelectorAll<HTMLElement>(".sk-abc[data-sk-source]"));
  if (diagrams.length > 0) {
    const mermaid = await loadMermaid(true);
    for (const el of diagrams) await drawMermaid(mermaid, el, el.dataset.skSource ?? "");
  }
  if (scores.length > 0) {
    const abcjs = await import("abcjs");
    for (const el of scores) drawAbc(abcjs, el, el.dataset.skSource ?? "");
  }
}

/** A theme color as the browser resolved it. */
function themeColor(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function showError(target: HTMLElement, message: string): void {
  const note = document.createElement("p");
  note.className = "sk-render-error";
  note.textContent = message;
  target.prepend(note);
}

// --- Math --------------------------------------------------------------------

let katexCss: Promise<unknown> | null = null;

async function renderMath(root: HTMLElement): Promise<void> {
  katexCss ??= import("katex/dist/katex.min.css");
  const [{ default: katex }] = await Promise.all([import("katex"), katexCss]);
  for (const el of Array.from(root.querySelectorAll<HTMLElement>(MATH))) {
    const display = el.dataset.mathStyle === "display";
    const tex = el.textContent ?? "";
    // ```math blocks arrive as <pre><code>; render them into a plain div.
    let target: HTMLElement = el;
    const pre = el.parentElement;
    if (el.tagName === "CODE" && pre?.tagName === "PRE") {
      target = document.createElement("div");
      target.className = "sk-math-display";
      pre.replaceWith(target);
    }
    target.dataset.skRendered = "math";
    katex.render(tex, target, { displayMode: display, throwOnError: false, trust: false, strict: "ignore" });
  }
}

// --- Mermaid -------------------------------------------------------------------

type Mermaid = typeof import("mermaid").default;
let mermaidLoaded: Promise<Mermaid> | null = null;
let diagramCount = 0;

/** Loads Mermaid once; (re)initializes it with the theme's colors when asked. */
async function loadMermaid(reinitialize = false): Promise<Mermaid> {
  const fresh = mermaidLoaded === null;
  mermaidLoaded ??= import("mermaid").then((m) => m.default);
  const mermaid = await mermaidLoaded;
  if (fresh || reinitialize) {
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: "base",
      themeVariables: {
        background: themeColor("--sk-color-background", "#ffffff"),
        primaryColor: themeColor("--sk-color-code-bg", "#f4f4f4"),
        primaryTextColor: themeColor("--sk-color-foreground", "#222222"),
        primaryBorderColor: themeColor("--sk-color-border", "#cccccc"),
        lineColor: themeColor("--sk-color-muted", "#777777"),
        secondaryColor: themeColor("--sk-color-selection", "#eeeeee"),
        tertiaryColor: themeColor("--sk-color-ui-background", "#fafafa"),
        fontFamily: themeColor("--sk-font-ui", "sans-serif"),
      },
    });
  }
  return mermaid;
}

async function renderMermaid(root: HTMLElement): Promise<void> {
  const mermaid = await loadMermaid();
  for (const code of Array.from(root.querySelectorAll<HTMLElement>(MERMAID))) {
    const source = code.textContent ?? "";
    const container = document.createElement("div");
    container.className = "sk-mermaid";
    container.dataset.skSource = source;
    code.parentElement?.replaceWith(container);
    await drawMermaid(mermaid, container, source);
  }
}

async function drawMermaid(mermaid: Mermaid, container: HTMLElement, source: string): Promise<void> {
  try {
    const { svg } = await mermaid.render(`sk-mermaid-${++diagramCount}`, source);
    // Mermaid's own SVG output, sanitized by Mermaid in strict mode.
    container.innerHTML = svg;
  } catch (e) {
    container.textContent = source;
    container.classList.add("sk-render-failed");
    showError(container, `Diagram error: ${(e as Error).message ?? String(e)}`);
  }
}

// --- ABC music notation ---------------------------------------------------------

type Abcjs = typeof import("abcjs");

async function renderAbc(root: HTMLElement): Promise<void> {
  const abcjs = await import("abcjs");
  for (const code of Array.from(root.querySelectorAll<HTMLElement>(ABC))) {
    const source = code.textContent ?? "";
    const container = document.createElement("div");
    container.className = "sk-abc";
    container.dataset.skSource = source;
    code.parentElement?.replaceWith(container);
    drawAbc(abcjs, container, source);
  }
}

function drawAbc(abcjs: Abcjs, container: HTMLElement, source: string): void {
  const { tune, options, error } = parseAbcBlock(source);
  container.replaceChildren();
  const score = document.createElement("div");
  container.append(score);
  abcjs.renderAbc(score, tune, {
    responsive: "resize",
    foregroundColor: themeColor("--sk-color-foreground", "#000000"),
    ...options,
  });
  if (error) showError(container, error);
}
