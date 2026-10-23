import { describe, expect, test } from "bun:test";

import { underBase } from "../../app/domain/web-address";

// Where an address the browser loads by itself stands in a console served under a base path
// (`docs/layout.md`, "The console bundle").

describe("a path of the console's own", () => {
  test("stands under the base the console app is served at", () => {
    expect(underBase("/console/", "/api/artifact/documents/1/picture")).toBe("/console/api/artifact/documents/1/picture");
    expect(underBase("/console", "/api/artifact/documents/1/picture")).toBe("/console/api/artifact/documents/1/picture");
  });

  test("is left as it is where it stands there already, or the base is the root", () => {
    expect(underBase("/console/", "/console/api/artifact/documents")).toBe("/console/api/artifact/documents");
    expect(underBase("/", "/api/artifact/documents")).toBe("/api/artifact/documents");
    expect(underBase("", "/artifact/documents")).toBe("/artifact/documents");
  });

  test("takes a path that only begins with the base's letters under the base", () => {
    expect(underBase("/console/", "/consoles/artifact/documents")).toBe("/console/consoles/artifact/documents");
  });

  test("leaves an address that is no path from the root alone", () => {
    expect(underBase("/console/", "https://example.test/api")).toBe("https://example.test/api");
    expect(underBase("/console/", "artifact/documents")).toBe("artifact/documents");
  });
});
