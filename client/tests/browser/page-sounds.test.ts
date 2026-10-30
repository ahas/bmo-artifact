import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium } from "playwright-core";

import hosting from "../../design/hosted-page.json";
import { HOSTED_PAGE_SANDBOX } from "../../shared/config";

import type { Browser, Page } from "playwright-core";

// A page's sounds and its questions about the scheme (`docs/archive.md`, "An HTML page"), checked in a real
// browser where the page stands in a frame sandboxed as the console's is, under the policy every hosted page
// goes out with. The page writes its own band with an `audio` element naming the kept file; the lines ahead
// of it are the design's bridges and bmo-ui's player script, put together as the server puts them, and the
// sound is answered at the address the server hands the page for it.

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();

const bridge = async (name: string): Promise<string> => (await Bun.file(new URL(`../../design/bridges/${name}.js`, import.meta.url)).text()).trimEnd();
const POLICY = [`sandbox ${hosting.sandbox}`, ...hosting.policy].join("; ");
const PLAYER = await Bun.file(new URL("../../node_modules/bmo-ui/dist/bmo-audio-player.js", import.meta.url)).text();
const words = (await Bun.file(new URL("../../i18n/en.json", import.meta.url)).json()) as { artifact: { player: unknown } };

// A second of a tone, as a WAV file: a sound any browser plays with no codec of its own.
function tone(): Buffer {
  const rate = 8000;
  const bytes = Buffer.alloc(44 + rate * 2);
  const ascii = (at: number, text: string) => bytes.write(text, at, "ascii");

  ascii(0, "RIFF");
  bytes.writeUInt32LE(36 + rate * 2, 4);
  ascii(8, "WAVEfmt ");
  bytes.writeUInt32LE(16, 16);
  bytes.writeUInt16LE(1, 20);
  bytes.writeUInt16LE(1, 22);
  bytes.writeUInt32LE(rate, 24);
  bytes.writeUInt32LE(rate * 2, 28);
  bytes.writeUInt16LE(2, 32);
  bytes.writeUInt16LE(16, 34);
  ascii(36, "data");
  bytes.writeUInt32LE(rate * 2, 40);

  for (let at = 0; at < rate; at += 1) bytes.writeInt16LE(Math.round(Math.sin((at / rate) * 2 * Math.PI * 440) * 12000 * (at / rate)), 44 + at * 2);

  return bytes;
}

const ORIGIN = "https://archive.test";
const CONSOLE = `${ORIGIN}/console`;
const PAGE_ADDRESS = `${ORIGIN}/api/artifact/documents/7/content`;
const SOUND_ADDRESS = `${ORIGIN}/api/artifact/readings/token/attachments/9`;
const PLAYER_ADDRESS = `${ORIGIN}/api/artifact/player/bmo-audio-player.js`;

// The lines ahead of the page, for a reader in the dark scheme.
const SOUNDS = {
  files: { "narration-ko.ogg": { play: "../../readings/token/attachments/9", download: "../../readings/token/attachments/9?download=true" } },
  words: words.artifact.player,
  locale: "en",
  scheme: "dark"
};
const PRELUDE = [
  `<script data-archive-prelude>${await bridge("scheme")}("dark")</script>`,
  `<script data-archive-prelude defer src="../../player/bmo-audio-player.js"></script>`,
  `<script data-archive-prelude>${await bridge("sound")}(${JSON.stringify(SOUNDS)})</script>`
].join("");

// A research page's band under its title, its Mermaid-style question asked as its scripts run, and its
// language switch pausing the sounds it no longer shows.
const PAGE = `<!doctype html>${PRELUDE}<html lang="en"><head><title>Read</title></head><body>
  <h1>Read</h1>
  <section id="band" class="band"><p>Listen while you read</p>
    <audio id="ko" src="attachment:narration-ko.ogg" data-language="ko" controls preload="none"></audio>
    <audio id="missing" src="attachment:none.ogg" data-language="en"></audio>
  </section>
  <p>Body.</p>
  <script>window.askedDark = matchMedia("(prefers-color-scheme: dark)").matches;</script>
</body></html>`;

const FRAMING = `<iframe id="page" src="${PAGE_ADDRESS}" sandbox="${HOSTED_PAGE_SANDBOX}" style="width:700px;height:500px"></iframe>`;

let browser: Browser;
let page: Page;

describe.skipIf(!ready)("a page's sounds and scheme", () => {
  beforeAll(async () => {
    browser = await chromium.launch();
    // The reader's system asks light; the reader's scheme is dark.
    page = await browser.newPage({ viewport: { width: 800, height: 600 }, colorScheme: "light" });
    await page.context().route(/^https?:/, (route) => {
      const address = route.request().url().split("?")[0];

      if (address === CONSOLE) return route.fulfill({ contentType: "text/html; charset=utf-8", body: FRAMING });
      if (address === PAGE_ADDRESS) return route.fulfill({ contentType: "text/html; charset=utf-8", headers: { "content-security-policy": POLICY }, body: PAGE });
      if (address === PLAYER_ADDRESS) return route.fulfill({ contentType: "text/javascript", body: PLAYER });
      if (address === SOUND_ADDRESS) return route.fulfill({ contentType: "audio/wav", headers: { "access-control-allow-origin": "*" }, body: tone() });

      return route.abort();
    });
    await page.goto(CONSOLE);
    await page.frameLocator("#page").locator("#ko [data-attachment]").waitFor();
  });

  afterAll(async () => {
    await browser?.close();
  });

  const inPage = <T>(run: () => T): Promise<T> => page.frames().find((frame) => frame.url().startsWith(PAGE_ADDRESS))!.evaluate(run);

  test("stands each audio element in the page's band as the design's player, in its place, its attributes kept", async () => {
    const seen = await inPage(() => {
      const band = document.getElementById("band")!;

      return {
        audios: document.querySelectorAll("audio:not(bmo-audio-player audio)").length,
        children: [...band.children].map((child) => `${child.localName}${child.id === "" ? "" : `#${child.id}`}`),
        language: document.getElementById("ko")!.getAttribute("data-language"),
        src: document.getElementById("ko")!.getAttribute("src"),
        name: document.getElementById("ko")!.shadowRoot!.querySelector("[data-attachment]")!.getAttribute("aria-label"),
        ground: document.getElementById("ko")!.shadowRoot!.querySelector(".audio-element")!.classList.contains("dark")
      };
    });

    expect(seen).toEqual({
      audios: 0,
      children: ["p", "bmo-audio-player#ko", "bmo-audio-player#missing"],
      language: "ko",
      src: "../../readings/token/attachments/9",
      name: "narration-ko.ogg",
      ground: true
    });
  });

  test("plays the kept sound from the address the page was handed, and the page's own script pauses it", async () => {
    await page.frameLocator("#page").locator("#ko button.rounded-full").click();

    const frame = page.frames().find((one) => one.url().startsWith(PAGE_ADDRESS))!;

    await frame.waitForFunction(() => (document.getElementById("ko")!.shadowRoot!.querySelector("audio")!.currentTime ?? 0) > 0.1);
    await frame.evaluate(() => {
      for (const sound of document.querySelectorAll<HTMLElement & { pause: () => void }>("[data-language]")) sound.pause();
    });

    expect(await frame.evaluate(() => document.getElementById("ko")!.shadowRoot!.querySelector("audio")!.paused)).toBe(true);
  });

  test("opens its volume inside the player", async () => {
    await page.frameLocator("#page").locator("#ko [data-volume]").click();

    expect(await inPage(() => document.getElementById("ko")!.shadowRoot!.querySelector("button[aria-label=Mute]") !== null)).toBe(true);
  });

  test("leaves a sound the document keeps no file for saying it cannot be played", async () => {
    await page.frameLocator("#page").locator("#missing").getByText("Cannot be played").waitFor();
  });

  test("answers the page's question about the scheme with the reader's, not the system's", async () => {
    expect(await inPage(() => (window as unknown as { askedDark: boolean }).askedDark)).toBe(true);
    expect(await inPage(() => matchMedia("(prefers-color-scheme: light)").matches)).toBe(false);
  });
});
