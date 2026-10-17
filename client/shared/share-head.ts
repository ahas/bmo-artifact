import type { ShareMetaTag, SharedDocumentHead } from "./types";

// The head a share link's page is sent with (`docs/archive.md`, "The share page"): one list for every
// kind of document, so a link reads the same in a chat app's preview whichever kind it opens. A page
// document's own tags stand over the link's, as they would were the page read alone.

export type { ShareMetaTag } from "./types";

// A link is handed to people rather than published, so it is kept out of every index — a link that
// opens nothing as much as one that opens its document.
export const SHARE_ROBOTS: ShareMetaTag = { name: "robots", content: "noindex, nofollow" };

export interface ShareHeadTags {
  title: string;
  meta: ShareMetaTag[];
  canonical: string;
}

// The tags of one link. `root` is the console's root as the link was reached, ending in a slash, and
// `link` the address it was opened at, which is its canonical address unless the page names its own.
export function shareHeadTags(head: SharedDocumentHead, root: string, link: string): ShareHeadTags {
  const own = head.declared?.meta ?? [];
  const declared = new Set(own.map(tagName));
  const tags = linkTags(head, root, link).filter((tag) => !declared.has(tagName(tag)));

  return { title: head.declared?.title ?? head.title, canonical: head.declared?.canonical ?? link, meta: [...tags, ...own] };
}

function tagName(tag: ShareMetaTag): string {
  return (tag.name ?? tag.property ?? "").toLowerCase();
}

function linkTags(head: SharedDocumentHead, root: string, link: string): ShareMetaTag[] {
  // A document with no picture of its own names none, rather than one standing in for it.
  const image = head.image === null ? null : `${root}${head.image}`;
  const described = head.description !== "";

  return [
    SHARE_ROBOTS,
    ...(described ? [{ name: "description", content: head.description }] : []),
    { property: "og:type", content: "article" },
    { property: "og:title", content: head.title },
    ...(described ? [{ property: "og:description", content: head.description }] : []),
    { property: "og:url", content: link },
    ...(image === null ? [] : [{ property: "og:image", content: image }]),
    { name: "twitter:card", content: image === null ? "summary" : "summary_large_image" },
    { name: "twitter:title", content: head.title },
    ...(described ? [{ name: "twitter:description", content: head.description }] : []),
    ...(image === null ? [] : [{ name: "twitter:image", content: image }])
  ];
}
