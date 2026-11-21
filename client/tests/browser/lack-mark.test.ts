import { afterAll, beforeAll, describe, expect, test } from "bun:test";

import words from "../../i18n/ko.json";
import { browserReady, standScreen } from "./screen";

import type { BrowserContext, Page, Route } from "playwright-core";
import type { ArtifactDocument, ArtifactKind } from "../../shared/types";
import type { Screen } from "./screen";

// What a document lacks and what spins on the document screens (`docs/artifact.md`, 「A document that lacks a part」),
// driven in Chromium at a wide window and at a phone's width against a stand-in of the service: a document that
// lacks a part says so in one still mark on its row and under its own title, never in either screen's bar, and
// nothing turns on a screen once what it asked has come. A read still running turns its control until it ends.
// Skipped where the browser playwright-core pins is not on the machine.

const ready = browserReady();

// What a test may take at most, a bound and never a wait: the dev server builds the host on its first page.
const STANDING_MS = 300_000;
const DRIVING_MS = 120_000;

const WIDTHS = [
  { name: "a wide window", viewport: { width: 1280, height: 900 }, touch: false },
  { name: "a phone", viewport: { width: 360, height: 780 }, touch: true }
] as const;

type Width = (typeof WIDTHS)[number];

const KINDS: ArtifactKind[] = [
  { name: "record", description: "A record", hosted: false, contentsDepth: 2, icon: "description-outline" },
  { name: "page", description: "A page", hosted: true, contentsDepth: 0, icon: "web" }
];

function filed(id: number, kind: string, title: string, labels: Record<string, string>): ArtifactDocument {
  return {
    id,
    kind,
    title,
    scope: null,
    actorName: "tester",
    owner: null,
    labels,
    description: `About ${title.toLowerCase()}`,
    attachmentCount: 0,
    hasPicture: false,
    createdAt: 1_790_000_000_000,
    changedAt: 1_790_000_000_000
  };
}

const LACKING = filed(3, "record", "Lacking record", { pending: "narration-ko", category: "report" });
const WHOLE = filed(2, "record", "Whole record", { category: "report" });
const LACKING_PAGE = filed(1, "page", "Lacking page", { pending: "korean, review" });
const DOCUMENTS = [LACKING, WHOLE, LACKING_PAGE];

// The still mark's words for what a document lacks, from the screens' own pack.
function lackWords(...parts: (keyof typeof words.artifact.pending)[]): string {
  return words.artifact.label.lacks.replace("{lacks}", parts.map((part) => words.artifact.pending[part]).join(", "));
}

// The stand-in behind the relay: the routes the two screens read, each answered at once, but the list's documents
// while a test holds them back. Any other route is answered 404.
interface StandIn {
  holdList: () => () => void;
}

async function standBehind(context: BrowserContext): Promise<StandIn> {
  let held: Promise<void> | null = null;
  const json = (route: Route, body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  const none = { items: [], more: false };

  await context.route("**/relay/artifact/**", async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/relay\/artifact/, "");
    const one = /^\/documents\/(\d+)(\/[a-z-]+)?$/.exec(path);

    if (path === "/documents") {
      if (held !== null) await held;
      return json(route, { items: DOCUMENTS, more: false });
    }
    if (path === "/document-kinds") return json(route, { items: KINDS, more: false });
    if (path === "/label-values" || path === "/shares") return json(route, none);

    const document = one === null ? undefined : DOCUMENTS.find((it) => it.id === Number(one[1]));

    if (document === undefined) return route.fulfill({ status: 404, body: "" });
    if (one![2] === undefined) return json(route, document);
    if (one![2] === "/attachments" || one![2] === "/references") return json(route, none);
    if (one![2] === "/content" && document.kind === "page") return route.fulfill({ status: 200, contentType: "text/html", body: "<!doctype html><p>Page body</p>" });
    if (one![2] === "/content") return route.fulfill({ status: 200, contentType: "text/markdown", body: "# Lacking record\n\nBody text." });

    return route.fulfill({ status: 404, body: "" });
  });

  return {
    holdList: () => {
      let release = () => {};

      held = new Promise<void>((resolve) => {
        release = () => {
          held = null;
          resolve();
        };
      });

      return release;
    }
  };
}

// The marks turning on the page now: every running CSS animation that spins, whatever element carries it.
function spinning(page: Page): Promise<number> {
  return page.evaluate(() => document.getAnimations().filter((one) => one.playState === "running" && (one as CSSAnimation).animationName === "spin").length);
}

let screen: Screen;

beforeAll(async () => {
  if (ready) screen = await standScreen();
}, STANDING_MS);

afterAll(async () => {
  await screen?.close();
});

// A page of the screens at one width, handed to `drive` with the stand-in and closed with its context however `drive` ends.
async function onScreen(width: Width, drive: (page: Page, stand: StandIn) => Promise<void>): Promise<void> {
  const context = await screen.browser.newContext({ viewport: width.viewport, hasTouch: width.touch, isMobile: width.touch });

  try {
    const stand = await standBehind(context);

    await drive(await context.newPage(), stand);
  } finally {
    await context.close();
  }
}

for (const width of WIDTHS) {
  describe.skipIf(!ready)(`at ${width.name}`, () => {
    test(
      "the list says what a document lacks on its row alone, still, and nothing turns once the list is read",
      async () => {
        await onScreen(width, async (page) => {
          await page.goto(`${screen.origin}/artifact?view=list`);

          const rows = page.locator("ul[aria-busy] > li");

          await rows.nth(DOCUMENTS.length - 1).waitFor();
          await page.waitForLoadState("networkidle");
          expect(await rows.count()).toBe(DOCUMENTS.length);

          const row = (title: string) => rows.filter({ hasText: title }).locator("[data-lacks]");

          expect(await row("Lacking record").innerText()).toBe(lackWords("narration-ko"));
          expect(await row("Lacking page").innerText()).toBe(lackWords("korean", "review"));
          expect(await row("Whole record").count()).toBe(0);
          expect(await row("Lacking record").evaluate((mark) => getComputedStyle(mark).animationName)).toBe("none");
          expect(await page.locator("[class*='progress-activity'], .animate-spin").count()).toBe(0);
          expect(await spinning(page)).toBe(0);

          // The bar carries no mark and no narrowing by it, standing or folded into its filter popover.
          const bar = page.locator("[data-list-bar]");

          expect(await bar.locator("[data-lacks]").count()).toBe(0);
          if ((await page.locator("[data-filters]").count()) > 0) {
            await page.locator("[data-filters]").click();
            await page.locator("[data-filter-popover]").waitFor();
            expect(await page.locator("[data-filter-popover] [class*='progress-activity']").count()).toBe(0);
          }
          expect(await bar.innerText()).not.toContain(words.artifact.pending.review);
          expect(await spinning(page)).toBe(0);
        });
      },
      DRIVING_MS
    );

    test(
      "the refresh turns while the list is read again, and stops once it is read",
      async () => {
        await onScreen(width, async (page, stand) => {
          await page.goto(`${screen.origin}/artifact?view=list`);
          await page.locator("ul[aria-busy] > li").nth(DOCUMENTS.length - 1).waitFor();
          await page.waitForLoadState("networkidle");

          const release = stand.holdList();
          const refresh = page.locator("[data-list-bar]").getByRole("button", { name: words.artifact.action.refresh });

          await refresh.click();
          await page.waitForFunction(() => document.getAnimations().some((one) => one.playState === "running" && (one as CSSAnimation).animationName === "spin"));
          release();
          await page.waitForFunction(() => !document.getAnimations().some((one) => one.playState === "running" && (one as CSSAnimation).animationName === "spin"));
          expect(await spinning(page)).toBe(0);
        });
      },
      DRIVING_MS
    );

    test(
      "a document says what it lacks under its own title, never in the screen's row",
      async () => {
        await onScreen(width, async (page) => {
          await page.goto(`${screen.origin}/artifact/${LACKING.id}`);
          await page.locator(".artifact-masthead [data-lacks]").waitFor();
          await page.waitForLoadState("networkidle");

          expect(await page.locator(".artifact-masthead [data-lacks]").innerText()).toBe(lackWords("narration-ko"));
          expect(await page.locator("[data-document-labels]").innerText()).not.toContain("pending");
          expect(await page.locator("[data-document-row] [data-lacks]").count()).toBe(0);
          expect(await spinning(page)).toBe(0);

          await page.goto(`${screen.origin}/artifact/${LACKING_PAGE.id}`);
          await page.locator("[data-page-title] + [data-lacks]").waitFor();
          await page.waitForLoadState("networkidle");

          expect(await page.locator("[data-page-title] + [data-lacks]").innerText()).toBe(lackWords("korean", "review"));
          expect(await page.locator("[data-document-row] [data-lacks]").count()).toBe(0);
          expect(await spinning(page)).toBe(0);

          await page.goto(`${screen.origin}/artifact/${WHOLE.id}`);
          await page.locator(".artifact-masthead").waitFor();
          await page.waitForLoadState("networkidle");

          expect(await page.locator("[data-lacks]").count()).toBe(0);
          expect(await spinning(page)).toBe(0);
        });
      },
      DRIVING_MS
    );
  });
}
