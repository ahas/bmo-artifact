import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { CONSOLE_SHARED_LIBRARIES, LAYER_VERSION } from "bmo-ui/shared/console-manifest";

// The browser module a console loads (`docs/layout.md`, "The console bundle"), built as `bun run module`
// builds it into a folder of the test's own: one module whose default export is the screens' routes,
// importing nothing but the libraries bmo-ui names as shared with the console, with its stylesheet, its words
// and the layer it was built on beside it.

const ROOT = resolve(import.meta.dir, "..");
const transpiler = new Bun.Transpiler({ loader: "js" });

let out = "";

beforeAll(() => {
  out = mkdtempSync(join(tmpdir(), "artifact-module-"));

  const built = Bun.spawnSync(["bun", "x", "vite", "build", "--config", "module.config.ts", "--outDir", out, "--emptyOutDir", "--logLevel", "error"], { cwd: ROOT });

  if (built.exitCode !== 0) throw new Error(`the module build failed: ${built.stderr.toString()}`);
}, 120_000);

afterAll(() => {
  rmSync(out, { recursive: true, force: true });
});

describe("the module built", () => {
  test("is one module, its stylesheet, its words and its layer, with bmo-ui's audio player script beside them", () => {
    expect(readdirSync(out).sort()).toEqual(["bmo-audio-player.js", "bundle.css", "bundle.js", "layer.json", "words.json"]);
    expect(readFileSync(join(out, "bmo-audio-player.js"))).toEqual(readFileSync(join(ROOT, "node_modules/bmo-ui/dist/bmo-audio-player.js")));
  });

  test("exports a default and imports only the libraries the console shares, by their own paths", () => {
    const scanned = transpiler.scan(readFileSync(join(out, "bundle.js"), "utf8"));
    const foreign = scanned.imports.map((found) => found.path).filter((path) => !CONSOLE_SHARED_LIBRARIES.some((name) => path === name || path.startsWith(`${name}/`)));

    expect(scanned.exports).toContain("default");
    expect(foreign).toEqual([]);
  });

  test("names the layer it was built on: the release the bundle's own install takes, never a source build", () => {
    const { layer } = JSON.parse(readFileSync(join(out, "layer.json"), "utf8")) as { layer: string };

    expect(layer).toBe(LAYER_VERSION);
    expect(layer).not.toContain("+source");
  });

  test("carries each locale's pack under its code, every key under the bundle's namespace", () => {
    const words = JSON.parse(readFileSync(join(out, "words.json"), "utf8")) as Record<string, Record<string, unknown>>;

    expect(Object.keys(words).sort()).toEqual(["en", "ko"]);
    expect(Object.values(words).map((pack) => Object.keys(pack))).toEqual([["artifact"], ["artifact"]]);
  });

  test("has a stylesheet with the screens' own classes", () => {
    expect(readFileSync(join(out, "bundle.css"), "utf8")).toContain(".artifact-paper");
  });
});
