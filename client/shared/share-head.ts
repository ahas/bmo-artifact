// The head a share link's page is sent with (`docs/artifact.md`, "Where a person reads it"): one list for
// every kind of document, so a link reads the same in a chat app's preview whichever kind it opens.

// A tag names what it says either by `name` or, for OpenGraph, by `property`.
export type ShareMetaTag = { name: string; property?: never; content: string } | { name?: never; property: string; content: string };

// A link is handed to people rather than published, so it is kept out of every index — a link that
// opens nothing as much as one that opens its document.
export const SHARE_ROBOTS: ShareMetaTag = { name: "robots", content: "noindex, nofollow" };

/** What a link reads as before it is opened: the document's title, its description and its picture. */
export interface SharedHead {
  title: string;
  description: string | null;
  // The cover picture's whole address, null for a document with none.
  image: string | null;
}

export interface ShareHeadTags {
  title: string;
  meta: ShareMetaTag[];
  canonical: string;
}

/** The tags of one link, opened at `link`, which is its canonical address. */
export function shareHeadTags(head: SharedHead, link: string): ShareHeadTags {
  const described = head.description !== null && head.description !== "";
  const description = head.description ?? "";

  return {
    title: head.title,
    canonical: link,
    meta: [
      SHARE_ROBOTS,
      ...(described ? [{ name: "description", content: description }] : []),
      { property: "og:type", content: "article" },
      { property: "og:title", content: head.title },
      ...(described ? [{ property: "og:description", content: description }] : []),
      { property: "og:url", content: link },
      ...(head.image === null ? [] : [{ property: "og:image", content: head.image }]),
      { name: "twitter:card", content: head.image === null ? "summary" : "summary_large_image" },
      { name: "twitter:title", content: head.title },
      ...(described ? [{ name: "twitter:description", content: description }] : []),
      ...(head.image === null ? [] : [{ name: "twitter:image", content: head.image }])
    ]
  };
}
