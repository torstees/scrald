// Editing one item of a list or one child of a block quote (#118): core
// gives such a block `parts`, one source range per item or child, and the
// reader finds which one was double-clicked from the block's HTML.

/** What's needed of a DOM element here; `Element` has all of it. */
export interface PartTree {
  readonly tagName: string;
  readonly children: ArrayLike<PartTree>;
  readonly firstElementChild: PartTree | null;
  contains(other: Node | null): boolean;
}

/**
 * The elements a block's parts are drawn as, in order: the items of the
 * list at the top of the block, or the children of its block quote.
 */
export function partElements<T extends PartTree>(block: T): T[] {
  const top = block.firstElementChild as T | null;
  if (!top) return [];
  const children = Array.from(top.children) as T[];
  switch (top.tagName) {
    case "UL":
    case "OL":
      return children.filter((child) => child.tagName === "LI");
    case "BLOCKQUOTE":
      return children;
    default:
      return [];
  }
}

/**
 * The index of the part holding `target`, or null to edit the whole block:
 * when the block has no parts, when the HTML doesn't draw exactly one
 * element per part (so the two can't be matched safely), or when the click
 * fell between parts.
 */
export function partAt(elements: readonly PartTree[], count: number, target: Node | null): number | null {
  if (count === 0 || elements.length !== count) return null;
  const index = elements.findIndex((el) => el.contains(target));
  return index < 0 ? null : index;
}
