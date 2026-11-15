import { afterEach, describe, expect, test } from "bun:test";

import { unitChoices, unitFailure, unitRoad } from "../../app/domain/units";

import type { ArtifactCompanyUnits, ArtifactUnitSharePage } from "../../shared/types";

// The road one document's organisation units go by in the unit dialog (`docs/artifact.md`, "Unit shares"): the owner's
// company's tree and the units shared with read from the service, and every change sent to it by the unit's number.

interface Sent {
  address: string;
  method: string;
  query?: Record<string, unknown>;
  body?: unknown;
}

const NUXT = ["useAppConfig", "useRuntimeConfig", "$fetch"] as const;

// What the console gives a bundle's module: its configuration naming the relay, and a fetcher that records each
// call and answers the reads handed it in turn.
function relayed(answers: (ArtifactUnitSharePage | ArtifactCompanyUnits)[]): Sent[] {
  const sent: Sent[] = [];

  Object.assign(globalThis, {
    useAppConfig: () => ({ bmoUi: { relayBase: "/relay" } }),
    useRuntimeConfig: () => ({ app: { baseURL: "/" } }),
    $fetch: (address: string, options: { method?: string; query?: Record<string, unknown>; body?: unknown } = {}) => {
      sent.push({ address, method: options.method ?? "GET", query: options.query, body: options.body });

      return Promise.resolve(options.method === undefined ? answers.shift() : undefined);
    }
  });

  return sent;
}

afterEach(() => {
  for (const name of NUXT) Reflect.deleteProperty(globalThis, name);
});

function share(id: number, unit: number) {
  return { id, documentId: 3, unit, sharedAt: id };
}

const TREE = [
  { id: 1, name: "Company", parent: null },
  { id: 2, name: "Team", parent: 1 },
  { id: 3, name: "Part", parent: 2 },
  { id: 4, name: "Beside", parent: 1 }
];

describe("a company's units", () => {
  test("each stand with the units above them, from the company down, in the tree's order", () => {
    expect(unitChoices(TREE)).toEqual([
      { id: 1, name: "Company", above: [] },
      { id: 2, name: "Team", above: ["Company"] },
      { id: 3, name: "Part", above: ["Company", "Team"] },
      { id: 4, name: "Beside", above: ["Company"] }
    ]);
  });

  test("a parent the tree does not hold ends the units above", () => {
    expect(unitChoices([{ id: 5, name: "Stray", parent: 9 }])).toEqual([{ id: 5, name: "Stray", above: [] }]);
  });
});

describe("a document's units", () => {
  test("are offered from the owner's company read through the document", async () => {
    const sent = relayed([{ company: 1, units: TREE }]);
    const choices = await unitRoad(3).tree();

    expect(choices.map((choice) => choice.id)).toEqual([1, 2, 3, 4]);
    expect(sent.map((call) => call.address)).toEqual(["/relay/artifact/documents/3/company-units"]);
  });

  test("shared with are read page after page as their numbers", async () => {
    const sent = relayed([
      { items: [share(1, 2)], more: true },
      { items: [share(2, 4)], more: false }
    ]);

    expect(await unitRoad(3).read()).toEqual([2, 4]);
    expect(sent.map((call) => [call.address, call.query?.after])).toEqual([
      ["/relay/artifact/documents/3/units", undefined],
      ["/relay/artifact/documents/3/units", 1]
    ]);
  });

  test("are shared with and taken off at the service by their numbers", async () => {
    const sent = relayed([]);
    const road = unitRoad(3);

    await road.share(2);
    await road.remove(2);

    expect(sent).toEqual([
      { address: "/relay/artifact/documents/3/units", method: "POST", query: undefined, body: { units: [2] } },
      { address: "/relay/artifact/documents/3/units/2", method: "DELETE", query: undefined, body: undefined }
    ]);
  });
});

describe("a refused unit share", () => {
  const t = (key: string) => `[${key}]`;

  test("of a unit of another company says so in the reader's words", () => {
    expect(unitFailure(t)({ data: { code: "other-company", error: "another company" } }, "fallback")).toBe("[artifact.error.unit_other_company]");
  });

  test("of any other kind reads as bmo-ui reads it", () => {
    expect(unitFailure(t)(new Error("down"), "fallback")).toBe("fallback");
  });
});
