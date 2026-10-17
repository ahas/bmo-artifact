import { describe, expect, test } from "bun:test";

import { shareHeadTags } from "../../shared/share-head";

const ROOT = "https://console.example.com/base/";
const LINK = `${ROOT}share/token`;

function content(tags: ReturnType<typeof shareHeadTags>, key: string): string | undefined {
  return tags.meta.find((tag) => tag.name === key || tag.property === key)?.content;
}

describe("the head a share link previews by", () => {
  test("names the document, its words, its picture and the link itself, and keeps out of indexes", () => {
    const tags = shareHeadTags({ title: "Document", description: "First paragraph", image: "api/share/token/picture", declared: null }, ROOT, LINK);

    expect(tags.title).toBe("Document");
    expect(tags.canonical).toBe(LINK);
    expect(content(tags, "og:url")).toBe(LINK);
    expect(content(tags, "og:image")).toBe(`${ROOT}api/share/token/picture`);
    expect(content(tags, "twitter:card")).toBe("summary_large_image");
    expect(content(tags, "description")).toBe("First paragraph");
    expect(content(tags, "robots")).toBe("noindex, nofollow");
  });

  test("names no picture and asks for the small card where the document has none, and no empty description", () => {
    const tags = shareHeadTags({ title: "Document", description: "", image: null, declared: null }, ROOT, LINK);

    expect(content(tags, "og:image")).toBeUndefined();
    expect(content(tags, "twitter:image")).toBeUndefined();
    expect(content(tags, "twitter:card")).toBe("summary");
    expect(tags.meta.some((tag) => (tag.name ?? tag.property ?? "").endsWith("description"))).toBe(false);
  });

  test("lets a page's own tags stand over the link's, as the page would read alone", () => {
    const declared = { title: "Its own title", meta: [{ property: "og:image", content: "https://example.com/c.png" }], canonical: "https://example.com/c" };
    const tags = shareHeadTags({ title: "Document", description: "First paragraph", image: null, declared }, ROOT, LINK);

    expect(tags.title).toBe("Its own title");
    expect(tags.canonical).toBe("https://example.com/c");
    expect(tags.meta.filter((tag) => tag.property === "og:image")).toEqual([{ property: "og:image", content: "https://example.com/c.png" }]);
    expect(content(tags, "og:title")).toBe("Document");
  });

  // A page that declares a head writes some of its tags and not others; what it leaves out is the
  // link's.
  test("keeps the link's title and address where a page declares neither", () => {
    const tags = shareHeadTags({ title: "Document", description: "", image: null, declared: { title: null, meta: [], canonical: null } }, ROOT, LINK);

    expect(tags.title).toBe("Document");
    expect(tags.canonical).toBe(LINK);
  });
});
