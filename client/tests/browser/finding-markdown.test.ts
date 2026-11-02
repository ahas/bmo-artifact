import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium } from "playwright-core";

import type { Browser, Page } from "playwright-core";

// What the finding reading lets reach a page. The cleaning is DOMPurify's, which needs a real
// document, so it is checked in a real browser with the reading built as the bundle ships it —
// skipped where the browser build is not on the machine.

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();
const built = ready ? await Bun.build({ entrypoints: [new URL("../../app/domain/finding-markdown.ts", import.meta.url).pathname], target: "browser" }) : null;
const script = built === null ? "" : await built.outputs[0]!.text();

const HOSTILE = [
  "[press](javascript:alert(1))",
  '<a href="javascript:alert(1)">link</a>',
  "<script>window.ran = true</script>",
  '<img src=x onerror="window.ran = true">',
  '<p onclick="window.ran = true" style="color:red">paragraph</p>',
  "[outside](https://example.org)"
].join("\n\n");

let browser: Browser;
let page: Page;

// The finding drawn into the page, with every picture answered by a copy of the console's, and what
// stands of it read back.
async function drawn(markdown: string, slugs: string[] = []): Promise<void> {
  await page.evaluate(
    ([text, marks]) => {
      const scope = window as unknown as { draw: (text: string, slugs: string[]) => string };

      document.getElementById("out")!.innerHTML = scope.draw(text, marks);
    },
    [markdown, slugs] as const
  );
}

describe.skipIf(!ready)("what a finding lets reach the page", () => {
  beforeAll(async () => {
    browser = await chromium.launch();
    page = await browser.newPage();
    await page.setContent("<div id=out></div>");
    await page.addScriptTag({
      type: "module",
      content: `${script}\nwindow.draw = (text, slugs) => renderFinding((key) => key, text, { images: (href) => "/copy?of=" + encodeURIComponent(href), slugs });`
    });
    await page.waitForFunction(() => "draw" in window);
  }, 60_000);

  afterAll(async () => {
    await browser?.close();
  });

  test("runs nothing and links nowhere a script is", async () => {
    await page.evaluate(() => {
      (window as unknown as { ran: boolean }).ran = false;
    });
    await drawn(HOSTILE);
    await page.waitForTimeout(200);

    const seen = await page.evaluate(() => {
      const out = document.getElementById("out")!;
      const links = [...out.querySelectorAll("a")];

      return {
        ran: (window as unknown as { ran: boolean }).ran,
        scripts: out.querySelectorAll("script").length,
        handlers: [...out.querySelectorAll("*")].flatMap((element) => element.getAttributeNames().filter((name) => name.startsWith("on"))),
        hrefs: links.map((link) => link.getAttribute("href") ?? ""),
        targets: links.map((link) => `${link.getAttribute("target")} ${link.getAttribute("rel")}`)
      };
    });

    expect(seen.ran).toBe(false);
    expect(seen.scripts).toBe(0);
    expect(seen.handlers).toEqual([]);
    expect(seen.hrefs.some((href) => href.startsWith("javascript:"))).toBe(false);
    expect(seen.hrefs).toContain("https://example.org");
    expect(seen.targets).toContain("_blank noopener noreferrer");
  }, 30_000);

  // The places the bundle fills with its own components, the marks the contents steer by and the
  // footnotes the sources are brought forward by are all written as attributes, so each must come
  // through the cleaning as the reading wrote it.
  test("keeps the blocks' places, the headings' marks and the footnotes through the cleaning", async () => {
    await drawn(
      [
        "### The shop",
        "",
        "::: fold Menu",
        "| Dish | Price |",
        "| --- | --- |",
        "| Soup | 9,000 |",
        ":::",
        "",
        "::: gallery",
        "![soup](<https://cdn.example.com/p.jpg>)",
        "![front](<https://cdn.example.com/q.jpg>)",
        ":::",
        "",
        "::: map 1 Main Street",
        "",
        "a value[1](#mark-1) and a snippet[2†](#mark-2%E2%80%A0)"
      ].join("\n"),
      ["the-shop"]
    );

    const seen = await page.evaluate(() => {
      const out = document.getElementById("out")!;

      return {
        heading: [out.querySelector("h3")?.id, out.querySelector("h3")?.getAttribute("data-heading")],
        fold: out.querySelector(".finding-fold")?.getAttribute("data-fold-label"),
        foldBody: out.querySelector(".finding-fold .finding-fold-body table") !== null,
        slides: JSON.parse(out.querySelector(".finding-gallery")?.getAttribute("data-slides") ?? "[]").map((slide: { src: string }) => slide.src),
        map: out.querySelector(".finding-map")?.getAttribute("data-map-query"),
        marks: [...out.querySelectorAll("a[data-mark]")].map((link) => link.getAttribute("data-mark")),
        address: out.querySelector("img[data-picture-address]")?.getAttribute("data-picture-address")
      };
    });

    expect(seen).toEqual({
      heading: ["the-shop", "the-shop"],
      fold: "Menu",
      foldBody: true,
      slides: ["/copy?of=https%3A%2F%2Fcdn.example.com%2Fp.jpg", "/copy?of=https%3A%2F%2Fcdn.example.com%2Fq.jpg"],
      map: "1 Main Street",
      marks: ["1", "2†"],
      address: "https://cdn.example.com/p.jpg"
    });
  }, 30_000);

  // A bare mark and its source entry carry the places a press moves between, which must come through the
  // cleaning, and an entry's page still opens in a tab of its own while its mark stays in the text.
  test("keeps a bare mark's place and its source entry's through the cleaning", async () => {
    await drawn("a weight[1] and a snippet[2†]\n\n- [1] [The **first** page](https://one.example/a)\n- [2†] [Second](https://two.example)");

    const seen = await page.evaluate(() => {
      const out = document.getElementById("out")!;

      return {
        places: [...out.querySelectorAll("[id]")].map((element) => element.id),
        sources: [...out.querySelectorAll("li[data-source]")].map((entry) => entry.getAttribute("data-source")),
        back: [...out.querySelectorAll("a[data-mark-back]")].map((link) => [link.getAttribute("href"), link.getAttribute("target")]),
        page: [...out.querySelectorAll("li a[href^='https']")].map((link) => link.getAttribute("target"))
      };
    });

    expect(seen).toEqual({
      places: ["cite-1", "cite-2†", "mark-1", "mark-2†"],
      sources: ["1", "2†"],
      back: [
        ["#cite-1", null],
        ["#cite-2%E2%80%A0", null]
      ],
      page: ["_blank", "_blank"]
    });
  }, 30_000);

  test("keeps the column count a table's box is laid out by through the cleaning", async () => {
    await drawn("| A | B |\n|---|---|\n| 1 | 2 |");

    const columns = await page.evaluate(() => (document.querySelector("#out .markdown-table") as HTMLElement | null)?.style.getPropertyValue("--columns").trim() ?? null);

    expect(columns).toBe("2");
  }, 30_000);

  test("keeps a fenced block's copy button and its two marks through the cleaning", async () => {
    await drawn("```\nbun install\n```");

    const button = await page.evaluate(() => {
      const found = document.querySelector("#out .markdown-code > button[data-copy]");

      return found === null ? null : { type: found.getAttribute("type"), label: found.getAttribute("aria-label"), tooltip: found.getAttribute("data-tooltip"), marks: found.querySelectorAll("svg path").length };
    });

    expect(button).toEqual({ type: "button", label: "common.action.copy", tooltip: "common.action.copy", marks: 2 });
  }, 30_000);
});
