import { afterAll, beforeAll, beforeEach, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium } from "playwright-core";

import type { Browser, Page } from "playwright-core";

// A footnote mark's jump and the back gesture that takes it back (docs/artifact.md, "How a document is drawn"), in a
// real browser: the jumps module built as the bundle ships it, with bmo-ui's layers under it, over a text that scrolls
// in a box of its own on a page at an address of its own, so the history it pushes onto is the browser's. Skipped
// where the browser build is not on the machine.

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();
const built = ready ? await Bun.build({ entrypoints: [new URL("../../app/domain/footnote-jumps.ts", import.meta.url).pathname], target: "browser" }) : null;
const script = built === null ? "" : await built.outputs[0]!.text();

const SITE = "https://text.test";
const filler = (height: number) => `<div style="height:${height}px"></div>`;

// The text: a mark well down it, its entry far below, and the entry's own mark leading back. The layers module asks
// the console's router for its history; a host with no router of its own answers none, as this page does.
const TEXT = `<!doctype html><html><body style="margin:0">
  <div id="paper" style="height:300px;overflow-y:auto">
    ${filler(600)}<p><a id="cite-1" href="#mark-1" data-mark="1">[1]</a></p>
    ${filler(3000)}<ol><li id="mark-1"><a href="#cite-1" data-mark-back="1">[1]</a> the source</li></ol>
    ${filler(600)}
  </div>
  <script>window.useRouter = () => ({ options: {}, beforeEach() {} });</script>
  <script type="module">${script.replaceAll("</script", "<\\/script")}
    window.jumps = footnoteJumps();</script>
</body></html>`;

let browser: Browser;
let page: Page;

async function scrolled(top: number): Promise<void> {
  await page.evaluate((at) => {
    document.getElementById("paper")!.scrollTop = at;
  }, top);
}

async function jump(to: string, from: string): Promise<number> {
  return page.evaluate(
    ([place, mark]) => {
      (window as unknown as { jumps: { jump: (place: HTMLElement, from: HTMLElement) => void } }).jumps.jump(document.getElementById(place)!, document.getElementById(mark)!);

      return document.getElementById("paper")!.scrollTop;
    },
    [to, from] as const
  );
}

// A step back, landed once the text stands at `top` on the document's own address.
async function backTo(top: number): Promise<void> {
  await page.evaluate(() => history.back());
  await page.waitForFunction((at) => document.getElementById("paper")!.scrollTop === at && location.pathname === "/doc", top);
}

// A step back that leaves the document for the page before it.
async function backLeaves(): Promise<void> {
  await page.evaluate(() => history.back());
  await page.waitForURL(`${SITE}/list`);
}

describe.skipIf(!ready)("a footnote jump and the back gesture", () => {
  beforeAll(async () => {
    browser = await chromium.launch();
    page = await browser.newPage();
    page.setDefaultTimeout(5_000);
    await page.route(`${SITE}/**`, (route) =>
      route.fulfill({ contentType: "text/html", body: new URL(route.request().url()).pathname === "/doc" ? TEXT : "<!doctype html><p>list</p>" })
    );
  }, 60_000);

  afterAll(async () => {
    await browser?.close();
  });

  // The list, then the document opened from it, so a step back past the document reaches the list.
  beforeEach(async () => {
    await page.goto(`${SITE}/list`);
    await page.goto(`${SITE}/doc`);
    await page.waitForFunction(() => "jumps" in window);
    await scrolled(450);
  });

  test("takes one jump back to where the text stood, then leaves the document", async () => {
    const landed = await jump("mark-1", "cite-1");
    const seen = await page.evaluate(() => ({ marked: document.getElementById("mark-1")!.hasAttribute("data-mark-found"), href: location.href }));

    expect(landed).toBeGreaterThan(3000);
    expect(seen).toEqual({ marked: true, href: `${SITE}/doc` });

    await backTo(450);
    await backLeaves();
  }, 30_000);

  test("takes two jumps back one press each, the later first", async () => {
    const atEntry = await jump("mark-1", "cite-1");

    await scrolled(atEntry - 40);
    expect(await jump("cite-1", "mark-1")).toBeLessThan(1000);

    await backTo(atEntry - 40);
    await backTo(450);
    await backLeaves();
  }, 30_000);

  // The router moves the address before the text leaves, so the step the text gives up is behind the new page, and
  // a press arriving at it is carried on to the document's own place.
  test("leaves a jump given up by a navigation for one press to step over", async () => {
    await jump("mark-1", "cite-1");
    await page.evaluate(() => {
      history.pushState(null, "", "/other");
      (window as unknown as { jumps: { giveUp: () => void } }).jumps.giveUp();
    });

    await page.evaluate(() => history.back());
    await page.waitForFunction(() => location.pathname === "/doc" && history.state === null);
    await backLeaves();
  }, 30_000);
});
