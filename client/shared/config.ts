import hosting from "../design/hosted-page.json";

// The settings the archive screen keeps for itself.

// How many documents the list asks for at a time.
export const SEARCHES_PAGE_SIZE = 30;

// How long the phrase the list is searched by may be. The read API reads no more of it.
export const SEARCHES_PHRASE_LIMIT = 200;

// How much of a row's one-line text stands before a hit in a searched list, so the hit is still in
// view where a phone cuts the line short.
export const SEARCHES_HIT_LEAD = 8;

// What a document is called: its headline, and its question where it wrote none.
export function searchTitle(search: { title: string | null; query: string }): string {
  return search.title ?? search.query;
}

// The spans a share link may be made to live, by the key the share API takes, each with the key its
// name is read by in the language pack. The server works the expiry out of the key, so the screen
// never hands over a time of its own; one span has no end, and is ended by taking the link away.
export const SHARE_SPANS = [
  { key: "1h", label: "common.label.span_1h" },
  { key: "6h", label: "common.label.span_6h" },
  { key: "24h", label: "common.label.span_24h" },
  { key: "7d", label: "common.label.span_7d" },
  { key: "30d", label: "common.label.span_30d" },
  { key: "forever", label: "common.label.forever" }
] as const;

export type ShareSpanKey = (typeof SHARE_SPANS)[number]["key"];

// The sandbox a kept page runs in, which the console's frame gives the page it stands as the server's
// policy gives it (`docs/archive.md`, "An HTML page").
export const HOSTED_PAGE_SANDBOX: string = hosting.sandbox;
