import { afterAll, beforeAll, describe, expect, test } from "bun:test";

import words from "../../i18n/ko.json";
import { CLOSING_MS, browserReady, standScreen } from "./screen";

import type { Browser, Page, Route } from "playwright-core";
import type { Screen } from "./screen";

// The front page and the list (docs/artifact.md, 「The front page」 and 「Where a person reads it」) driven in a real
// Chromium at a wide window and a phone's: the newest led by a pictured card, a section for each kind holding one row
// of cards, the kinds, the recently read and the most read in a column beside them from the top, held in view as the
// page scrolls and never taller than what the page shows, its own content scrolling where it is taller, and on a
// phone one column of the kinds' chips, the newest, the recently and most read and the sections, nothing held; the search, a section's link, a kind and a chip turning the
// screen into the list and the way back to the front page; a card opening its document, which keeps a read and goes
// back where it came from, at the place it was left; and a share page keeping its browser's read. The screens stand alone on the host under `host/`, and the service's routes are answered by a stand-in here.
// Skipped where the browser playwright-core pins is not on the machine.

const ready = browserReady();
const RELAY = "/relay/artifact";
// The screens' words, in Korean as the host gives them.
const WORDS = words;
const KIND_WORDS: Record<string, string> = words.artifact.kind;

const WIDE = { width: 1280, height: 900 };
// A wide window too short for the side column, which then scrolls its own content.
const SHORT = { width: 1280, height: 420 };
const PHONE = { width: 360, height: 780 };

// --- The stand-in service ---

const KINDS = [
  { name: "search", description: "", hosted: false, contentsDepth: 2, icon: "travel-explore" },
  { name: "record", description: "", hosted: false, contentsDepth: 2, icon: "description-outline" },
  { name: "learning", description: "", hosted: false, contentsDepth: 3, icon: "menu-book-outline" },
  { name: "import", description: "", hosted: false, contentsDepth: 3, icon: "public" },
  { name: "page", description: "", hosted: true, contentsDepth: 0, icon: "web" }
];

const NOW = Date.now();
const HOUR = 60 * 60 * 1000;

interface Fixture {
  id: number;
  kind: string;
  title: string;
  hasPicture: boolean;
  category?: string;
  hours: number;
}

// Newest first by number. No document is an import, so the import has no section.
const FIXTURES: Fixture[] = [
  { id: 20, kind: "record", title: "Weekly deploy summary", hasPicture: false, hours: 1 },
  { id: 19, kind: "record", title: "Mail outage notes", hasPicture: false, hours: 2 },
  { id: 18, kind: "search", title: "Disk life compared", hasPicture: true, category: "investigation", hours: 3 },
  { id: 17, kind: "page", title: "Patch advisory", hasPicture: false, category: "advisory", hours: 4 },
  { id: 16, kind: "learning", title: "Index basics", hasPicture: false, hours: 5 },
  { id: 15, kind: "search", title: "GPU prices surveyed", hasPicture: false, hours: 6 },
  { id: 14, kind: "record", title: "Node plan", hasPicture: true, category: "plan", hours: 7 },
  { id: 13, kind: "search", title: "Framework migration notes", hasPicture: false, hours: 8 },
  { id: 12, kind: "record", title: "Power measured", hasPicture: false, hours: 9 },
  { id: 11, kind: "search", title: "Voice engines compared", hasPicture: false, hours: 10 },
  { id: 10, kind: "record", title: "Release checklist", hasPicture: false, hours: 11 },
  { id: 9, kind: "search", title: "Messenger options", hasPicture: false, hours: 12 },
  { id: 8, kind: "page", title: "Console manual", hasPicture: false, category: "manual", hours: 13 }
];

function documentOf(fixture: Fixture) {
  const at = NOW - fixture.hours * HOUR;

  return {
    id: fixture.id,
    kind: fixture.kind,
    title: fixture.title,
    scope: null,
    actorName: "tester",
    owner: null,
    labels: fixture.category === undefined ? {} : { category: fixture.category },
    description: `What ${fixture.title.toLowerCase()} says before it is opened.`,
    attachmentCount: 0,
    hasPicture: fixture.hasPicture,
    createdAt: at,
    changedAt: at
  };
}

const SHARED_TOKEN = "Tok3nAbc";
// The documents with a live link, which their cards say are public.
const SHARED = [18, 20];
// What the stand-in was asked to keep: each read of a document, and each share read with its visitor.
const reads: number[] = [];
const visitors: string[] = [];

// A cover picture: a one-colour square.
const PICTURE = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M/wHwAEBgIApD5fRAAAAABJRU5ErkJggg==",
  "base64"
);

function json(route: Route, body: unknown, status = 200): Promise<void> {
  return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function answer(route: Route): Promise<void> {
  const url = new URL(route.request().url());
  const method = route.request().method();
  const path = url.pathname.slice(RELAY.length);
  const query = url.searchParams;

  if (path === "/document-kinds") return json(route, { items: KINDS, more: false });

  if (path === "/documents") {
    const kind = query.get("kind");
    const phrase = (query.get("q") ?? "").toLowerCase();
    const limit = Number(query.get("limit") ?? 30);
    const found = FIXTURES.filter(
      (fixture) =>
        (kind === null || fixture.kind === kind) &&
        (query.get("picture") !== "true" || fixture.hasPicture) &&
        (phrase === "" || fixture.title.toLowerCase().includes(phrase))
    );

    return json(route, { items: found.slice(0, limit).map(documentOf), more: found.length > limit });
  }

  if (path === "/document-reads/recent") {
    const opened = [...new Set([...reads].reverse())].slice(0, Number(query.get("limit") ?? 30));

    return json(route, {
      items: opened.map((id, at) => ({ document: documentOf(FIXTURES.find((fixture) => fixture.id === id)!), readAt: NOW - at })),
      more: false
    });
  }

  if (path === "/document-reads") {
    return json(route, { items: [FIXTURES[3]!, FIXTURES[0]!, FIXTURES[6]!].map((fixture, at) => ({ document: documentOf(fixture), reads: 3 - at })), more: false });
  }

  if (path === "/shares") {
    return json(route, { items: SHARED.map((id) => ({ token: `t${id}aaaaaaa`, documentId: id, url: "", createdAt: NOW, expiresAt: null })), more: false });
  }

  if (path === "/label-values") return json(route, { items: [], more: false });

  const one = /^\/documents\/(\d+)(\/.*)?$/.exec(path);

  if (one !== null) {
    const fixture = FIXTURES.find((each) => each.id === Number(one[1]));
    const rest = one[2] ?? "";

    if (fixture === undefined) return json(route, { code: "not-found", message: "there is no such document" }, 404);
    if (rest === "/reads" && method === "POST") {
      reads.push(fixture.id);

      return route.fulfill({ status: 204 });
    }
    if (rest === "/picture") return route.fulfill({ status: 200, contentType: "image/png", body: PICTURE });
    if (rest === "/content") return route.fulfill({ status: 200, contentType: "text/markdown", body: `# ${fixture.title}\n\nWords of it.` });
    if (rest === "/attachments" || rest === "/references") return json(route, { items: [], more: false });
    if (rest === "") return json(route, documentOf(fixture));
  }

  const shared = /^\/shares\/([\w-]+)(\/.*)?$/.exec(path);

  if (shared !== null && shared[1] === SHARED_TOKEN) {
    const fixture = FIXTURES[0]!;
    const rest = shared[2] ?? "";

    if (rest === "") return json(route, { token: SHARED_TOKEN, documentId: fixture.id, url: "", createdAt: NOW, expiresAt: null });
    if (rest === "/document") {
      const visitor = query.get("visitor");

      if (visitor !== null) visitors.push(visitor);

      return json(route, documentOf(fixture));
    }
    if (rest === "/kind") return json(route, KINDS[1]);
    if (rest === "/content") return route.fulfill({ status: 200, contentType: "text/markdown", body: "# Shared\n\nWords." });
    if (rest === "/attachments" || rest === "/references") return json(route, { items: [], more: false });
  }

  return json(route, { code: "not-found", message: "nothing stands here" }, 404);
}

// --- The screens and the browser ---

let screen: Screen | null = null;
let origin = "";
let browser: Browser;
let page: Page;

async function until(holds: () => boolean | Promise<boolean>, what: string, ms = 15_000): Promise<void> {
  const end = Date.now() + ms;

  while (!(await holds())) {
    if (Date.now() > end) throw new Error(`${what} did not come within ${ms} ms`);

    await Bun.sleep(25);
  }
}

type Box = { x: number; y: number; width: number; height: number };

async function box(selector: string): Promise<Box> {
  const found = await page.locator(selector).first().boundingBox();

  if (found === null) throw new Error(`${selector} stands nowhere`);

  return found;
}

// The front page opened afresh, every part read.
async function openFront(): Promise<void> {
  await page.goto(`${origin}/artifact`, { waitUntil: "load" });
  await page.locator("[data-section]").first().waitFor();
  await page.locator("[data-most-read-item]").first().waitFor();
  await page.locator("[data-newest] [data-card]").first().waitFor();
}

// The cards of a section that stand on the screen, each its box.
async function shownCards(section: string): Promise<Box[]> {
  return page.locator(`[data-section="${section}"] [data-card]`).evaluateAll((cards) =>
    cards
      .filter((card) => getComputedStyle(card).display !== "none")
      .map((card) => {
        const { x, y, width, height } = card.getBoundingClientRect();

        return { x, y, width, height };
      })
  );
}

// The panel's body that scrolls the front page.
function frontScroller() {
  return page.locator('[data-slot="body"]').filter({ has: page.locator("[data-front]") });
}

// Whether the page scrolls sideways.
async function overflowsSideways(): Promise<boolean> {
  return page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
}

describe.skipIf(!ready)("the front page in a browser", () => {
  beforeAll(async () => {
    screen = await standScreen();
    origin = screen.origin;
    browser = screen.browser;

    const context = await browser.newContext({ viewport: WIDE });

    await context.addCookies([{ name: "nuxt-color-mode", value: "light", url: origin }]);
    page = await context.newPage();
    await page.route((url) => url.pathname.startsWith(`${RELAY}/`), answer);

    // The first load compiles the screens and may reload once their dependencies are bundled.
    await page.goto(`${origin}/artifact`, { waitUntil: "load", timeout: 120_000 });
    await page.locator("[data-section]").first().waitFor({ timeout: 120_000 });
  }, 300_000);

  afterAll(async () => {
    await screen?.close();
  }, CLOSING_MS);

  describe("at a wide window", () => {
    beforeAll(async () => {
      await page.setViewportSize(WIDE);
    });

    test("the newest are led by the pictured card of the last days, with four beside it one under another", async () => {
      await openFront();

      const cards = page.locator("[data-newest] [data-card]");

      expect(await cards.count()).toBe(5);
      expect(await cards.first().getAttribute("data-card")).toBe("18");
      expect(await cards.first().locator("[data-cover-picture]").count()).toBe(1);
      expect(await cards.nth(1).locator("[data-cover-icon]").count()).toBe(1);

      const lead = (await cards.first().boundingBox())!;
      const beside = await Promise.all([1, 2, 3, 4].map(async (at) => (await cards.nth(at).boundingBox())!));

      expect(beside.every((card) => card.x > lead.x + lead.width)).toBe(true);
      expect(beside.every((card, at) => at === 0 || card.y > beside[at - 1]!.y + beside[at - 1]!.height - 1)).toBe(true);
      expect(Math.round(beside[3]!.y + beside[3]!.height)).toBeLessThanOrEqual(Math.round(lead.y + lead.height) + 1);
      expect(await cards.first().locator("[data-shared]").count()).toBe(1);
    }, 60_000);

    test("a section stands for each kind the reader has a document of, in the catalogue's order, each one row of cards", async () => {
      await openFront();

      const sections = await page.locator("[data-section]").evaluateAll((found) => found.map((section) => section.getAttribute("data-section")));

      expect(sections).toEqual(["search", "record", "learning", "page"]);

      const search = await shownCards("search");

      expect(search).toHaveLength(4);
      expect(new Set(search.map((card) => Math.round(card.y))).size).toBe(1);
      expect(await shownCards("learning")).toHaveLength(1);
    }, 60_000);

    test("the kinds and then the most read stand in a column beside the newest and the sections, from the top down", async () => {
      await openFront();

      const newest = await box("[data-newest]");
      const sections = await box("[data-sections]");
      const side = await box("[data-side]");
      const kinds = await box("[data-kinds]");
      const mostRead = await box("[data-most-read-part]");

      expect(side.x).toBeGreaterThanOrEqual(sections.x + sections.width);
      expect(side.x).toBeGreaterThanOrEqual(newest.x + newest.width);
      expect(Math.round(side.y)).toBe(Math.round(newest.y));
      expect(mostRead.y).toBeGreaterThan(kinds.y + kinds.height - 1);
      expect(await page.locator("[data-kind-chips]").isVisible()).toBe(false);
      expect(await page.locator("[data-most-read-item]").allInnerTexts()).toEqual(["1\nPatch advisory", "2\nWeekly deploy summary", "3\nNode plan"]);
      expect(await page.locator("[data-kinds] a").allInnerTexts()).toEqual([
        WORDS.artifact.front.all_kinds,
        ...KINDS.map((kind) => KIND_WORDS[kind.name]!)
      ]);
      expect(await overflowsSideways()).toBe(false);
    }, 60_000);

    test("the side column stays in view while the page scrolls, never taller than the body's visible room", async () => {
      await openFront();

      const scroller = frontScroller();
      const room = (await scroller.boundingBox())!;
      const before = await box("[data-side]");

      await scroller.evaluate((element) => element.scrollTo({ top: element.scrollHeight }));
      await until(async () => (await scroller.evaluate((element) => element.scrollTop)) > 0, "the page's scroll");

      const after = await box("[data-side]");
      const sections = await box("[data-sections]");

      expect(sections.y + sections.height).toBeLessThan(room.y + room.height + 1);
      expect(after.y).toBeGreaterThanOrEqual(room.y);
      expect(after.y + after.height).toBeLessThanOrEqual(room.y + room.height);
      expect(Math.round(after.y)).toBe(Math.round(before.y));
      expect(await page.locator("[data-kinds]").isVisible()).toBe(true);
    }, 60_000);

    test("the recently read stand above the most read, the document just opened first", async () => {
      await openFront();
      await page.locator('[data-section="learning"] [data-card-title]').click();
      await page.waitForURL(`${origin}/artifact/16`);
      await until(() => reads.at(-1) === 16, "the document's read");
      await page.locator("[data-back]").click();
      await page.locator("[data-front]").waitFor();
      await until(async () => (await page.locator("[data-recent-read-item]").first().innerText().catch(() => "")) === "Index basics", "the read listed");

      const kinds = await box("[data-kinds]");
      const recent = await box("[data-recent-read-part]");
      const mostRead = await box("[data-most-read-part]");

      expect(recent.y).toBeGreaterThan(kinds.y + kinds.height - 1);
      expect(mostRead.y).toBeGreaterThan(recent.y + recent.height - 1);
      expect(await page.locator("[data-recent-read-part] h2").innerText()).toBe(WORDS.artifact.front.recent_read);
    }, 60_000);

    test("a section's link opens the list of its kind, and the way back brings the front page again", async () => {
      await openFront();
      await page.locator('[data-section="record"] [data-see-all]').click();
      await page.waitForURL((url) => url.searchParams.get("kind") === "record");
      await page.locator("ul li").first().waitFor();

      expect(await page.locator("[data-front]").count()).toBe(0);
      expect(await page.locator("ul li").count()).toBe(FIXTURES.filter((fixture) => fixture.kind === "record").length);

      await page.locator("[data-to-front]").click();
      await page.locator("[data-front]").waitFor();
      expect(new URL(page.url()).search).toBe("");
    }, 60_000);

    test("a phrase typed turns the front page into the list it finds, the field keeping the focus, and cleared brings it back", async () => {
      await openFront();

      const field = page.getByRole("textbox");

      await field.click();
      await field.pressSequentially("compared");
      await page.waitForURL((url) => url.searchParams.get("q") === "compared");
      await page.locator("ul li").first().waitFor();

      expect(await page.locator("[data-front]").count()).toBe(0);
      expect(await page.locator("ul li").count()).toBe(2);
      expect(await field.evaluate((element) => element === document.activeElement)).toBe(true);

      await field.fill("");
      await page.locator("[data-front]").waitFor();
      expect(new URL(page.url()).searchParams.get("q")).toBeNull();
    }, 60_000);

    test("a card opens its document, which keeps a read and goes back to the front page", async () => {
      await openFront();
      reads.length = 0;
      await page.locator('[data-section="learning"] [data-card-title]').click();
      await page.waitForURL(`${origin}/artifact/16`);
      await until(() => reads.includes(16), "the document's read");

      await page.locator("[data-back]").click();
      await page.locator("[data-front]").waitFor();
      expect(new URL(page.url()).pathname).toBe("/artifact");
    }, 60_000);

    test("back from a card opened low on the front page, the reader finds it where they left it", async () => {
      await openFront();

      const scroller = page.locator('[data-slot="body"]').filter({ has: page.locator("[data-front]") });

      await page.locator('[data-section="page"]').scrollIntoViewIfNeeded();

      const left = await scroller.evaluate((element) => element.scrollTop);

      expect(left).toBeGreaterThan(0);
      await page.locator('[data-section="page"] [data-card-title]').first().click();
      await page.waitForURL(`${origin}/artifact/17`);
      await page.locator("[data-back]").click();
      await page.locator("[data-front]").waitFor();
      await until(async () => Math.abs((await scroller.evaluate((element) => element.scrollTop)) - left) <= 1, "the place left");
    }, 60_000);

    test("a share page keeps a read for its browser, the same token each time", async () => {
      visitors.length = 0;

      for (const _ of [1, 2]) {
        await page.goto(`${origin}/artifact/share/${SHARED_TOKEN}`, { waitUntil: "load" });
        await until(() => visitors.length === _, "the share page's read");
      }

      expect(visitors[0]).toMatch(/^[\w-]{8,64}$/);
      expect(visitors[1]).toBe(visitors[0]!);
    }, 60_000);
  });

  describe("at a wide window too short for the side column", () => {
    beforeAll(async () => {
      await page.setViewportSize(SHORT);
    });

    afterAll(async () => {
      await page.setViewportSize(WIDE);
    });

    test("the side column keeps to the body's visible room and its own content scrolls to its last line", async () => {
      await openFront();

      const scroller = frontScroller();
      const room = (await scroller.boundingBox())!;
      const side = page.locator("[data-side]");
      const held = await box("[data-side]");

      expect(held.y).toBeGreaterThanOrEqual(room.y);
      expect(held.y + held.height).toBeLessThanOrEqual(room.y + room.height);
      expect(await side.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true);

      const last = page.locator("[data-most-read-item]").last();
      const pageTop = await scroller.evaluate((element) => element.scrollTop);

      await page.mouse.move(held.x + held.width / 2, held.y + held.height / 2);
      await page.mouse.wheel(0, 2000);
      await until(async () => (await side.evaluate((element) => element.scrollTop + element.clientHeight >= element.scrollHeight - 1)), "the side column's end");

      const line = (await last.boundingBox())!;

      expect(line.y).toBeGreaterThanOrEqual(held.y);
      expect(line.y + line.height).toBeLessThanOrEqual(held.y + held.height + 1);
      expect(await scroller.evaluate((element) => element.scrollTop)).toBe(pageTop);
    }, 60_000);
  });

  describe("at a phone's width", () => {
    beforeAll(async () => {
      await page.setViewportSize(PHONE);
    });

    test("everything stands in one column: the kinds as chips under the bar, the newest, the most read, then the sections", async () => {
      await openFront();

      const chips = await box("[data-kind-chips]");
      const newest = await box("[data-newest]");
      const lead = await box("[data-newest] [data-card]");
      const recent = await box("[data-recent-read-part]");
      const mostRead = await box("[data-most-read-part]");
      const sections = await box("[data-sections]");

      expect(newest.y).toBeGreaterThan(chips.y + chips.height - 1);
      expect(recent.y).toBeGreaterThan(newest.y + newest.height - 1);
      expect(mostRead.y).toBeGreaterThan(recent.y + recent.height - 1);
      expect(sections.y).toBeGreaterThan(mostRead.y + mostRead.height - 1);
      expect(Math.round(mostRead.x)).toBe(Math.round(sections.x));
      expect(lead.width).toBeGreaterThan(PHONE.width * 0.8);
      expect(await page.locator("[data-kinds]").isVisible()).toBe(false);
      expect(await page.locator("[data-kind-chips] a").allInnerTexts()).toEqual([
        WORDS.artifact.front.all_kinds,
        ...KINDS.map((kind) => KIND_WORDS[kind.name]!)
      ]);
      expect(await overflowsSideways()).toBe(false);
    }, 60_000);

    test("nothing is held in place: the side parts scroll away with the page, and every line is reached by scrolling it", async () => {
      await openFront();

      const scroller = frontScroller();
      const side = page.locator("[data-side]");

      expect(await side.evaluate((element) => [getComputedStyle(element).position, getComputedStyle(element).overflowY])).toEqual(["static", "visible"]);

      const last = page.locator("[data-most-read-item]").last();

      await last.scrollIntoViewIfNeeded();

      const line = (await last.boundingBox())!;
      const room = (await scroller.boundingBox())!;

      expect(line.y).toBeGreaterThanOrEqual(room.y);
      expect(line.y + line.height).toBeLessThanOrEqual(room.y + room.height);

      const before = await box("[data-side]");

      await scroller.evaluate((element) => element.scrollTo({ top: element.scrollHeight }));
      await until(async () => (await box("[data-side]")).y < before.y - 1, "the side parts scrolling away");
    }, 60_000);

    test("a chip opens the list of its kind", async () => {
      await openFront();
      await page.locator("[data-kind-chips] a").nth(2).click();
      await page.waitForURL((url) => url.searchParams.get("kind") === "record");
      await page.locator("ul li").first().waitFor();
      expect(await page.locator("[data-front]").count()).toBe(0);
    }, 60_000);

    test("a card but the lead stands as a row, its cover at its leading edge, and a section holds three of them one under another", async () => {
      await openFront();

      const card = page.locator("[data-newest] [data-card]").nth(1);
      const cover = (await card.locator("[data-cover]").boundingBox())!;
      const title = (await card.locator("[data-card-title]").boundingBox())!;

      expect(title.x).toBeGreaterThan(cover.x + cover.width);

      const search = await shownCards("search");

      expect(search).toHaveLength(3);
      expect(search[1]!.y).toBeGreaterThan(search[0]!.y + search[0]!.height - 1);
      expect(new Set(search.map((each) => Math.round(each.x))).size).toBe(1);
    }, 60_000);

    test("the bar stays one row with the search and the refresh alone", async () => {
      await openFront();

      const field = await box('[role="textbox"], input');
      const refresh = await box(`button[aria-label="${WORDS.artifact.action.refresh}"]`);

      expect(Math.abs(field.y + field.height / 2 - (refresh.y + refresh.height / 2))).toBeLessThan(2);
      expect(await page.locator("[data-filters], [data-filter]").count()).toBe(0);
    }, 60_000);
  });
});
