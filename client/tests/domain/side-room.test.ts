import { afterEach, describe, expect, test } from "bun:test";

import { scrollingAncestor, sideRoom } from "../../app/domain/side-room";

// The room a column held in view has (docs/artifact.md, 「The front page」): the region that scrolls it found among
// its ancestors, and its visible height less its padding above and below. What a real browser draws is held by the
// front page's browser test; this holds the reading.

afterEach(() => {
  document.body.innerHTML = "";
});

function region(): { scroller: HTMLElement; column: HTMLElement } {
  document.body.innerHTML = `<div id="scroller" style="overflow-y: auto; padding: 24px 0 16px"><div><aside id="column"></aside></div></div>`;

  return { scroller: document.getElementById("scroller")!, column: document.getElementById("column")! };
}

describe("the side column's room", () => {
  test("the region scrolling the column is its nearest ancestor that scrolls up and down", () => {
    const { scroller, column } = region();

    expect(scrollingAncestor(column)).toBe(scroller);
  });

  test("a column no region scrolls has the window's", () => {
    document.body.innerHTML = `<div><aside id="column"></aside></div>`;

    expect(scrollingAncestor(document.getElementById("column")!)).toBeNull();
    expect(sideRoom(null)).toBe(window.innerHeight);
  });

  test("the room is the region's visible height less its padding above and below", () => {
    const { scroller } = region();

    Object.defineProperty(scroller, "clientHeight", { value: 600 });

    expect(sideRoom(scroller)).toBe(560);
  });
});
