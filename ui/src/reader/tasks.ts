// Task lists in the reader (#218): clicking a checkbox checks or unchecks
// its item, and a per-document toggle hides the checked ones. Core gives
// each block its tasks (`Block.tasks`) in the order its checkboxes appear in
// the HTML, so the nth checkbox is the nth task. When the counts differ (raw
// HTML checkboxes, a note shown in place), the block's boxes stay read-only,
// so a click can never change the wrong item.
import type { Block, TaskMark } from "../lib/types";

/** Checked and total task items in the document. */
export function taskCounts(blocks: readonly Block[]): { done: number; total: number } {
  let done = 0;
  let total = 0;
  for (const block of blocks) {
    for (const task of block.tasks ?? []) {
      total += 1;
      if (task.checked) done += 1;
    }
  }
  return { done, total };
}

/** The status bar's summary, such as "3 / 12 done". */
export function formatTasks(done: number, total: number): string {
  return `${done} / ${total} done`;
}

/** A block's checkboxes, in document order, which core's tasks follow. */
export function checkboxesIn(el: Element): HTMLInputElement[] {
  return Array.from(el.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'));
}

/** A task item's own text, without its nested lists, for the checkbox's label. */
export function itemLabel(box: HTMLInputElement): string {
  const item = box.closest("li");
  if (!item) return "Task";
  let text = "";
  for (const node of Array.from(item.childNodes)) {
    if (node instanceof Element && (node.tagName === "UL" || node.tagName === "OL")) continue;
    text += node.textContent ?? "";
  }
  const label = text.replace(/\s+/g, " ").trim();
  return label === "" ? "Task" : label;
}

/**
 * Makes a block's checkboxes clickable when they match its tasks one to one:
 * each gets `data-task` (its index in `tasks`) and a label, and loses
 * `disabled`. Returns whether they matched.
 */
export function enableCheckboxes(el: Element, tasks: readonly TaskMark[]): boolean {
  const boxes = checkboxesIn(el);
  if (tasks.length === 0 || boxes.length !== tasks.length) return false;
  boxes.forEach((box, i) => {
    box.disabled = false;
    box.dataset.task = String(i);
    box.setAttribute("aria-label", itemLabel(box));
  });
  return true;
}

/**
 * The items that hide `node` when "Hide checked items" is on: each item
 * around it whose own checkbox is checked, innermost first.
 */
export function hidingItems(node: Node): HTMLLIElement[] {
  const items: HTMLLIElement[] = [];
  for (let el = node instanceof Element ? node : node.parentElement; el; el = el.parentElement) {
    if (el instanceof HTMLLIElement && el.querySelector(':scope > input[type="checkbox"]:checked')) items.push(el);
  }
  return items;
}
