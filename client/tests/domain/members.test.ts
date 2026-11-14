import { afterEach, describe, expect, test } from "bun:test";

import { memberRoad, shareFailure } from "../../app/domain/members";

import type { PeopleDirectory } from "../../app/domain/members";
import type { ArtifactMemberSharePage } from "../../shared/types";

// The road one document's members go by in the share dialog (`docs/artifact.md`, "Member shares"): who it is
// shared with read from the service and named by the console's people directory, and every change sent to the
// service under the member's account key.

interface Sent {
  address: string;
  method: string;
  query?: Record<string, unknown>;
  body?: unknown;
}

const NUXT = ["useAppConfig", "useRuntimeConfig", "$fetch"] as const;

// What the console gives a bundle's module: its configuration naming the relay, and a fetcher that records each
// call and answers the pages handed it in turn.
function relayed(pages: ArtifactMemberSharePage[]): Sent[] {
  const sent: Sent[] = [];

  Object.assign(globalThis, {
    useAppConfig: () => ({ bmoUi: { relayBase: "/relay" } }),
    useRuntimeConfig: () => ({ app: { baseURL: "/" } }),
    $fetch: (address: string, options: { method?: string; query?: Record<string, unknown>; body?: unknown } = {}) => {
      sent.push({ address, method: options.method ?? "GET", query: options.query, body: options.body });

      return Promise.resolve(options.method === undefined ? pages.shift() : undefined);
    }
  });

  return sent;
}

afterEach(() => {
  for (const name of NUXT) Reflect.deleteProperty(globalThis, name);
});

const directory: PeopleDirectory = {
  search: (words) => Promise.resolve([{ user: "7", name: `Found ${words}`, organization: "Lab", jobTitle: "Engineer" }]),
  person: (user) => Promise.resolve(user === "7" ? { user, name: "Tester", organization: "Lab", jobTitle: "Engineer" } : null),
  me: () => Promise.resolve(null)
};

function member(id: number, user: string) {
  return { id, documentId: 3, user, permission: "read" as const, sharedAt: id };
}

describe("a document's members", () => {
  test("are read page after page and named by the directory, a member it does not answer standing as the key", async () => {
    const sent = relayed([
      { items: [member(1, "7")], more: true },
      { items: [member(2, "8")], more: false }
    ]);

    const members = await memberRoad(3, directory).read();

    expect(members).toEqual([
      { person: { user: "7", name: "Tester", organization: "Lab", jobTitle: "Engineer" }, permission: "read" },
      { person: { user: "8", name: "8", organization: "", jobTitle: "" }, permission: "read" }
    ]);
    expect(sent.map((call) => [call.address, call.query?.after])).toEqual([
      ["/relay/artifact/documents/3/members", undefined],
      ["/relay/artifact/documents/3/members", 1]
    ]);
  });

  test("are found through the directory, and shared with, changed and taken off at the service by their keys", async () => {
    const sent = relayed([]);
    const road = memberRoad(3, directory);

    expect((await road.search("te")).map((person) => person.name)).toEqual(["Found te"]);

    await road.share(["7", "a/b"], "read");
    await road.change("7", "read");
    await road.remove("a/b");

    expect(sent).toEqual([
      { address: "/relay/artifact/documents/3/members", method: "POST", query: undefined, body: { users: ["7", "a/b"], permission: "read" } },
      { address: "/relay/artifact/documents/3/members/7", method: "PATCH", query: undefined, body: { permission: "read" } },
      { address: "/relay/artifact/documents/3/members/a%2Fb", method: "DELETE", query: undefined, body: undefined }
    ]);
  });

  test("are granted read alone, the one permission the service keeps", () => {
    expect(memberRoad(3, directory).permissions).toEqual(["read"]);
  });
});

describe("a refused share", () => {
  const t = (key: string) => `[${key}]`;

  test("of a permission other than read says in the reader's words that a document is shared to read only", () => {
    expect(shareFailure(t)({ data: { code: "read-only", error: "the permission is read" } }, "fallback")).toBe("[artifact.error.member_read_only]");
  });

  test("of any other kind reads as bmo-ui reads it", () => {
    expect(shareFailure(t)({ data: { code: "forbidden", message: "not yours" } }, "fallback")).toBe("not yours");
    expect(shareFailure(t)(new Error("down"), "fallback")).toBe("fallback");
  });
});
