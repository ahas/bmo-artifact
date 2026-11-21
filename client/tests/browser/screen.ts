import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { get } from "node:http";
import { createServer } from "node:net";

import { chromium } from "playwright-core";

import type { Browser } from "playwright-core";

// The document screens standing on their own for a browser test: the client's layer served by Nuxt's dev server
// from the host under `host/` on a free loopback port, and a headless Chromium to drive it. The dev server runs in
// a process group of its own, so taking it down takes every process it started with it.

export interface Screen {
  origin: string;
  browser: Browser;
  close: () => Promise<void>;
}

/** Whether the browser this version of playwright pins is on the machine; a test that needs it is skipped where it is not. */
export function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

// How long the dev server may take to answer, which it builds the host for on the first ask.
const READY_MS = 180_000;

/** What a hook taking a screen or a browser down may take at most, a bound and never a wait: a loaded machine closes slowly. */
export const CLOSING_MS = 60_000;

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = createServer();

    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;

      probe.close(() => resolve(port));
    });
  });
}

// Whether the dev server answers an address with success. It is asked over Node's own HTTP client: the tests run
// with happy-dom's window registered, whose fetch holds a call to another origin to a browser's rules.
function answers(address: string): Promise<boolean> {
  return new Promise((resolve) => {
    get(address, (response) => {
      response.resume();
      resolve(response.statusCode === 200);
    }).once("error", () => resolve(false));
  });
}

// The dev server's answer to the list's address once it gives one, asked until it does or the bound passes.
async function ready(origin: string, exited: () => boolean): Promise<void> {
  const deadline = Date.now() + READY_MS;

  while (Date.now() < deadline) {
    if (exited()) throw new Error("The artifact client's dev server ended before it answered.");

    if (await answers(`${origin}/artifact`)) return;

    await Bun.sleep(250);
  }

  throw new Error(`The artifact client's dev server did not answer ${origin}/artifact in time.`);
}

// The screens loaded once in a page of their own before any test drives them: the dev server compiles them on the
// first page that asks, which on a busy machine takes longer than a test's own waits allow, so no test pays for it.
// The service's routes are left unanswered here; the screen needs only to stand.
async function warm(browser: Browser, origin: string): Promise<void> {
  const page = await browser.newPage();

  try {
    await page.goto(`${origin}/artifact`, { waitUntil: "load", timeout: READY_MS });
    await page.locator("[data-front]").waitFor({ timeout: READY_MS });
  } finally {
    await page.close();
  }
}

export async function standScreen(): Promise<Screen> {
  const port = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const server = spawn("bun", ["x", "nuxi", "dev", "--port", String(port), "--host", "127.0.0.1"], {
    cwd: new URL("./host", import.meta.url).pathname,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"]
  });
  let exited = false;
  const ended = new Promise<void>((resolve) => server.once("exit", () => resolve()));
  // What the dev server said, kept for the failure that names it; it ends at once when its output is cast aside,
  // so the output is read.
  let said = "";
  const hear = (chunk: Buffer) => {
    said = (said + chunk.toString()).slice(-4000);
  };

  server.stdout.on("data", hear);
  server.stderr.on("data", hear);
  server.once("exit", () => {
    exited = true;
  });

  // The dev server's whole group taken down, and its end waited for.
  const stop = async () => {
    if (!exited && server.pid !== undefined) process.kill(-server.pid, "SIGTERM");
    await ended;
  };

  try {
    await ready(origin, () => exited);

    const browser = await chromium.launch();

    try {
      await warm(browser, origin);
    } catch (failure) {
      await browser.close();
      throw failure;
    }

    return {
      origin,
      browser,
      close: async () => {
        await browser.close();
        await stop();
      }
    };
  } catch (failure) {
    await stop();
    throw new Error(`${(failure as Error).message}\nThe dev server said:\n${said}`);
  }
}
