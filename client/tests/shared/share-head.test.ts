import { describe, expect, test } from "bun:test";

import { SHARE_ROBOTS, shareHeadTags } from "../../shared/share-head";

// The head a share link's page is sent with (`docs/artifact.md`, "Where a person reads it").

describe("a share link's head", () => {
  test("carries the document's title, description and picture, kept out of every index", () => {
    const tags = shareHeadTags({ title: "Plan", description: "What it is", image: "https://example.com/picture" }, "https://example.com/artifact/share/t");

    expect(tags.title).toBe("Plan");
    expect(tags.canonical).toBe("https://example.com/artifact/share/t");
    expect(tags.meta).toContainEqual(SHARE_ROBOTS);
    expect(tags.meta).toContainEqual({ property: "og:image", content: "https://example.com/picture" });
    expect(tags.meta).toContainEqual({ name: "twitter:card", content: "summary_large_image" });
    expect(tags.meta).toContainEqual({ name: "description", content: "What it is" });
  });

  test("names no picture and no description it does not have, and asks for the small card", () => {
    const tags = shareHeadTags({ title: "Plan", description: null, image: null }, "https://example.com/artifact/share/t");

    expect(tags.meta.some((tag) => tag.property === "og:image" || tag.name === "description")).toBe(false);
    expect(tags.meta).toContainEqual({ name: "twitter:card", content: "summary" });
  });
});
