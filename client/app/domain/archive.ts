import { hitSnippet, searchHits } from "bmo-ui/shared/search";

import { SEARCHES_HIT_LEAD, searchTitle } from "../../shared/config";

import type { BarProperty } from "bmo-ui/app/domain/tabs";
import type { SearchField } from "bmo-ui/shared/search";
import type { Translate } from "bmo-ui/shared/wording";
import type {
  ArchiveAttachmentView,
  ArchiveKind,
  HeadProperty,
  SearchFindingView,
  SearchRef,
  SearchRefPreview,
  SearchSourceView,
  SearchSummary
} from "../../shared/types";

// How the archive screen says what a document is: its depth, its state, and what one of its pages is
// called when the page itself told the server little or nothing. How a document is outlined, what a
// finding is called and where it folds are the read API's to say, and stand on what it answers.

// The kind a document's word names in the catalogue the read API answers. A word the catalogue does
// not know reads as its first kind, as the server reads it; none stands while the catalogue is unread.
export function archiveKind(kinds: readonly ArchiveKind[], id: string | null | undefined): ArchiveKind | null {
  return kinds.find((kind) => kind.id === id) ?? kinds[0] ?? null;
}

const DEPTHS: Record<string, string> = {
  basic: "common.label.depth_basic",
  thorough: "common.label.depth_thorough",
  exhaustive: "common.label.depth_exhaustive"
};

export function depthLabel(t: Translate, depth: string): string {
  const key = DEPTHS[depth];

  return key === undefined ? depth : t(key);
}

// The depth it started on, and where it got to when it stepped up.
export function depthText(t: Translate, started: string, ended: string | null): string {
  return ended === null || ended === started ? depthLabel(t, started) : `${depthLabel(t, started)} → ${depthLabel(t, ended)}`;
}

export interface StatusView {
  label: string;
  color: "neutral" | "info" | "success" | "warning" | "error";
  icon: string;
}

// A state is said in words and with an icon, never by its colour alone. A word the screen does not
// know stands as it was written.
const STATUSES: Record<string, StatusView> = {
  open: { label: "archive.status.open", color: "info", icon: "i-material-symbols-progress-activity" },
  done: { label: "archive.status.done", color: "success", icon: "i-material-symbols-check-circle-outline" },
  unanswered: { label: "archive.status.unanswered", color: "warning", icon: "i-material-symbols-help-outline" },
  "cut-short": { label: "archive.status.cut_short", color: "warning", icon: "i-material-symbols-content-cut" },
  stopped: { label: "archive.status.stopped", color: "neutral", icon: "i-material-symbols-stop-circle-outline" },
  failed: { label: "archive.status.failed", color: "error", icon: "i-material-symbols-cancel-outline" }
};

export function statusView(t: Translate, status: string): StatusView {
  const known = STATUSES[status];

  return known === undefined
    ? { label: status, color: "neutral", icon: "i-material-symbols-radio-button-checked-outline" }
    : { ...known, label: t(known.label) };
}

// The site a page is on, without the prefix nobody reads.
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

// What a page's thumbnail falls back to when it has no picture and no icon.
export function hostInitial(url: string): string {
  return (hostOf(url).match(/[\p{L}\p{N}]/u)?.[0] ?? "?").toUpperCase();
}

// A page's title: what the page said of itself, then what the record called it, then its site.
export function pageTitle(url: string, recorded: string | null, preview: SearchRefPreview | null): string {
  return preview?.title ?? recorded ?? hostOf(url);
}

// The line under it: what the page was cited for, else its site.
export function pageSubtitle(url: string, note: string | null): string {
  return note === null || note.trim() === "" ? hostOf(url) : note;
}

// One heading of the source list and the pages under it.
export interface SourceGroup {
  key: string;
  title: string;
  sources: SearchSourceView[];
}

// The source list's groups. An entry of a set — a candidate, a place — that names itself keeps its
// pages under that name. Every other finding's pages stand together, first, under the cited-sources
// heading: a part label says where a finding opens rather than what its pages back — a finding opening
// with its summary cites for the whole document under it — and a first sentence is no name at all.
export function sourceGroups(t: Translate, findings: readonly Pick<SearchFindingView, "name" | "title" | "place" | "sources">[]): SourceGroup[] {
  const cited = findings.filter((finding) => finding.name === null).flatMap((finding) => finding.sources);
  const named = findings
    .filter((finding) => finding.sources.length > 0 && finding.name !== null)
    .map((finding) => ({ key: String(finding.place), title: finding.title, sources: finding.sources }));

  return [...(cited.length === 0 ? [] : [{ key: "cited", title: t("archive.title.cited_sources"), sources: cited }]), ...named];
}

// Each option's name and each of its values, by the key of the word it is said in.
const OPTION_NAMES: Record<string, string> = {
  period: "archive.label.option_period",
  region: "archive.label.option_region",
  exclude: "archive.label.option_exclude",
  shape: "archive.label.option_shape",
  source: "archive.label.option_source"
};
const OPTION_VALUES: Record<string, string> = {
  domestic: "archive.label.value_domestic",
  global: "archive.label.value_global",
  brief: "archive.label.value_brief",
  table: "archive.label.value_table",
  full: "archive.label.value_full",
  learning: "archive.label.value_learning"
};

// A name or a value as it is said: in its word where there is one, and as it was written where not.
function said(t: Translate, keys: Record<string, string>, word: string): string {
  const key = keys[word];

  return key === undefined ? word : t(key);
}

// The launch's options as the properties viewer says them.
export function optionLines(t: Translate, options: Record<string, string>): { label: string; value: string }[] {
  return Object.entries(options).map(([name, value]) => ({ label: said(t, OPTION_NAMES, name), value: said(t, OPTION_VALUES, value) }));
}

type HeadSource = Pick<SearchSummary, "title" | "query" | "topic" | "keywords" | "rule" | "options" | "startedDepth" | "endedDepth">;

// What each property a kind's head may show is called on the screen, and what it reads off the
// document. A property with nothing to say stands nowhere.
const HEAD_LINES: Record<HeadProperty, (t: Translate, search: HeadSource) => BarProperty[]> = {
  // Where the headline is the question itself, the question is already on the screen.
  question: (t, search) => (search.title === null ? [] : [{ label: t("archive.label.head_question"), value: search.query }]),
  request: (t, search) => [{ label: t("archive.label.head_request"), value: search.query }],
  topic: (t, search) => (search.topic === null ? [] : [{ label: t("archive.label.head_topic"), value: search.topic }]),
  // A page keeps the description read out of it where a search keeps its topic.
  description: (t, search) => (search.topic === null ? [] : [{ label: t("archive.label.head_description"), value: search.topic }]),
  keywords: (t, search) =>
    search.keywords.length === 0 ? [] : [{ label: t("archive.label.head_keywords"), value: search.keywords.join(", ") }],
  rule: (t, search) => (search.rule === null ? [] : [{ label: t("archive.label.head_rule"), value: search.rule }]),
  options: (t, search) => optionLines(t, Object.fromEntries(Object.entries(search.options).filter(([name]) => name !== "source"))),
  depth: (t, search) => [{ label: t("archive.label.head_depth"), value: depthText(t, search.startedDepth, search.endedDepth) }],
  source: (t, search) =>
    search.options.source === undefined ? [] : [{ label: said(t, OPTION_NAMES, "source"), value: search.options.source, mono: true }]
};

// How many lines of an attached text its preview shows.
const PREVIEW_LINES = 40;

// The start of an attached text as its preview shows it, out of the bytes a ranged read brought. A
// read cut inside a character loses that character rather than showing it broken, and a text that
// goes on past what is shown ends in an ellipsis, so nothing reads as the whole of it that is not. The
// range header says how long the whole file is.
export function textPreview(bytes: Uint8Array, contentRange: string | null): string {
  const total = Number(/\/(\d+)\s*$/.exec(contentRange ?? "")?.[1] ?? bytes.byteLength);
  const decoded = new TextDecoder("utf-8").decode(bytes).replace(/�+$/, "");
  const lines = decoded.split("\n");
  const shown = lines.slice(0, PREVIEW_LINES).join("\n").trimEnd();

  return total > bytes.byteLength || lines.length > PREVIEW_LINES ? `${shown}\n…` : shown;
}

// The properties a document's head shows, in the order its kind names them. The state, the room and
// the times stand on every document and are the page's to add.
export function headProperties(t: Translate, head: readonly HeadProperty[], search: HeadSource): BarProperty[] {
  return head.flatMap((property) => HEAD_LINES[property](t, search));
}

// Where the console serves what a ref of one document stands for. The base is the road that
// document's refs are answered under, which the page reading it supplies: the console names its
// document there, a share page names its token, and neither the document's parts nor this function
// knows which of the two it is. The address outside is never part of it.
export function refPath(refBase: string, ref: SearchRef, part: "preview" | "image", as?: "thumb" | "favicon"): string {
  const path = `${refBase}/${encodeURIComponent(ref)}/${part}`;

  return as === undefined ? path : `${path}?as=${as}`;
}

function comparable(href: string): string {
  try {
    return new URL(href).toString();
  } catch {
    return href;
  }
}

// The pictures a finding may show, from the ones the server found in its text: each address is
// answered with the console's copy of it, and anything else with nothing.
export function findingImages(
  refBase: string,
  finding: Pick<SearchFindingView, "images">,
  prefix: (path: string) => string
): (href: string) => string | null {
  const known = new Map(finding.images.map((image) => [comparable(image.url), prefix(refPath(refBase, image.ref, "image"))]));

  return (href) => known.get(comparable(href)) ?? null;
}

// Where one file a document keeps is served under the road its files are answered under, and where it
// is taken to be saved.
export function attachmentUrl(base: string, attachment: Pick<ArchiveAttachmentView, "id">, download = false): string {
  return `${base}/${attachment.id}${download ? "?download=1" : ""}`;
}

// A page's address naming the scheme the console stands in, so it is drawn in that scheme from its
// first frame (`docs/archive.md`, "How a document is drawn").
export function schemedPage(url: string, mode: string): string {
  return `${url}?scheme=${mode === "light" ? "light" : "dark"}`;
}

// The files a document keeps that are heard rather than read, and the rest. A sound stands under the
// masthead as the document's listening band (`docs/archive.md`, "How a document is drawn"), and the
// rest stand after the body.
export function soundsFirst<T extends Pick<ArchiveAttachmentView, "media">>(attachments: readonly T[]): { sounds: T[]; others: T[] } {
  return { sounds: attachments.filter((one) => one.media === "audio"), others: attachments.filter((one) => one.media !== "audio") };
}

// Where each file a document keeps is served, by the name it is kept under: a picture or a link in the
// body that names one of them by that name, rather than by an address, is drawn from it or goes to it.
// The base is the road the document's files are answered under, as for the attachment section. A name
// two files share is the first of them.
export function attachmentAddresses(base: string, attachments: readonly Pick<ArchiveAttachmentView, "id" | "name">[]): (href: string) => string | null {
  const byName = new Map<string, string>();

  for (const attachment of attachments) {
    if (!byName.has(attachment.name)) byName.set(attachment.name, `${base}/${attachment.id}`);
  }

  return (href) => {
    try {
      return byName.get(decodeURIComponent(href)) ?? null;
    } catch {
      return byName.get(href) ?? null;
    }
  };
}

// The attachments that stand in the attachment section: every one but a picture the body already draws
// by the name it is kept under, which the read API says of each (`docs/api.md`, "Documents (HTTP)").
export function undrawnAttachments<T extends Pick<ArchiveAttachmentView, "drawn">>(attachments: readonly T[]): T[] {
  return attachments.filter((attachment) => !attachment.drawn);
}

// The topic and keywords of a document as one line under its title, and nothing for one that wrote
// neither.
export function subjectText(search: { topic: string | null; keywords: string[]; rule: string | null }): string {
  return [search.topic, search.keywords.length === 0 ? null : search.keywords.map((word) => `#${word}`).join(" "), search.rule]
    .filter((part): part is string => part !== null && part !== "")
    .join(" · ");
}

// The topic, keywords and rule as a row's one line shows them: from just before the hit where the
// phrase the rows were found by is in them, since the line is cut at its end.
export function subjectLine(search: { topic: string | null; keywords: string[]; rule: string | null }, phrase: string): string {
  const text = subjectText(search);

  return searchHits(text, phrase).length === 0 ? text : hitSnippet(text, phrase, text.length, SEARCHES_HIT_LEAD);
}

// Every text a row of the list carries, for the console's list search to mark and to say where a row
// matched: what the row shows, then the question where the title stands in its place, and the body's
// words around the hit the server handed over.
export function archiveSearchFields(t: Translate, search: SearchSummary, room: string): SearchField[] {
  return [
    { label: "", text: searchTitle(search), shown: true },
    { label: "", text: subjectText(search), shown: true },
    { label: "", text: room, shown: true },
    { label: t("archive.label.head_question"), text: search.title === null ? null : search.query, shown: false },
    { label: t("archive.label.body"), text: search.excerpt ?? null, shown: false }
  ];
}

// A room's name, and its number where the room's own unit named none.
export function roomName(search: Pick<SearchSummary, "room" | "chatId">): string {
  return search.room ?? String(search.chatId);
}
