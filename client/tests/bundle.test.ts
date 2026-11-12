import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { WORDING_KEYS } from "bmo-ui/shared/wording-keys";

// The bundle's sources as the build reads them (`docs/layout.md`, "The console bundle"): its components carry
// its prefix, its pages stand under its screen path, and every word its screens ask for is in each of its
// packs under its namespace, or is one the console answers for bmo-ui. The manifest naming them is the
// service's, held to the contract by the service's own test.

const ROOT = resolve(import.meta.dir, "..");
// The bundle's name, its words' namespace and its pages' folder, and the folder of its words.
const NAMESPACE = "artifact";
const LANGUAGE_PACK = "i18n";

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

describe("the bundle's sources", () => {
  test("have every component named with its prefix and every page under its screen path", () => {
    expect(filesUnder("app/components").filter((name) => !name.startsWith(join("app", "components", "Artifact")))).toEqual([]);
    expect(filesUnder("app/pages").filter((name) => name.endsWith(".vue") && !name.startsWith(join("app", "pages", NAMESPACE)))).toEqual([]);
  });
});

describe("the bundle's words", () => {
  const packs = readdirSync(join(ROOT, LANGUAGE_PACK)).filter((name) => name.endsWith(".json"));
  const answered = new Set<string>(WORDING_KEYS);

  test("stand in a pack for each locale, every pack holding the same keys under the bundle's namespace", () => {
    expect(packs.sort()).toEqual(["en.json", "ko.json"]);

    const held = packs.map((name) => keysHeld(JSON.parse(readFileSync(join(ROOT, LANGUAGE_PACK, name), "utf8"))).sort());

    expect(held[0]).toEqual(held[1]!);
    expect(held[0]!.filter((key) => !key.startsWith(`${NAMESPACE}.`))).toEqual([]);
  });

  test("answer every key a screen asks for, beside the ones the console answers for bmo-ui", () => {
    const held = new Set(keysHeld(JSON.parse(readFileSync(join(ROOT, LANGUAGE_PACK, "en.json"), "utf8"))));
    const missing = [...keysAsked()].filter((key) => !held.has(key) && !answered.has(key));

    expect(missing).toEqual([]);
  });
});
