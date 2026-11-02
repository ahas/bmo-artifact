import { Lexer } from "marked";

import type { Token } from "marked";
import type { ArtifactAttachment, ArtifactDocument, ArtifactKind } from "../../shared/types";

// How the document screens read what the service answers: a document's kind in the catalogue, its
// category, number and progress, its contents out of its headings, the files its body draws and the ones that stand after it, and the
// sounds heard at its head.

/** The kind a document's word names in the catalogue; null while the catalogue is unread. */
export function kindOf(kinds: readonly ArtifactKind[], name: string | null | undefined): ArtifactKind | null {
  return kinds.find((kind) => kind.name === name) ?? null;
}

/** The label a document's category is kept under, the filer's own word for the work that made it (`docs/artifact.md`, "Where a person reads it"). */
export const CATEGORY_LABEL = "category";

/** A document's category: its `category` label's word, null where it carries none. */
export function categoryOf(document: Pick<ArtifactDocument, "labels">): string | null {
  const word = document.labels[CATEGORY_LABEL]?.trim() ?? "";

  return word === "" ? null : word;
}

/** The list's narrowing by a category, as `GET /documents` takes it. */
export function categoryFilter(word: string): string {
  return `${CATEGORY_LABEL}:${word}`;
}

/** The label a document in progress carries, naming what its filer still has to give it (`docs/artifact.md`, "Where a person reads it"). */
export const PENDING_LABEL = "pending";

/** What a document in progress still lacks, in its filer's words; null for a document done. */
export function pendingOf(document: Pick<ArtifactDocument, "labels">): string[] | null {
  const words = (document.labels[PENDING_LABEL] ?? "")
    .split(",")
    .map((word) => word.trim())
    .filter((word) => word !== "");

  return words.length === 0 ? null : words;
}

/** The label a document's instruction is kept under, the words it was asked for with (`docs/artifact.md`, "Where a person reads it"). */
export const INSTRUCTION_LABEL = "query";

/** A document's instruction, null where it carries none. */
export function instructionOf(document: Pick<ArtifactDocument, "labels">): string | null {
  const words = document.labels[INSTRUCTION_LABEL]?.trim() ?? "";

  return words === "" ? null : words;
}

/** A document's labels but its instruction, which its screen offers behind an action of its own. */
export function labelsBesideInstruction(labels: Readonly<Record<string, string>>): Record<string, string> {
  return Object.fromEntries(Object.entries(labels).filter(([name]) => name !== INSTRUCTION_LABEL));
}

/** A document's number as a person reads, says and copies it. */
export function numberText(id: number): string {
  return `#${id}`;
}

/** The site a page is on, without the prefix nobody reads. */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** What a page's thumbnail falls back to when it has no picture. */
export function hostInitial(url: string): string {
  return (hostOf(url).match(/[\p{L}\p{N}]/u)?.[0] ?? "?").toUpperCase();
}

// Every token of a markdown text, the inline ones inside the blocks among them.
function walk(tokens: readonly Token[], visit: (token: Token) => void): void {
  for (const token of tokens) {
    visit(token);

    const inner = (token as { tokens?: Token[] }).tokens;
    const items = (token as { items?: Token[] }).items;

    if (inner !== undefined) walk(inner, visit);
    if (items !== undefined) walk(items, visit);
  }
}

/** One line of a document's table of contents: what it is called, how deep it sits and its mark. */
export interface ContentsEntry {
  slug: string;
  text: string;
  depth: number;
}

/**
 * A markdown body's contents to its kind's depth: its headings counted by rank, the shallowest depth the
 * body writes being its first level, each with the mark its heading is stamped with in the text.
 */
export function contentsOf(markdown: string, depth: number): { entries: ContentsEntry[]; slugs: string[] } {
  const headings: { depth: number; text: string }[] = [];

  walk(new Lexer().lex(markdown), (token) => {
    if (token.type === "heading") headings.push({ depth: (token as { depth: number }).depth, text: (token as { text: string }).text });
  });

  const levels = [...new Set(headings.map((heading) => heading.depth))].sort((a, b) => a - b);
  const slugs = headings.map((_, at) => `section-${at + 1}`);
  const entries = headings
    .map((heading, at) => ({ slug: slugs[at]!, text: heading.text, depth: levels.indexOf(heading.depth) + 1 }))
    .filter((entry) => entry.depth <= depth);

  return { entries, slugs };
}

// Whether a text is a web address, or any address with a scheme, rather than a path.
function isAddress(text: string): boolean {
  try {
    new URL(text);

    return true;
  } catch {
    return false;
  }
}

/**
 * The names a picture's or a link's target may name a kept file by, its percent-encoding undone: the target
 * whole, and where it is a path with no scheme, its last part. A web address names no kept file but by its
 * whole.
 */
export function keptNames(href: string): string[] {
  let target = href;

  try {
    target = decodeURIComponent(href);
  } catch {
    // A target whose percent-encoding does not decode is read as it stands.
  }

  const last = target.slice(target.lastIndexOf("/") + 1);

  return last === target || last === "" || isAddress(target) ? [target] : [target, last];
}

/** The names a markdown body draws pictures by, each as `keptNames` reads it. */
export function drawnNames(markdown: string): Set<string> {
  const names = new Set<string>();

  walk(new Lexer().lex(markdown), (token) => {
    if (token.type !== "image") return;

    for (const name of keptNames((token as { href: string }).href)) names.add(name);
  });

  return names;
}

/**
 * Where each file a document keeps is served, by the name it is kept under, for a target that names it as
 * `keptNames` reads one; the first of two that share one.
 */
export function fileAddresses(attachments: readonly ArtifactAttachment[], address: (attachment: ArtifactAttachment) => string): (href: string) => string | null {
  const byName = new Map<string, string>();

  for (const attachment of attachments) {
    if (!byName.has(attachment.name)) byName.set(attachment.name, address(attachment));
  }

  return (href) => {
    for (const name of keptNames(href)) {
      const found = byName.get(name);

      if (found !== undefined) return found;
    }

    return null;
  };
}

// What a file's name says of the language it is in (`docs/artifact.md`, "Attachments"): the name with that
// part taken out, and the language's two-letter code, for a name whose last part before the extension is a
// code some language is known by; null for any other name.
function spokenIn(name: string): { rest: string; language: string } | null {
  const found = /^(.+)-([a-z]{2})(\.[^.]+)$/i.exec(name);

  if (found === null) return null;

  const language = found[2]!.toLowerCase();

  return languageName(language) === null ? null : { rest: `${found[1]}${found[3]}`, language };
}

// A language's name in its own words, null for a code no language is known by.
function languageName(language: string): string | null {
  try {
    return new Intl.DisplayNames([language], { type: "language", fallback: "none" }).of(language) ?? null;
  } catch {
    return null;
  }
}

/** The language a file's name says it is in, named in its own words; null where the name says none. */
export function languageOf(attachment: ArtifactAttachment): string | null {
  const spoken = spokenIn(attachment.name);

  return spoken === null ? null : languageName(spoken.language);
}

/**
 * The sounds heard at a markdown document's head, in the reader's listen band (`docs/artifact.md`, "How a
 * document is drawn"): every sound marked `head`, and beside a head sound whose name carries a language,
 * every sound named as it is in another language, in the order they were kept.
 */
export function headSounds(attachments: readonly ArtifactAttachment[]): ArtifactAttachment[] {
  const sounds = attachments.filter((one) => one.kind === "audio");
  const heads = sounds.filter((one) => one.role === "head");
  const told = new Set(heads.flatMap((one) => spokenIn(one.name)?.rest ?? []));

  return sounds.filter((one) => one.role === "head" || told.has(spokenIn(one.name)?.rest ?? ""));
}

/**
 * The files a document keeps, split as a reader meets them: the sounds heard at its head, and the files
 * that stand after the body, every one but a picture the body already draws by its name.
 */
export function filesAround(
  attachments: readonly ArtifactAttachment[],
  drawn: ReadonlySet<string>
): { head: ArtifactAttachment[]; after: ArtifactAttachment[] } {
  const head = headSounds(attachments);

  return {
    head,
    after: attachments.filter((one) => !head.includes(one) && !(one.kind === "image" && drawn.has(one.name)))
  };
}

/** How many lines of an attached text its preview shows. */
const PREVIEW_LINES = 40;

/**
 * The start of an attached text as its preview shows it, out of the bytes a ranged read brought. A read
 * cut inside a character loses that character, and a text that goes on past what is shown ends in an
 * ellipsis. The range header says how long the whole file is.
 */
export function textPreview(bytes: Uint8Array, contentRange: string | null): string {
  const total = Number(/\/(\d+)\s*$/.exec(contentRange ?? "")?.[1] ?? bytes.byteLength);
  const decoded = new TextDecoder("utf-8").decode(bytes).replace(/�+$/, "");
  const lines = decoded.split("\n");
  const shown = lines.slice(0, PREVIEW_LINES).join("\n").trimEnd();

  return total > bytes.byteLength || lines.length > PREVIEW_LINES ? `${shown}\n…` : shown;
}

/** A page's address naming the scheme the console stands in, so it is drawn in it from its first frame. */
export function schemeOf(mode: string): string {
  return mode === "light" ? "light" : "dark";
}
