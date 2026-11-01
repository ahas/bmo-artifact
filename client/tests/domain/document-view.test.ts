import { describe, expect, test } from "bun:test";

import { categoryFilter, categoryOf, contentsOf, drawnNames, fileAddresses, filesAround, headSounds, hostInitial, hostOf, keptNames, kindOf, languageOf, numberText, pendingOf, textPreview } from "../../app/domain/document-view";

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

describe("a document's category, number and progress", () => {
  test("is its category label's word, and none where the label is missing or blank", () => {
    expect(categoryOf({ labels: { category: "security-audit", topic: "x" } })).toBe("security-audit");
    expect(categoryOf({ labels: { category: "  " } })).toBeNull();
    expect(categoryOf({ labels: { report: "security-audit-2026-11-01" } })).toBeNull();
  });

  test("is in progress while its pending label names what it lacks, and done without one", () => {
    expect(pendingOf({ labels: { pending: "korean, narration-ko,,review" } })).toEqual(["korean", "narration-ko", "review"]);
    expect(pendingOf({ labels: { pending: " , " } })).toBeNull();
    expect(pendingOf({ labels: {} })).toBeNull();
  });

  test("narrows the list by the category label, and the number reads with its mark", () => {
    expect(categoryFilter("search:exhaustive")).toBe("category:search:exhaustive");
    expect(numberText(463)).toBe("#463");
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

describe("the listen band", () => {
  test("holds a page's head sound, labelled with the language its name carries", () => {
    const files = [attachment(1, "index.html", "page"), attachment(2, "narration-en.ogg", "audio", "head"), attachment(3, "figure-1.png", "image")];

    expect(headSounds(files).map((one) => one.id)).toEqual([2]);
    expect(languageOf(files[1]!)).toBe("English");
  });

  test("lists a narration in two languages as two sounds, the one with no role beside the head sound named as it is", () => {
    const files = [attachment(1, "narration-ko.ogg", "audio", "head"), attachment(2, "narration-en.ogg", "audio"), attachment(3, "song-fr.ogg", "audio")];
    const sounds = headSounds(files);
    const labels = sounds.map(languageOf);

    expect(sounds.map((one) => one.id)).toEqual([1, 2]);
    expect(labels[0]).not.toBeNull();
    expect(labels[0]).not.toBe("English");
    expect(labels[1]).toBe("English");
    expect(filesAround(files, new Set()).after.map((one) => one.id)).toEqual([3]);
  });

  test("is empty for a document with no sound marked for its head", () => {
    const files = [attachment(1, "narration-en.ogg", "audio"), attachment(2, "voice.mp3", "audio"), attachment(3, "cover-en.png", "image", "head")];

    expect(headSounds(files)).toEqual([]);
  });

  test("labels a sound only by a language its name carries", () => {
    expect(languageOf(attachment(1, "voice.mp3", "audio", "head"))).toBeNull();
    expect(languageOf(attachment(2, "take-1.ogg", "audio", "head"))).toBeNull();
    expect(languageOf(attachment(3, "take-zz.ogg", "audio", "head"))).toBeNull();
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
