import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium } from "playwright-core";

import type { Browser } from "playwright-core";

// A kept picture the body draws small is drawn from the copy the service makes near that size (`docs/artifact.md`,
// "A picture drawn small"), checked in a real browser: the kept file is red and every copy blue, so what the browser
// draws says which it drew. The picture stands in a column as a markdown figure does, on a screen of density 1.25;
// once it has loaded it is drawn from the first copy covering the pixels it stands on, keeps the width it stood at,
// and opens whole in the viewer from its kept file. A picture the body draws from the web is left as it came.

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();
const built = ready
  ? await Bun.build({
      entrypoints: [new URL("./drawn-pictures-page.ts", import.meta.url).pathname],
      target: "browser"
    })
  : null;
const script = built === null ? "" : await built.outputs[0]!.text();
const ORIGIN = "https://console.test";
const DENSITY = 1.25;
const COLUMN = 320;

const square = (width: number, height: number, colour: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="${width}" height="${height}" fill="${colour}"/></svg>`;

const PAGE = `<!doctype html><html><head><style>
  body { margin: 0; } main { width: ${COLUMN}px; }
  img { display: block; max-width: 100%; max-height: 24rem; height: auto; }
</style></head><body><main>
  <img id="kept" data-drawn-copies src="${ORIGIN}/documents/1/attachments/7" alt="kept">
  <img id="web" src="${ORIGIN}/web.svg" alt="web">
</main><script type="module">${script}</script></body></html>`;

let browser: Browser | null = null;

beforeAll(async () => {
  if (ready) browser = await chromium.launch();
}, 60_000);

afterAll(async () => {
  await browser?.close();
}, 60_000);

describe.skipIf(!ready)("a kept picture the body draws small", () => {
  test("is drawn from the copy covering its pixels, keeps its width and opens whole from its kept file", async () => {
    const context = await browser!.newContext({ viewport: { width: 600, height: 800 }, deviceScaleFactor: DENSITY });
    const page = await context.newPage();
    const asked: string[] = [];

    page.on("pageerror", (error) => console.log("page error", error.message));

    try {
      await page.route(`${ORIGIN}/**`, (route) => {
        const url = new URL(route.request().url());

        asked.push(`${url.pathname}${url.search}`);

        if (url.pathname === "/") return route.fulfill({ contentType: "text/html; charset=utf-8", body: PAGE });
        if (url.pathname === "/web.svg") return route.fulfill({ contentType: "image/svg+xml", body: square(1000, 500, "#00ff00") });

        const width = url.searchParams.get("width");

        return route.fulfill({
          contentType: "image/svg+xml",
          body: width === null ? square(1000, 500, "#ff0000") : square(Number(width), Math.round(Number(width) / 2), "#0000ff")
        });
      });
      await page.goto(`${ORIGIN}/`);
      await page.waitForFunction(() => (document.getElementById("kept") as HTMLImageElement).currentSrc.includes("width="), undefined, { timeout: 30_000 });
      await page.waitForFunction(() => (document.getElementById("kept") as HTMLImageElement).complete);

      const seen = await page.evaluate(() => {
        const kept = document.getElementById("kept") as HTMLImageElement;
        const web = document.getElementById("web") as HTMLImageElement;
        const canvas = document.createElement("canvas");

        canvas.width = 1;
        canvas.height = 1;

        return {
          drawnFrom: kept.currentSrc,
          width: kept.getBoundingClientRect().width,
          webSrcset: web.srcset,
          viewer: (window as unknown as { viewerOf: (image: HTMLImageElement) => string | null }).viewerOf(kept)
        };
      });
      const shot = await page.locator("#kept").screenshot();
      const middle = await page.evaluate(async (data) => {
        const image = new Image();

        image.src = `data:image/png;base64,${data}`;
        await image.decode();

        const canvas = new OffscreenCanvas(image.width, image.height);
        const context = canvas.getContext("2d")!;

        context.drawImage(image, 0, 0);

        const [red, , blue] = context.getImageData(image.width >> 1, image.height >> 1, 1, 1).data;

        return { red: red!, blue: blue! };
      }, shot.toString("base64"));

      // 320 pixels at a density of 1.25 stand on 400, so the copy at 425, the first step past them, is drawn.
      expect(new URL(seen.drawnFrom).searchParams.get("width")).toBe("425");
      expect(Math.abs(seen.width - COLUMN)).toBeLessThan(0.5);
      expect(middle.blue).toBeGreaterThan(200);
      expect(middle.red).toBeLessThan(50);
      // The viewer opens the kept file, and the picture from the web is drawn as it came.
      expect(seen.viewer).toBe(`${ORIGIN}/documents/1/attachments/7`);
      expect(seen.webSrcset).toBe("");
      expect(asked.some((one) => one.startsWith("/web.svg?"))).toBe(false);
    } finally {
      await context.close();
    }
  });
});
