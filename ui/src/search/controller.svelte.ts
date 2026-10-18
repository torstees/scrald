// In-document search state for one window (DESIGN.md §10): the query and
// options, the matches core found, and which one is current. Searches run
// one at a time, a moment after typing stops; the reader highlights them.
//
// This is a `.svelte.ts` module so its class fields can use runes and stay
// reactive in components that read them.

import { searchDocument } from "../lib/commands";
import type { SearchReply } from "../lib/types";
import { normalizeQuery, type SearchOptions } from "./matcher";
import { firstMatchFrom, matchLabel } from "./results";

/** What the controller needs from the reader. */
export interface SearchView {
  showSearch(search: { needle: string; options: SearchOptions; results: SearchReply } | null): void;
  /** Makes match `i` current; scrolls to it unless `scroll` is false. */
  revealMatch(i: number, scroll?: boolean): Promise<void>;
  topBlockId(): number;
}

/** How long typing must pause before searching. */
const SEARCH_DELAY_MS = 120;

/** How often to ask again while the app is busy with another search or model. */
const RETRY_MS = 50;
const RETRIES = 20;

export class SearchController {
  open = $state(false);
  query = $state("");
  matchCase = $state(false);
  wholeWord = $state(false);
  results = $state<SearchReply | null>(null);
  /** The current match's number, from 0; -1 for none. */
  current = $state(-1);
  readonly label = $derived(matchLabel(this.results, this.current, this.query));

  private timer = 0;
  private running = false;
  private again = false;
  /** Whether the next results should jump to the first match after the reading position. */
  private jump = false;

  constructor(private readonly view: () => SearchView | undefined) {}

  /** Opens the bar (with `text` as the query, if given) and searches. */
  show(text: string | null): void {
    if (text !== null) this.query = text;
    this.open = true;
    this.schedule(true, 0);
  }

  close(): void {
    this.open = false;
    window.clearTimeout(this.timer);
    this.results = null;
    this.current = -1;
    this.view()?.showSearch(null);
  }

  change(query: string, matchCase: boolean, wholeWord: boolean): void {
    this.query = query;
    this.matchCase = matchCase;
    this.wholeWord = wholeWord;
    this.schedule(true);
  }

  /**
   * The reader shows a new model: an edit (keep the current match, without
   * scrolling) or another document (start from its reading position).
   */
  documentChanged(otherDocument: boolean): void {
    if (this.open) this.schedule(otherDocument, otherDocument ? SEARCH_DELAY_MS : 0);
  }

  /** Steps to the next (1) or previous (-1) match, wrapping around. */
  step(by: 1 | -1): void {
    const total = this.results?.total ?? 0;
    if (total === 0) return;
    const from = this.current < 0 ? (by > 0 ? -1 : 0) : this.current;
    this.current = (from + by + total) % total;
    void this.view()?.revealMatch(this.current);
  }

  private schedule(jump: boolean, delay = SEARCH_DELAY_MS): void {
    this.jump ||= jump;
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => void this.run(), delay);
  }

  private async run(): Promise<void> {
    if (this.running) {
      this.again = true;
      return;
    }
    this.running = true;
    try {
      for (let attempt = 0; attempt < RETRIES && this.open; attempt++) {
        this.again = false;
        const options = { matchCase: this.matchCase, wholeWord: this.wholeWord };
        const query = this.query;
        const needle = normalizeQuery(query, options);
        if (needle === "") {
          this.results = null;
          this.current = -1;
          this.view()?.showSearch(null);
          return;
        }
        const reply = await searchDocument(query, options.matchCase, options.wholeWord);
        // The query changed meanwhile; `finally` searches again.
        if (this.again || !this.open) return;
        if (!reply) {
          await new Promise((resolve) => window.setTimeout(resolve, RETRY_MS));
          continue;
        }
        this.apply(reply, needle, options);
        return;
      }
    } catch {
      // No document, or it closed meanwhile: nothing to show.
    } finally {
      this.running = false;
      if (this.again) void this.run();
    }
  }

  private apply(reply: SearchReply, needle: string, options: SearchOptions): void {
    const view = this.view();
    this.results = reply;
    view?.showSearch({ needle, options, results: reply });
    if (reply.total === 0) {
      this.current = -1;
      this.jump = false;
      return;
    }
    const jump = this.jump || this.current < 0;
    this.jump = false;
    this.current = jump ? firstMatchFrom(reply, view?.topBlockId() ?? 0) : Math.min(this.current, reply.total - 1);
    void view?.revealMatch(this.current, jump);
  }
}
