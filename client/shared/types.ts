// What this repository's read API and share API answer, as the screen reads it (`docs/api.md`,
// "Documents (HTTP)" and "Share links (HTTP)"). Times are epoch seconds.

// A place in a document's record: a source row, or a picture or a link of one finding.
export type SearchRef = string;

// That a document is out on a link, and when the link stops opening: null where it never does.
export interface SearchShareMark {
  expiresAt: number | null;
}

// One document as the list reads it.
export interface SearchSummary {
  id: number;
  chatId: number;
  // The room's name as the room's own unit gives it, or null where it named none.
  room: string | null;
  query: string;
  startedDepth: string;
  endedDepth: string | null;
  status: string;
  startedAt: number;
  finishedAt: number | null;
  findings: number;
  sources: number;
  title: string | null;
  topic: string | null;
  keywords: string[];
  rule: string | null;
  options: Record<string, string>;
  kind: string;
  author: string;
  documented: boolean;
  shared: SearchShareMark | null;
  // The body's words around the first hit, on a searched list alone; null where only the head said it.
  excerpt?: string | null;
}

export interface SearchPage {
  items: SearchSummary[];
  hasMore: boolean;
}

// A property a document's head may show.
export type HeadProperty = "question" | "request" | "topic" | "description" | "keywords" | "rule" | "options" | "depth" | "source";

// One kind of the catalogue: the key its label is read by in the language pack, whether it opens with
// a summary, whether it is the page it keeps, whether a search may write it, and what its head shows.
export interface ArchiveKind {
  id: string;
  labelKey: string;
  summary: boolean;
  hosted: boolean;
  searched: boolean;
  head: HeadProperty[];
}

export interface ArchiveKinds {
  kinds: ArchiveKind[];
}

export type OutlineLevel = "part" | "chapter" | "section";

export interface OutlineEntry {
  slug: string;
  text: string;
  depth: number;
  level: OutlineLevel | null;
}

// A document's contents, and the marks each finding's headings are stamped with, by the finding's place.
export interface ArchiveOutline {
  entries: OutlineEntry[];
  marks: Record<string, string[]>;
}

export interface SearchSourceView {
  ref: SearchRef;
  url: string;
  title: string | null;
  supports: string | null;
  mark: string | null;
}

export interface SearchImageView {
  ref: SearchRef;
  url: string;
  caption: string | null;
}

export interface SearchLinkView {
  ref: SearchRef;
  url: string;
  label: string | null;
}

export interface SearchFindingView {
  place: number;
  runId: number | null;
  branch: string | null;
  text: string;
  // The name it gave itself, and what it is called as a line of its own.
  name: string | null;
  title: string;
  // Its text split where its long explanation begins.
  open: string;
  folded: string | null;
  images: SearchImageView[];
  links: SearchLinkView[];
  sources: SearchSourceView[];
}

export type AttachmentMedia = "image" | "audio" | "video" | "text" | "page" | "file";

export interface ArchiveAttachmentView {
  id: number;
  media: AttachmentMedia;
  type: string;
  bytes: number;
  name: string;
}

// One document read whole on the console.
export interface SearchDetail {
  search: SearchSummary & { runs: number[]; document: boolean };
  kind: ArchiveKind;
  outline: ArchiveOutline;
  findings: SearchFindingView[];
  opened: SearchSourceView[];
  attachments: ArchiveAttachmentView[];
}

// One document as somebody holding its link reads it.
export interface SharedSearchDetail {
  title: string;
  runCount: number;
  kind: string;
  kindEntry: ArchiveKind;
  outline: ArchiveOutline;
  findings: SearchFindingView[];
  opened: SearchSourceView[];
  attachments: ArchiveAttachmentView[];
}

export interface SearchRefPreview {
  title: string | null;
  thumbnail: boolean;
  favicon: boolean;
}

export interface SearchShareView {
  token: string;
  searchId: number;
  createdBy: string;
  createdAt: number;
  expiresAt: number | null;
}

// A tag names what it says either by `name` or, for OpenGraph, by `property`.
export type ShareMetaTag = { name: string; property?: never; content: string } | { name?: never; property: string; content: string };

export interface DeclaredHead {
  title: string | null;
  meta: ShareMetaTag[];
  canonical: string | null;
}

// What a link reads as before it is opened.
export interface SharedDocumentHead {
  title: string;
  description: string;
  image: string | null;
  declared: DeclaredHead | null;
}
