import { describe, expect, test } from "bun:test";

import { addressText, backMarkAt, markAt, sourceOf } from "../../app/domain/footnotes";

// How a footnote mark answers a reader, read off the text as the finding reading draws it and the cleaning
// leaves it (`docs/artifact.md`, "How a document is drawn").

function drawn(html: string): HTMLElement {
  const root = document.createElement("div");

  root.innerHTML = html;
  document.body.replaceChildren(root);

  return root;
}

const TEXT = [
  '<p>a weight<sup class="search-mark"><a href="#mark-1" data-mark="1" id="cite-1">[1]</a></sup> and a price',
  '<sup class="search-mark search-mark-snippet"><a href="#mark-10%E2%80%A0" data-mark="10†" id="cite-10†">[10†]</a></sup>',
  ' and an orphan<sup class="search-mark"><a href="#mark-9" data-mark="9">9</a></sup></p>',
  "<ul>",
  '<li id="mark-1" data-source="1"><a href="#cite-1" class="search-mark-back" data-mark-back="1">[1]</a> ',
  '<a href="https://one.example/a%20b" target="_blank" rel="noopener noreferrer">The <strong>first</strong> page</a></li>',
  '<li id="mark-10†" data-source="10†"><a href="#cite-10%E2%80%A0" class="search-mark-back" data-mark-back="10†">[10†]</a> ',
  '<a href="https://ten.example/" target="_blank" rel="noopener noreferrer">A snippet</a></li>',
  "</ul>"
].join("");

describe("a footnote mark", () => {
  test("is found under a press on it, and an entry's mark apart from it", () => {
    const root = drawn(TEXT);
    const mark = root.querySelector("#cite-1")!;
    const back = root.querySelector("[data-mark-back='1']")!;

    expect(markAt(mark, root)).toBe(mark as HTMLAnchorElement);
    expect(markAt(mark.parentElement, root)).toBeNull();
    expect(markAt(back, root)).toBeNull();
    expect(backMarkAt(back, root)).toBe(back as HTMLAnchorElement);
    expect(markAt(root.querySelector("p"), root)).toBeNull();
  });

  test("brings forward its entry's words, formatted, with the page's address apart as a link of its own", () => {
    const source = sourceOf(drawn(TEXT), "1");

    expect(source).toEqual({ mark: "[1]", html: "The <strong>first</strong> page", addresses: ["https://one.example/a%20b"], snippet: false });
  });

  test("says a snippet's value was checked against the snippet only", () => {
    expect(sourceOf(drawn(TEXT), "10†")?.snippet).toBe(true);
  });

  test("brings nothing forward where the text carries no entry for it", () => {
    expect(sourceOf(drawn(TEXT), "9")).toBeNull();
  });

  test("names the page by its address as a reader reads it", () => {
    expect(addressText("https://one.example/a%20b")).toBe("one.example/a b");
    expect(addressText("http://x.example/%E0%A4%A")).toBe("x.example/%E0%A4%A");
  });
});
