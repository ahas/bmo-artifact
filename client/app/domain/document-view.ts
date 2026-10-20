import { Lexer } from "marked";

import type { Token } from "marked";
import type { ArtifactAttachment, ArtifactKind } from "../../shared/types";

// How the document screens read what the service answers: a document's kind in the catalogue, its
// contents out of its headings, the files its body draws and the ones that stand after it, and the
// sounds heard at its head.

/** The kind a document's word names in the catalogue; null while the catalogue is unread. */
export function kindOf(kinds: readonly ArtifactKind[], name: string | null | undefined): ArtifactKind | null {
  return kinds.find((kind) => kind.name === name) ?? null;
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

/** The names a markdown body draws pictures by, their percent-encoding undone. */
export function drawnNames(markdown: string): Set<string> {
  const names = new Set<string>();

  walk(new Lexer().lex(markdown), (token) => {
    if (token.type !== "image") return;

    const href = (token as { href: string }).href;

    try {
      names.add(decodeURIComponent(href));
    } catch {
      names.add(href);
    }
  });

  return names;
}

/** Where each file a document keeps is served, by the name it is kept under; the first of two that share one. */
export function fileAddresses(attachments: readonly ArtifactAttachment[], address: (attachment: ArtifactAttachment) => string): (href: string) => string | null {
  const byName = new Map<string, string>();

  for (const attachment of attachments) {
    if (!byName.has(attachment.name)) byName.set(attachment.name, address(attachment));
  }

  return (href) => {
    try {
      return byName.get(decodeURIComponent(href)) ?? null;
    } catch {
      return byName.get(href) ?? null;
    }
  };
}

/**
 * The files a document keeps, split as a reader meets them: the sounds marked to be heard at its head,
 * and the files that stand after the body, every one but a picture the body already draws by its name.
 */
export function filesAround(
  attachments: readonly ArtifactAttachment[],
  drawn: ReadonlySet<string>
): { head: ArtifactAttachment[]; after: ArtifactAttachment[] } {
  const head = attachments.filter((one) => one.role === "head" && one.kind === "audio");

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
