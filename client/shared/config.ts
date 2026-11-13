import hosting from "../design/hosted-page.json";

// The settings the document screens keep for themselves.

// How many documents the list asks for at a time.
export const DOCUMENTS_PAGE_SIZE = 30;

// How long the phrase the list is searched by may be. The service reads no more of it.
export const DOCUMENTS_PHRASE_LIMIT = 200;

// The spans a share link may be made to live, each with the key its name is read by in the bundle's
// own words, and how long it lasts in milliseconds; one span has no end, and is ended by taking the
// link away.
export const SHARE_SPANS = [
  { value: "1h", label: "artifact.label.span_1h", ms: 60 * 60 * 1000 },
  { value: "6h", label: "artifact.label.span_6h", ms: 6 * 60 * 60 * 1000 },
  { value: "24h", label: "artifact.label.span_24h", ms: 24 * 60 * 60 * 1000 },
  { value: "7d", label: "artifact.label.span_7d", ms: 7 * 24 * 60 * 60 * 1000 },
  { value: "30d", label: "artifact.label.span_30d", ms: 30 * 24 * 60 * 60 * 1000 },
  { value: "forever", label: "artifact.label.span_forever", ms: null }
] as const;

// The sandbox a kept page runs in, which the screen's frame gives the page it stands as the service's
// policy gives it (`docs/artifact.md`, "An HTML page").
export const HOSTED_PAGE_SANDBOX: string = hosting.sandbox;
