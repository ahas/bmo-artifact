// The head a share link's page is sent with (`docs/artifact.md`, "Where a person reads it"): one list for
// every kind of document, so a link reads the same in a chat app's preview whichever kind it opens.

// A tag names what it says either by `name` or, for OpenGraph, by `property`.
export type ShareMetaTag = { name: string; property?: never; content: string } | { name?: never; property: string; content: string };

// A link is handed to people rather than published, so it is kept out of every index — a link that
// opens nothing as much as one that opens its document.
export const SHARE_ROBOTS: ShareMetaTag = { name: "robots", content: "noindex, nofollow" };

/** What a link reads as before it is opened. */
export interface SharedHead {
  // The document's title, as the page shows it.
  title: string;
  // What the preview says of the document, the kind's word ahead of it where the page names one, null for none.
  description: string | null;
  // The cover picture's whole address, null for a document with none.
  image: string | null;
  // The link's own address, the share address the service's settings name and the token.
  link: string;
  // The name the preview stands under, and the locale of the language the page is shown in.
  siteName: string;
  locale: string;
}

export interface ShareHeadTags {
  title: string;
  meta: ShareMetaTag[];
  canonical: string;
}

/**
 * What the preview says of a document: its description, led by the kind's word where the page names the kind
 * over the title, joined as the language pack's sentence does; null where the document has no description.
 */
export function previewDescription(description: string | null, kind: string | null, joined: (kind: string, description: string) => string): string | null {
  if (description === null || description === "") return null;

  return kind === null || kind === "" ? description : joined(kind, description);
}

/** The tags of one link, its own address the canonical one. */
export function shareHeadTags(head: SharedHead): ShareHeadTags {
  const description = head.description === "" ? null : head.description;
  const image = head.image;

  return {
    title: head.title,
    canonical: head.link,
    meta: [
      SHARE_ROBOTS,
      ...(description === null ? [] : [{ name: "description", content: description }]),
      { property: "og:type", content: "article" },
      { property: "og:site_name", content: head.siteName },
      { property: "og:locale", content: head.locale },
      { property: "og:title", content: head.title },
      ...(description === null ? [] : [{ property: "og:description", content: description }]),
      { property: "og:url", content: head.link },
      ...(image === null ? [] : [{ property: "og:image", content: image }]),
      { name: "twitter:card", content: image === null ? "summary" : "summary_large_image" },
      { name: "twitter:title", content: head.title },
      ...(description === null ? [] : [{ name: "twitter:description", content: description }]),
      ...(image === null ? [] : [{ name: "twitter:image", content: image }])
    ]
  };
}
