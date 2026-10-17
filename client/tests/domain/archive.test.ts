import { describe, expect, test } from "bun:test";

import { matchFields } from "bmo-ui/shared/search";

import {
  archiveKind,
  archiveSearchFields,
  attachmentAddresses,
  depthText,
  findingImages,
  headProperties,
  hostInitial,
  hostOf,
  optionLines,
  pageSubtitle,
  pageTitle,
  refPath,
  roomName,
  soundsFirst,
  sourceGroups,
  statusView,
  subjectLine,
  subjectText,
  textPreview,
  undrawnAttachments
} from "../../app/domain/archive";
import { searchTitle } from "../../shared/config";

import type { Translate } from "bmo-ui/shared/wording";
import type { ArchiveKind, HeadProperty, SearchSummary } from "../../shared/types";

// The words of the language pack are the host's, so the screen is read here by the keys it asks for:
// a key stands for its line, its marks written after it.
const t: Translate = (key, params) =>
  params === undefined
    ? key
    : `${key}(${Object.entries(params)
        .map(([name, value]) => `${name}=${value}`)
        .join(",")})`;

const head = {
  title: "Silver rises",
  query: "silver price",
  topic: "silver market",
  keywords: ["silver", "spot"],
  rule: null,
  options: { shape: "brief", source: "https://a.example/post" },
  startedDepth: "basic",
  endedDepth: "thorough"
};

// What a document's head shows is its kind's to say, in the order the kind names it, and a property
// with nothing to say stands nowhere.
describe("the head of a document", () => {
  test("shows what its kind names, in that order", () => {
    expect(headProperties(t, ["question", "topic", "keywords", "rule", "options", "depth"], head)).toEqual([
      { label: "archive.label.head_question", value: "silver price" },
      { label: "archive.label.head_topic", value: "silver market" },
      { label: "archive.label.head_keywords", value: "silver, spot" },
      { label: "archive.label.option_shape", value: "archive.label.value_brief" },
      { label: "archive.label.head_depth", value: "common.label.depth_basic → common.label.depth_thorough" }
    ]);
    expect(headProperties(t, ["source"], head)).toEqual([{ label: "archive.label.option_source", value: "https://a.example/post", mono: true }]);
    expect(headProperties(t, ["request"], head)).toEqual([{ label: "archive.label.head_request", value: "silver price" }]);
  });

  test("the question stands only where the headline is not the question itself", () => {
    expect(headProperties(t, ["question"], { ...head, title: null })).toEqual([]);
  });

  // The catalogue is the read API's; every property it may name for a head is one the screen draws.
  test("every property a kind's head may name is one the screen can draw", () => {
    const every: HeadProperty[] = ["question", "request", "topic", "description", "keywords", "rule", "options", "depth", "source"];

    expect(() => headProperties(t, every, head)).not.toThrow();
  });
});

describe("the kind of a document", () => {
  const kinds: ArchiveKind[] = [
    { id: "search", labelKey: "archive.kind.search", summary: true, hosted: false, searched: true, head: ["question"] },
    { id: "page", labelKey: "archive.kind.page", summary: false, hosted: true, searched: false, head: ["description"] }
  ];

  test("is the catalogue's entry for its word, its first where the catalogue does not know the word", () => {
    expect(archiveKind(kinds, "page")?.id).toBe("page");
    expect(archiveKind(kinds, "unheard")?.id).toBe("search");
    expect(archiveKind(kinds, null)?.id).toBe("search");
  });

  test("is none while the catalogue is unread", () => {
    expect(archiveKind([], "page")).toBeNull();
  });
});

// A text attachment shows its start. A ranged read may stop inside a character and a long file goes on
// past what is shown; neither may read as the whole of it.
describe("the preview of an attached text", () => {
  const encoded = (text: string): Uint8Array => new TextEncoder().encode(text);

  test("is the text as it came where it came whole", () => {
    expect(textPreview(encoded("meeting notes\n- decided"), "bytes 0-22/23")).toBe("meeting notes\n- decided");
    expect(textPreview(encoded("a short note"), null)).toBe("a short note");
  });

  test("drops a character cut in two and ends in an ellipsis where the file goes on", () => {
    // Each euro sign is three bytes, so seven bytes hold two whole and one cut.
    const bytes = encoded("€€€").subarray(0, 7);

    expect(textPreview(bytes, "bytes 0-6/9000")).toBe("€€\n…");
  });

  test("stops at forty lines", () => {
    const lines = Array.from({ length: 50 }, (_, at) => `line ${at + 1}`).join("\n");
    const shown = textPreview(encoded(lines), null).split("\n");

    expect(shown).toHaveLength(41);
    expect(shown.at(-2)).toBe("line 40");
    expect(shown.at(-1)).toBe("…");
  });
});

describe("the files a body names", () => {
  const attachments = [
    { id: 7, media: "page" as const, name: "gallery.html", drawn: false },
    { id: 8, media: "image" as const, name: "palette-01.webp", drawn: true },
    { id: 9, media: "image" as const, name: "receipt photo.png", drawn: true },
    { id: 10, media: "text" as const, name: "note.txt", drawn: false }
  ];

  test("are served under the document's own road by the name they are kept under", () => {
    const files = attachmentAddresses("/base/api/share/t/attachments", attachments);

    expect(files("gallery.html")).toBe("/base/api/share/t/attachments/7");
    expect(files("receipt%20photo.png")).toBe("/base/api/share/t/attachments/9");
    expect(files("https://example.com/gallery.html")).toBeNull();
    expect(files("other.html")).toBeNull();
  });

  // Which pictures the body draws is the read API's to say; the screen only leaves those out.
  test("a picture the body draws does not stand again among the attachments, and the rest keep their order", () => {
    expect(undrawnAttachments(attachments).map((attachment) => attachment.id)).toEqual([7, 10]);
    expect(undrawnAttachments(attachments.map((one) => ({ ...one, drawn: false }))).map((attachment) => attachment.id)).toEqual([7, 8, 9, 10]);
  });

  test("a sound is set apart to stand at the head, the rest keeping their order after the body", () => {
    const kept = [...attachments, { id: 11, media: "audio" as const, name: "briefing.ogg", drawn: false }];
    const { sounds, others } = soundsFirst(kept);

    expect(sounds.map((one) => one.id)).toEqual([11]);
    expect(others.map((one) => one.id)).toEqual([7, 8, 9, 10]);
  });
});

describe("what a search is called", () => {
  test("its depth, stepped up or not", () => {
    expect(depthText(t, "basic", null)).toBe("common.label.depth_basic");
    expect(depthText(t, "basic", "basic")).toBe("common.label.depth_basic");
    expect(depthText(t, "basic", "exhaustive")).toBe("common.label.depth_basic → common.label.depth_exhaustive");
    expect(depthText(t, "deep", null)).toBe("deep");
  });

  test("its state, in words and an icon, and an unknown word as it stands", () => {
    expect(statusView(t, "open")).toMatchObject({ label: "archive.status.open", color: "info" });
    expect(statusView(t, "done")).toMatchObject({ label: "archive.status.done", color: "success" });
    expect(statusView(t, "cut-short")).toMatchObject({ label: "archive.status.cut_short" });
    expect(statusView(t, "failed")).toMatchObject({ label: "archive.status.failed", color: "error" });
    expect(statusView(t, "weird")).toEqual({ label: "weird", color: "neutral", icon: "i-material-symbols-radio-button-checked-outline" });
  });
});

describe("what a page is called", () => {
  test("falls back from the page's own title to the record's to the site", () => {
    const url = "https://www.aoostar.com/products/gem12";

    expect(hostOf(url)).toBe("aoostar.com");
    expect(hostInitial(url)).toBe("A");
    expect(hostInitial("https://9gag.com/")).toBe("9");
    expect(pageTitle(url, "recorded title", { title: "page title", thumbnail: false, favicon: false })).toBe("page title");
    expect(pageTitle(url, "recorded title", { title: null, thumbnail: false, favicon: false })).toBe("recorded title");
    expect(pageTitle(url, null, null)).toBe("aoostar.com");
  });

  test("is cited for what the source line says, else by its site", () => {
    expect(pageSubtitle("https://a.example/x", "price")).toBe("price");
    expect(pageSubtitle("https://a.example/x", null)).toBe("a.example");
    expect(pageSubtitle("https://a.example/x", "  ")).toBe("a.example");
  });

  test("a search by its headline, its question where it wrote none", () => {
    expect(searchTitle({ title: "Silver rises", query: "silver price" })).toBe("Silver rises");
    expect(searchTitle({ title: null, query: "silver price" })).toBe("silver price");
    expect(subjectText({ topic: "silver market", keywords: ["silver", "spot"], rule: null })).toBe("silver market · #silver #spot");
    expect(subjectText({ topic: null, keywords: [], rule: "rabbit rule" })).toBe("rabbit rule");
    expect(subjectText({ topic: null, keywords: [], rule: null })).toBe("");
    expect(optionLines(t, { shape: "brief", region: "domestic", period: "one week" })).toEqual([
      { label: "archive.label.option_shape", value: "archive.label.value_brief" },
      { label: "archive.label.option_region", value: "archive.label.value_domestic" },
      { label: "archive.label.option_period", value: "one week" }
    ]);
  });

  test("a room by the name its own unit gave it, else by its number", () => {
    expect(roomName({ room: "Alpha Room", chatId: 4 })).toBe("Alpha Room");
    expect(roomName({ room: null, chatId: 4 })).toBe("4");
  });
});

describe("where the console serves a ref", () => {
  test("names the ref under the road it was given, never the address", () => {
    expect(refPath("/api/archive/4/refs", "s116", "preview")).toBe("/api/archive/4/refs/s116/preview");
    expect(refPath("/api/archive/4/refs", "i1.2", "image")).toBe("/api/archive/4/refs/i1.2/image");
    expect(refPath("/api/archive/4/refs", "l1.1", "image", "favicon")).toBe("/api/archive/4/refs/l1.1/image?as=favicon");
  });

  // The document's parts are handed the road rather than a document, which is the whole of what lets
  // a share page — whose reader has no document number and must never be given one — put the same
  // parts on the screen.
  test("a road named by a token serves the same refs, with no document anywhere in it", () => {
    const path = refPath("/api/share/tok-en_1/refs", "s116", "preview");

    expect(path).toBe("/api/share/tok-en_1/refs/s116/preview");
    expect(path).not.toContain("archive");
  });

  test("a ref that would leave its road is escaped rather than obeyed", () => {
    expect(refPath("/api/archive/4/refs", "../../7/refs/s1", "preview")).toBe("/api/archive/4/refs/..%2F..%2F7%2Frefs%2Fs1/preview");
  });

  test("answers a finding's own pictures and nothing else", () => {
    const images = findingImages(
      "/api/archive/4/refs",
      { images: [{ ref: "i1.1", url: "https://cdn.example.com/p.jpg?v=1", caption: null }] },
      (path) => `/base${path}`
    );

    expect(images("https://cdn.example.com/p.jpg?v=1")).toBe("/base/api/archive/4/refs/i1.1/image");
    expect(images("https://CDN.example.com/p.jpg?v=1")).toBe("/base/api/archive/4/refs/i1.1/image");
    expect(images("https://cdn.example.com/other.jpg")).toBeNull();
  });

  test("a finding's pictures follow whichever road the page reading it stands on", () => {
    const images = findingImages(
      "/api/share/tok-en_1/refs",
      { images: [{ ref: "i1.1", url: "https://cdn.example.com/p.jpg", caption: null }] },
      (path) => `/base${path}`
    );

    expect(images("https://cdn.example.com/p.jpg")).toBe("/base/api/share/tok-en_1/refs/i1.1/image");
  });
});

describe("the groups of the source list", () => {
  const source = (ref: string, mark: string) => ({ ref, url: `https://example.com/${ref}`, title: null, supports: "this section", mark });

  // A part label says where a finding opens, not what its sources back: the sources of a finding that
  // opens with its summary stand behind the whole of it, so none of them is filed under the summary's
  // label.
  test("files the sources of a finding with no name of its own under the cited sources, never under its part label", () => {
    const groups = sourceGroups(t, [
      { place: 1, name: null, title: "Summary", sources: [source("1", "1")] },
      { place: 2, name: null, title: "A long first sentence with emphasis", sources: [source("2", "2")] }
    ]);

    expect(groups).toEqual([{ key: "cited", title: "archive.title.cited_sources", sources: [source("1", "1"), source("2", "2")] }]);
    expect(groups.map((group) => group.title)).not.toContain("Summary");
  });

  test("keeps a named entry's sources under its own name, after the unnamed ones", () => {
    const groups = sourceGroups(t, [
      { place: 1, name: null, title: "Summary", sources: [source("1", "1")] },
      { place: 2, name: "Eco Whale", title: "Eco Whale", sources: [source("2", "2")] },
      { place: 3, name: "Empty entry", title: "Empty entry", sources: [] },
      { place: 4, name: null, title: "Conclusion", sources: [source("3", "3")] }
    ]);

    expect(groups).toEqual([
      { key: "cited", title: "archive.title.cited_sources", sources: [source("1", "1"), source("3", "3")] },
      { key: "2", title: "Eco Whale", sources: [source("2", "2")] }
    ]);
  });
});

// A row of the searched list: what it shows is marked where it stands, and a hit in nothing it shows —
// the question behind a title, the body — is said under it.
describe("where a row of the list matched", () => {
  const row = (fields: Partial<SearchSummary>): SearchSummary => ({ ...head, excerpt: null, ...fields }) as SearchSummary;
  const matched = (fields: Partial<SearchSummary>, phrase: string) => matchFields(archiveSearchFields(t, row(fields), "Alpha Room"), phrase);

  test("a hit the row shows needs no second line", () => {
    expect(matched({}, head.title)).toEqual({ shown: true, hidden: null });
    expect(matched({}, `#${head.keywords[1]}`)).toEqual({ shown: true, hidden: null });
    expect(matched({}, "alpha")).toEqual({ shown: true, hidden: null });
  });

  test("the question is said under the row only where a title stands in its place", () => {
    expect(matched({}, head.query)).toEqual({ shown: false, hidden: { label: "archive.label.head_question", snippet: head.query } });
    expect(matched({ title: null }, head.query)).toEqual({ shown: true, hidden: null });
  });

  // The topic line is cut at its end, so a hit late in it would be cut away with the rest.
  test("the topic line starts just before its hit, and as it is without one", () => {
    const later = { topic: `${"a".repeat(80)} needle`, keywords: [], rule: null };

    expect(subjectLine(later, "needle")).toBe(`…${"a".repeat(7)} needle`);
    expect(subjectLine(later, "missing")).toBe(subjectText(later));
    expect(subjectLine(later, "")).toBe(subjectText(later));
  });

  test("the body is said by the words the server handed over around the hit", () => {
    expect(matched({ excerpt: "…prices rose sharply…" }, "Sharply")).toEqual({ shown: false, hidden: { label: "archive.label.body", snippet: "…prices rose sharply…" } });
    expect(matched({}, "sharply")).toBeNull();
  });
});
