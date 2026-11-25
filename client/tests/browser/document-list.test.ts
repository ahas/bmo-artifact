import { afterAll, beforeAll, beforeEach, describe, expect, test } from "bun:test";

import { CLOSING_MS, browserReady, standScreen } from "./screen";

import type { Browser, Page, Route } from "playwright-core";
import type { Screen } from "./screen";

// The list of documents (docs/artifact.md, 「Where a person reads it」) read a page at a time, driven in a real
// Chromium at a wide window and a phone's: the next page brought by scrolling to the list's end alone, nothing at the
// list's foot to press and nothing under the last page, and a page that fails keeping the documents already read and
// read again from its retry. The screens stand alone on the host under `host/`, and the service's routes are
// answered by a stand-in holding forty-five documents. Skipped where the browser playwright-core pins is not on the
// machine.

const ready = browserReady();
const RELAY = "/relay/artifact";
const WIDE = { width: 1280, height: 900 };
const PHONE = { width: 360, height: 780 };
const HELD = 45;
const NOW = Date.now();

// The documents' numbers the stand-in holds, newest first.
const NUMBERS = Array.from({ length: HELD }, (_, at) => HELD - at);
// What the stand-in was asked for: each page's cursor, none for a first page.
let asked: (number | null)[] = [];
// Whether the next page asked past the first is refused, once.
let failNextPage = false;

function documentOf(id: number) {
  return {
    id,
    kind: "record",
    title: `Paged document ${id}`,
    scope: null,
    actorName: "tester",
    owner: null,
    labels: {},
    description: null,
    attachmentCount: 0,
    hasPicture: false,
    createdAt: NOW - id,
    changedAt: NOW - id
  };
}

function json(route: Route, body: unknown, status = 200): Promise<void> {
  return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function answer(route: Route): Promise<void> {
  const url = new URL(route.request().url());
  const path = url.pathname.slice(RELAY.length);

  if (path === "/document-kinds") return json(route, { items: [{ name: "record", description: "", hosted: false, contentsDepth: 2, icon: "description-outline" }], more: false });

  if (path === "/documents") {
    const before = url.searchParams.get("before");
    const limit = Number(url.searchParams.get("limit") ?? 30);
    const from = before === null ? 0 : NUMBERS.indexOf(Number(before)) + 1;

    asked.push(before === null ? null : Number(before));

    // A status the fetcher does not retry on its own, so the one failure reaches the screen.
    if (before !== null && failNextPage) {
      failNextPage = false;

      return json(route, { message: "unavailable" }, 400);
    }

    return json(route, { items: NUMBERS.slice(from, from + limit).map(documentOf), more: NUMBERS.length > from + limit });
  }

  return json(route, { items: [], more: false });
}

let screen: Screen | undefined;
let browser: Browser;
let page: Page;

const rows = () => page.locator("[data-documents-list] li");
const nextPages = () => asked.filter((before) => before !== null);

// The wheel turned over the list's first row, as a reader scrolling to the list's end does, until what is waited for
// holds.
async function scrollUntil(holds: () => Promise<boolean>, what: string): Promise<void> {
  const box = await rows().first().boundingBox();

  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);

  const end = Date.now() + 30_000;

  while (!(await holds())) {
    if (Date.now() > end) throw new Error(`${what} did not come by scrolling`);

    await page.mouse.wheel(0, 1200);
    await page.waitForTimeout(50);
  }
}

describe.skipIf(!ready)("the list of documents read a page at a time", () => {
  beforeAll(async () => {
    screen = await standScreen();
    browser = screen.browser;

    const context = await browser.newContext({ viewport: WIDE });

    await context.addCookies([{ name: "nuxt-color-mode", value: "light", url: screen.origin }]);
    page = await context.newPage();
    await page.route((url) => url.pathname.startsWith(`${RELAY}/`), answer);
  }, 300_000);

  afterAll(async () => {
    await screen?.close();
  }, CLOSING_MS);

  beforeEach(() => {
    asked = [];
    failNextPage = false;
  });

  for (const viewport of [WIDE, PHONE]) {
    test(`at ${viewport.width} wide, scrolling to the end reads the next page, and nothing is left to press`, async () => {
      await page.setViewportSize(viewport);
      await page.goto(`${screen!.origin}/artifact?view=list`, { waitUntil: "load", timeout: 120_000 });
      await page.getByText("Paged document 45", { exact: true }).waitFor({ timeout: 120_000 });
      expect(await rows().count()).toBe(30);
      expect(nextPages()).toEqual([]);
      expect(await page.locator("[data-next-page-foot] button").count()).toBe(0);

      await scrollUntil(async () => (await rows().count()) === HELD, "the second page");
      expect(nextPages()).toEqual([16]);
      // The last page holds nothing under it.
      await page.locator("[data-next-page-foot]").waitFor({ state: "detached" });
    }, 180_000);
  }

  test("a page that fails keeps the documents read and is read again from its retry", async () => {
    await page.setViewportSize(PHONE);
    failNextPage = true;
    await page.goto(`${screen!.origin}/artifact?view=list`, { waitUntil: "load", timeout: 120_000 });
    await page.getByText("Paged document 45", { exact: true }).waitFor({ timeout: 120_000 });

    const retry = page.locator("[data-next-page-foot]").getByRole("button", { name: "common.action.retry" });

    await scrollUntil(async () => (await retry.count()) > 0, "the failed page's retry");
    expect(await page.locator("[data-next-page-foot] [role=alert]").innerText()).toContain("unavailable");
    expect(await rows().count()).toBe(30);

    await retry.click();
    await page.getByText("Paged document 1", { exact: true }).waitFor();
    expect(await rows().count()).toBe(HELD);
  }, 180_000);
});
