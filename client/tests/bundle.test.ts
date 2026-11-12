import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { WORDING_KEYS } from "bmo-ui/shared/wording-keys";

// The bundle as a console attaches it (bmo-ui's `docs/console-bundles.md`; `docs/layout.md`, "The console
// bundle"): the declaration a console building it in reads names this service, its components carry its
// prefix, its pages stand under its screen path, and every word its screens ask for is in each of its
// packs under its namespace, or is one the console answers for bmo-ui. The service's own test holds the
// declaration to the manifest the service answers.

interface Declaration {
  name: string;
  serviceName: string;
  screenPath: string;
  formerScreenPaths: string[];
  languagePack: string;
}

const ROOT = resolve(import.meta.dir, "..");
const DECLARATION_FILE = "console-bundle.json";
const declaration = JSON.parse(readFileSync(join(ROOT, DECLARATION_FILE), "utf8")) as Declaration;

function filesUnder(dir: string): string[] {
  return readdirSync(join(ROOT, dir), { recursive: true, encoding: "utf8" }).map((one) => join(dir, one));
}

// Every key a screen asks for by its words, as the files ask for them.
function keysAsked(): Set<string> {
  const asked = new Set<string>();
  const sources = [...filesUnder("app"), ...filesUnder("shared")].filter((path) => /\.(?:vue|ts)$/.test(path));

  for (const path of sources) {
    const text = readFileSync(join(ROOT, path), "utf8");

    for (const found of text.matchAll(/(?:\$t|\bt|speak)\(\s*["'`]([a-z_]+(?:\.[a-z_]+)+)["'`]/g)) {
      asked.add(found[1]!);
    }
  }

  return asked;
}

// Every key a pack holds, its groups joined by dots.
function keysHeld(value: unknown, at = ""): string[] {
  if (typeof value !== "object" || value === null) return [at];

  return Object.entries(value).flatMap(([name, inner]) => keysHeld(inner, at === "" ? name : `${at}.${name}`));
}

describe("the bundle's declaration", () => {
  test("names this service, its screen path and its words", () => {
    expect(declaration).toMatchObject({ name: "artifact", serviceName: "artifact", screenPath: "/artifact", formerScreenPaths: ["/archive"], languagePack: "i18n" });
  });

  test("is carried by the package with the words it names", () => {
    const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as { files: string[] };

    expect(manifest.files).toEqual(expect.arrayContaining([DECLARATION_FILE, declaration.languagePack]));
  });

  test("has every component named with its prefix and every page under its screen path", () => {
    expect(filesUnder("app/components").filter((name) => !name.startsWith(join("app", "components", "Artifact")))).toEqual([]);
    expect(filesUnder("app/pages").filter((name) => name.endsWith(".vue") && !name.startsWith(join("app", "pages", declaration.screenPath.slice(1))))).toEqual([]);
  });
});

describe("the bundle's words", () => {
  const packs = readdirSync(join(ROOT, declaration.languagePack)).filter((name) => name.endsWith(".json"));
  const answered = new Set<string>(WORDING_KEYS);

  test("stand in a pack for each locale, every pack holding the same keys under the bundle's namespace", () => {
    expect(packs.sort()).toEqual(["en.json", "ko.json"]);

    const held = packs.map((name) => keysHeld(JSON.parse(readFileSync(join(ROOT, declaration.languagePack, name), "utf8"))).sort());

    expect(held[0]).toEqual(held[1]!);
    expect(held[0]!.filter((key) => !key.startsWith(`${declaration.name}.`))).toEqual([]);
  });

  test("answer every key a screen asks for, beside the ones the console answers for bmo-ui", () => {
    const held = new Set(keysHeld(JSON.parse(readFileSync(join(ROOT, declaration.languagePack, "en.json"), "utf8"))));
    const missing = [...keysAsked()].filter((key) => !held.has(key) && !answered.has(key));

    expect(missing).toEqual([]);
  });
});
