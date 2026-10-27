import { describe, expect, it } from "vitest";
import { ScrollKeeper, type Placed } from "./anchoring";

/** A scroller whose content is `height` px tall. */
function fakeScroller(scrollTop: number) {
  return { scrollTop, height: 10_000 };
}

/** An element whose on-screen top follows the scroll position, like a real one. */
function fakeElement(scroller: { scrollTop: number }, offsetTop: number) {
  const el = {
    offsetTop,
    isConnected: true,
    getBoundingClientRect: () => ({ top: el.offsetTop - scroller.scrollTop }),
  };
  return el;
}

function keeperFor(scroller: ReturnType<typeof fakeScroller>, element: Placed | null) {
  return new ScrollKeeper(
    scroller,
    () => element,
    () => scroller.height,
  );
}

describe("ScrollKeeper", () => {
  it("scrolls by what was added above the anchor", () => {
    const scroller = fakeScroller(1000);
    const block = fakeElement(scroller, 1020);
    const keeper = keeperFor(scroller, block);
    keeper.record();
    block.offsetTop += 300; // a section above mounted
    scroller.height += 300;
    keeper.correct();
    expect(scroller.scrollTop).toBe(1300);
    expect(block.getBoundingClientRect().top).toBe(20);
  });

  it("scrolls back when content above shrinks", () => {
    const scroller = fakeScroller(1000);
    const block = fakeElement(scroller, 1020);
    const keeper = keeperFor(scroller, block);
    keeper.record();
    block.offsetTop -= 120;
    scroller.height -= 120;
    keeper.correct();
    expect(scroller.scrollTop).toBe(880);
  });

  it("keeps a scroll made before the correction", () => {
    const scroller = fakeScroller(1000);
    const block = fakeElement(scroller, 1020);
    const keeper = keeperFor(scroller, block);
    keeper.record();
    block.offsetTop += 300;
    scroller.height += 300;
    scroller.scrollTop += 50; // the reader scrolled in the same frame
    keeper.record(); // the scroll handler, before the resize callback
    keeper.correct();
    expect(scroller.scrollTop).toBe(1350);
  });

  it("takes a jump as the new place, even with a correction due", () => {
    const scroller = fakeScroller(1000);
    const block = fakeElement(scroller, 1020);
    const keeper = keeperFor(scroller, block);
    keeper.record();
    block.offsetTop += 300; // zoom: the text grew
    scroller.height += 300;
    scroller.scrollTop = 1200; // and the reader put its own position back
    keeper.reset();
    keeper.correct();
    expect(scroller.scrollTop).toBe(1200);
  });

  it("does nothing at the top, for a lost anchor, or for a sub-pixel move", () => {
    const scroller = fakeScroller(0);
    const block = fakeElement(scroller, 40);
    const keeper = keeperFor(scroller, block);
    keeper.record();
    block.offsetTop += 200;
    keeper.correct();
    expect(scroller.scrollTop).toBe(0);

    scroller.scrollTop = 500;
    keeper.record();
    block.offsetTop += 0.3;
    keeper.correct();
    expect(scroller.scrollTop).toBe(500);

    block.isConnected = false;
    block.offsetTop += 100;
    keeper.correct();
    expect(scroller.scrollTop).toBe(500);
  });

  it("does nothing without an anchor", () => {
    const scroller = fakeScroller(500);
    const keeper = keeperFor(scroller, null);
    keeper.record();
    keeper.correct();
    expect(scroller.scrollTop).toBe(500);
  });
});
