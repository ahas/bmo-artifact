import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium } from "playwright-core";

import design from "../../design/document.json";
import hosting from "../../design/hosted-page.json";
import { HOSTED_PAGE_SANDBOX } from "../../shared/config";

import type { Browser, Page } from "playwright-core";

// A page's pictures reach the console's viewer through the bridge the server writes ahead of the page,
// and a markdown document's through the console reading its own body: both by one rule, checked here
// in a real browser where the page stands in a frame sandboxed as the console's is. The bridges and
// the policy are the design files the server reads, put together as the server puts them.

// Whether the browser this version of playwright pins is on the machine. Everything here is skipped
// where it is not.
function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();
const built = ready ? await Bun.build({ entrypoints: [new URL("../../app/domain/document-pictures.ts", import.meta.url).pathname], target: "browser" }) : null;
const script = built === null ? "" : await built.outputs[0]!.text();

const bridge = async (name: string): Promise<string> => (await Bun.file(new URL(`../../design/bridges/${name}.js`, import.meta.url)).text()).trimEnd();
const PICTURE_BRIDGE = `<script>${await bridge("picture")}(${JSON.stringify(design.viewer).replaceAll("<", "\\u003c")})</script>`;
const LINK_BRIDGE = `<script>${await bridge("link")}</script>`;
const POLICY = [`sandbox ${hosting.sandbox}`, ...hosting.policy].join("; ");

// Pictures written into the page as data, so nothing is fetched.
const dot = (colour: string) =>
  `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="40" height="40" fill="${colour}"/></svg>`)}`;

const BODY = `<p><img id="first" src="${dot("red")}" alt="first picture"></p>
  <figure><img id="second" src="${dot("green")}" alt=""><figcaption>second caption</figcaption></figure>
  <a href="#first"><img id="linked" src="${dot("blue")}" alt="linked picture"></a>
  <div data-archive-no-viewer><img id="own" src="${dot("black")}" alt="its own picture"></div>
  <img id="handled" src="${dot("gray")}" alt="a picture it handles">
  <img id="hidden" src="${dot("white")}" alt="hidden picture" style="display:none">
  <img id="third" src="${dot("yellow")}" alt="third picture">
  <a id="away" href="https://example.invalid/">away</a> <a id="within" href="#third">within</a> <a id="aimed" href="https://example.invalid/" target="side">aside</a>
  <img id="windowed" src="${dot("purple")}" alt="a picture the window handles">
  <script>
    document.getElementById("handled").addEventListener("click", (event) => event.preventDefault());
    addEventListener("click", (event) => { if (event.target.id === "windowed") event.preventDefault(); });
  </script>`;

// The page as it goes out, with the lines that act in a frame ahead of its own.
const PAGE = `<!doctype html>${PICTURE_BRIDGE}${LINK_BRIDGE}<html><head><title>Picture document</title></head><body>${BODY}</body></html>`;

// The console and the page it frames, each at an address of its own as the console serves them: the
// page under the policy every hosted page goes out with. Nothing else is answered, so a link pressed
// opens a tab that goes nowhere.
const CONSOLE = "https://console.test/";
const PAGE_ADDRESS = `${CONSOLE}page`;

const FRAMING = `<iframe id="page" src="${PAGE_ADDRESS}" sandbox="${HOSTED_PAGE_SANDBOX}" style="width:600px;height:700px"></iframe>
  <script>
    window.received = [];
    addEventListener("message", (event) => { if (event.source === document.getElementById("page").contentWindow) window.received.push(event.data); });
  </script>`;

let browser: Browser;
let page: Page;

async function framed(): Promise<void> {
  await page.goto(CONSOLE);
  await page.frameLocator("#page").locator("#third").waitFor();
}

async function received(): Promise<unknown[]> {
  return page.evaluate(() => (window as unknown as { received: unknown[] }).received);
}

async function pressInFrame(id: string): Promise<void> {
  await page.frameLocator("#page").locator(`#${id}`).click();
}

describe.skipIf(!ready)("pictures opening whole", () => {
  beforeAll(async () => {
    browser = await chromium.launch();
    page = await browser.newPage({ viewport: { width: 800, height: 800 } });
    await page.context().route(/^https?:/, (route) => {
      const address = route.request().url();

      if (address === CONSOLE) return route.fulfill({ contentType: "text/html; charset=utf-8", body: FRAMING });
      if (address === PAGE_ADDRESS) return route.fulfill({ contentType: "text/html; charset=utf-8", headers: { "content-security-policy": POLICY }, body: PAGE });

      return route.abort();
    });
  });

  afterAll(async () => {
    await browser?.close();
  });

  test("a page in the frame hands over every picture it lets open, in order, with the pressed one", async () => {
    await framed();
    await pressInFrame("second");
    await page.waitForFunction(() => (window as unknown as { received: unknown[] }).received.length > 0);

    expect(await received()).toEqual([
      {
        type: "bmo-pictures",
        at: 1,
        pictures: [
          { src: dot("red"), caption: "first picture" },
          { src: dot("green"), caption: "second caption" },
          { src: dot("gray"), caption: "a picture it handles" },
          { src: dot("yellow"), caption: "third picture" },
          { src: dot("purple"), caption: "a picture the window handles" }
        ]
      }
    ]);
    expect(await page.frameLocator("#page").locator("#first").evaluate((image) => getComputedStyle(image).cursor)).toBe("zoom-in");
  });

  test("a picture the page gives a press of its own keeps it", async () => {
    await framed();

    for (const id of ["linked", "own", "handled", "windowed"]) {
      await page.frameLocator("#page").locator(`#${id}`).click();
    }

    await pressInFrame("third");
    await page.waitForFunction(() => (window as unknown as { received: unknown[] }).received.length > 0);

    expect((await received()).map((message) => (message as { at: number }).at)).toEqual([3]);
    expect(await page.frameLocator("#page").locator("#linked").evaluate((image) => getComputedStyle(image).cursor)).not.toBe("zoom-in");
  });

  test("in a frame, a link that would open in the page's own window opens in a tab of its own", async () => {
    await framed();

    const inFrame = page.frameLocator("#page");

    for (const id of ["away", "within", "aimed"]) await inFrame.locator(`#${id}`).click();

    expect(await Promise.all(["away", "within", "aimed"].map((id) => inFrame.locator(`#${id}`).getAttribute("target")))).toEqual(["_blank", null, "side"]);
  });

  test("a page standing alone is left as it is", async () => {
    await page.goto(PAGE_ADDRESS);
    await page.locator("#first").click();

    expect(await page.locator("#first").evaluate((image) => getComputedStyle(image).cursor)).not.toBe("zoom-in");
  });

  test("the console reads a markdown body by the same rule", async () => {
    await page.setContent(`<div id="body">${BODY.replace(/<script>[\s\S]*<\/script>/, "")}</div>`);
    await page.addScriptTag({ type: "module", content: `${script}\nwindow.pressedPictures = pressedPictures;` });
    await page.waitForFunction(() => "pressedPictures" in window);

    const run = (id: string) =>
      page.evaluate((target) => {
        const read = (window as unknown as { pressedPictures: (root: ParentNode, target: EventTarget | null) => unknown }).pressedPictures;

        return read(document.getElementById("body")!, document.getElementById(target));
      }, id);

    expect(await run("third")).toEqual({
      at: 3,
      pictures: [
        { src: dot("red"), caption: "first picture", address: null },
        { src: dot("green"), caption: "second caption", address: null },
        { src: dot("gray"), caption: "a picture it handles", address: null },
        { src: dot("yellow"), caption: "third picture", address: null },
        { src: dot("purple"), caption: "a picture the window handles", address: null }
      ]
    });
    expect(await run("linked")).toBeNull();
    expect(await run("own")).toBeNull();
  });
});
