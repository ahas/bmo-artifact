import { describe, expect, test } from "bun:test";
import { createHead, renderSSRHead } from "@unhead/vue/server";

import { SHARE_ROBOTS, previewDescription, shareHeadTags } from "../../shared/share-head";

import type { SharedHead } from "../../shared/share-head";

// The head a share link's page is sent with (`docs/artifact.md`, "Where a person reads it").

const LINK = "https://example.com/artifact/share/t";

function head(fields: Partial<SharedHead> = {}): SharedHead {
  return {
    title: "Plan",
    description: "What it is",
    image: "https://example.com/picture",
    link: LINK,
    siteName: "Console",
    locale: "ko_KR",
    ...fields
  };
}

describe("a share link's head", () => {
  test("carries the document's title, description, picture, address, site and locale, kept out of every index", () => {
    const tags = shareHeadTags(head());

    expect(tags.title).toBe("Plan");
    expect(tags.canonical).toBe(LINK);
    expect(tags.meta).toEqual([
      SHARE_ROBOTS,
      { name: "description", content: "What it is" },
      { property: "og:type", content: "article" },
      { property: "og:site_name", content: "Console" },
      { property: "og:locale", content: "ko_KR" },
      { property: "og:title", content: "Plan" },
      { property: "og:description", content: "What it is" },
      { property: "og:url", content: LINK },
      { property: "og:image", content: "https://example.com/picture" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Plan" },
      { name: "twitter:description", content: "What it is" },
      { name: "twitter:image", content: "https://example.com/picture" }
    ]);
  });

  test("names no picture and no description it does not have, and asks for the small card", () => {
    const tags = shareHeadTags(head({ description: null, image: null }));
    const named = tags.meta.map((tag) => tag.property ?? tag.name);

    for (const absent of ["description", "og:description", "twitter:description", "og:image", "twitter:image"]) {
      expect(named).not.toContain(absent);
    }
    expect(tags.meta).toContainEqual({ name: "twitter:card", content: "summary" });
    expect(tags.meta).toContainEqual({ property: "og:url", content: LINK });
  });

  test("leads the description with the kind's word where the page names one, and says nothing where none is held", () => {
    const joined = (kind: string, said: string) => `${kind} | ${said}`;

    expect(previewDescription("What it found", "Research", joined)).toBe("Research | What it found");
    expect(previewDescription("What it found", null, joined)).toBe("What it found");
    expect(previewDescription(null, "Research", joined)).toBeNull();
    expect(previewDescription("", "Research", joined)).toBeNull();
  });

  test("writes every value as an attribute's text, so no description closes its tag", async () => {
    const said = 'A "quoted" <b>bold</b> & more"><script>alert(1)</script>';
    const tags = shareHeadTags(head({ title: 'Plan "one" <two>', description: said }));
    const unhead = createHead();

    unhead.push({ title: tags.title, meta: tags.meta, link: [{ rel: "canonical", href: tags.canonical }] });

    const { headTags } = await renderSSRHead(unhead);
    const page = new DOMParser().parseFromString(`<!doctype html><html><head>${headTags}</head><body></body></html>`, "text/html");
    const read = (key: string) => page.head.querySelector(`meta[property="${key}"], meta[name="${key}"]`)?.getAttribute("content");

    expect(page.head.querySelectorAll("script")).toHaveLength(0);
    for (const tag of tags.meta) expect(read(tag.property ?? tag.name)).toBe(tag.content);
    expect(read("og:description")).toBe(said);
    expect(read("twitter:description")).toBe(said);
    expect(read("og:title")).toBe('Plan "one" <two>');
    expect(page.title).toBe('Plan "one" <two>');
  });
});
