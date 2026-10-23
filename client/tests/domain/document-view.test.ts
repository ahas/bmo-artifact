import { describe, expect, test } from "bun:test";

import { contentsOf, drawnNames, fileAddresses, filesAround, hostInitial, hostOf, keptNames, kindOf, textPreview } from "../../app/domain/document-view";

import type { ArtifactAttachment } from "../../shared/types";

// How the document screens read what the service answers (`docs/archive.md`, "Kinds" and "Attachments").

function attachment(id: number, name: string, kind: string, role: string | null = null): ArtifactAttachment {
  return { id, documentId: 1, name, mediaType: "application/octet-stream", kind, byteCount: 1, role, createdAt: 0 };
}

describe("a document's kind", () => {
  test("is the catalogue's entry its word names, and none for a word the catalogue does not hold", () => {
    const kinds = [{ name: "record", description: "", hosted: false, contentsDepth: 2 }];

    expect(kindOf(kinds, "record")?.contentsDepth).toBe(2);
    expect(kindOf(kinds, "search")).toBeNull();
  });
});

describe("a document's contents", () => {
  test("count its headings by rank to the kind's depth, each with the mark its heading takes in the text", () => {
    const { entries, slugs } = contentsOf("## One\n\ntext\n\n### Under\n\n#### Deeper\n\n## Two", 2);

    expect(slugs).toEqual(["section-1", "section-2", "section-3", "section-4"]);
    expect(entries).toEqual([
      { slug: "section-1", text: "One", depth: 1 },
      { slug: "section-2", text: "Under", depth: 2 },
      { slug: "section-4", text: "Two", depth: 1 }
    ]);
  });
});

describe("a document's files", () => {
  test("stand where a reader meets them: a head sound first, a picture the body draws by its name nowhere else", () => {
    const files = [attachment(1, "voice.mp3", "audio", "head"), attachment(2, "plan.png", "image"), attachment(3, "notes.txt", "text"), attachment(4, "song.mp3", "audio")];
    const drawn = drawnNames("Before ![the plan](plan%2Epng) after, and [the notes](notes.txt).");
    const split = filesAround(files, drawn);

    expect([...drawn]).toEqual(["plan.png"]);
    expect(split.head.map((one) => one.id)).toEqual([1]);
    expect(split.after.map((one) => one.id)).toEqual([3, 4]);
  });

  test("are found by the names they are kept under", () => {
    const address = fileAddresses([attachment(7, "plan v2.png", "image"), attachment(8, "plan v2.png", "image")], (one) => `/files/${one.id}`);

    expect(address("plan%20v2.png")).toBe("/files/7");
    expect(address("elsewhere.png")).toBeNull();
  });

  test("are found by a path ending in the names they are kept under, and never by a web address", () => {
    const files = [attachment(5, "kept.avif", "image"), attachment(6, "notes.txt", "text")];
    const address = fileAddresses(files, (one) => `/files/${one.id}`);
    const drawn = drawnNames("![a figure](/data/searches/9/images/kept.avif) and ![elsewhere](https://example.com/kept.avif)");

    expect(address("/data/searches/9/images/kept.avif")).toBe("/files/5");
    expect(address("images/kept.avif")).toBe("/files/5");
    expect(address("https://example.com/kept.avif")).toBeNull();
    expect(address("/data/searches/9/images/")).toBeNull();
    expect(keptNames("https://example.com/kept.avif")).toEqual(["https://example.com/kept.avif"]);
    expect(filesAround(files, drawn).after.map((one) => one.id)).toEqual([6]);
  });
});

describe("a page's site and a text's preview", () => {
  test("name the site without its prefix, and its first letter where it has no picture", () => {
    expect(hostOf("https://www.example.com/a")).toBe("example.com");
    expect(hostInitial("https://www.example.com/a")).toBe("E");
    expect(hostOf("not an address")).toBe("not an address");
  });

  test("ends a text read only in part in an ellipsis", () => {
    const bytes = new TextEncoder().encode("one\ntwo");

    expect(textPreview(bytes, null)).toBe("one\ntwo");
    expect(textPreview(bytes, "bytes 0-6/100")).toBe("one\ntwo\n…");
  });
});
