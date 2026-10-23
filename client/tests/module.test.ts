import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// The browser module a console loads (bmo-ui's `docs/console-bundles.md`, "What a bundle is";
// `docs/layout.md`, "The console bundle"), built as `bun run module` builds it into a folder of the
// test's own: one module whose default export is the screens' routes, importing nothing but the
// libraries the console shares, with its stylesheet and its words beside it.

const ROOT = resolve(import.meta.dir, "..");
const SHARED = ["vue", "vue-router", "vue-i18n", "nuxt/app", "@nuxt/ui", "bmo-ui"];
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
  test("is one module, its stylesheet and its words", () => {
    expect(readdirSync(out).sort()).toEqual(["bundle.css", "bundle.js", "words.json"]);
  });

  test("exports a default and imports only the libraries the console shares, by their own paths", () => {
    const scanned = transpiler.scan(readFileSync(join(out, "bundle.js"), "utf8"));
    const foreign = scanned.imports.map((found) => found.path).filter((path) => !SHARED.some((name) => path === name || path.startsWith(`${name}/`)));

    expect(scanned.exports).toContain("default");
    expect(foreign).toEqual([]);
  });

  test("carries each locale's pack under its code, every key under the bundle's namespace", () => {
    const words = JSON.parse(readFileSync(join(out, "words.json"), "utf8")) as Record<string, Record<string, unknown>>;

    expect(Object.keys(words).sort()).toEqual(["en", "ko"]);
    expect(Object.values(words).map((pack) => Object.keys(pack))).toEqual([["artifact"], ["artifact"]]);
  });

  test("has a stylesheet with the screens' own classes", () => {
    expect(readFileSync(join(out, "bundle.css"), "utf8")).toContain(".document-paper");
  });
});
