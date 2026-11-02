import { isDiagram } from "bmo-ui/app/domain/diagram";
import DOMPurify from "dompurify";
import { Marked, Renderer, Tokenizer } from "marked";

import { citeId, entryId } from "./footnotes";
import { addressAsWritten, isWebAddress } from "./web-address";

import type { Translate } from "bmo-ui/shared/wording";
import type { RendererThis, Token, TokenizerAndRendererExtension, Tokens } from "marked";

// A finding's markdown, as the read API answers it, turned into the HTML that stands for it
// (`docs/layout.md`, "The console plugin bundle"). The server has already turned the finding's
// structure lines into markdown in the wording file's words; what is read here is that markdown,
// with the three blocks of its own the server writes for what markdown has no form for (`docs/api.md`,
// "Documents (HTTP)").
//
// A finding is written a line at a time, so a newline is a line break. The HTML typed into it stands
// as the characters it was typed as, and a tilde is a range and never a strike. A heading takes the
// mark the contents beside the text steer by, a footnote mark is a raised number linking to its
// source entry and the entry's mark a link back to it (`docs/archive.md`, "How a document is drawn"),
// and a picture is a picture only where the caller hands an address for it — the console's
// own copy of a picture the record holds, or a file the document keeps by its name; anything it does
// not answer for stands as a link, so a line of somebody else's text never makes the reader's
// browser call a server of its choosing.
//
// Nothing in here may be reached from the server: DOMPurify cleans against a real document and there
// is none on the server, and every screen renders there as well. So the module is asked for with a
// dynamic import at the moment a finding is first shown (see finding-load).

// Where a picture or a file a finding names is shown from, or null where it is not to be loaded.
export type ImageSource = (href: string) => string | null;

export interface FindingSources {
  images?: ImageSource;
  // Where a file the finding's document keeps is served, for a picture or a link that names one of
  // them by its name rather than by an address.
  files?: ImageSource;
  // The mark to stamp on each heading, in the order the headings stand: the outline's own list, so
  // the contents beside the text and the headings in it are the same marks by construction.
  slugs?: readonly string[];
}

// One slide of a gallery as the screen builds it: where its picture is shown from, null where the
// finding's record holds no copy of it, the address the search named and its caption.
export interface GallerySlide {
  src: string | null;
  href: string;
  caption: string;
}

// The server's blocks: a fold or a gallery closed by a line of its own, and a map on one line. These
// are the archive's own syntax, written by the server's reading, so they are read here by their own
// shape.
const FINDING_BLOCK = /^::: (fold|gallery)(?: ([^\n]*))?\n([\s\S]*?)\n:::(?:\n|$)/;
const FINDING_MAP = /^::: map ([^\n]+)(?:\n|$)/;

const FOOTNOTE_HREF = /^#mark-(.+)$/;

// A footnote mark written straight into the text: a page's number in square brackets, a dagger after it
// where the value was read only in a snippet. A bracket a link's address follows is that link's words.
const BARE_MARK = /^\[(\d{1,4}†?)\](?!\()/;

const ESCAPES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (character) => ESCAPES[character]!);
}

const MARKED_ESCAPES: Record<string, string> = { "&amp;": "&", "&lt;": "<", "&gt;": ">", "&quot;": "\"", "&#39;": "'" };

// The words of a picture as they were typed. marked hands a renderer a picture's words already
// escaped, unlike every other token's, so they are taken back to their characters before being
// escaped once for where they stand.
function pictureWords(text: string): string {
  return text.replace(/&(?:amp|lt|gt|quot|#39);/g, (entity) => MARKED_ESCAPES[entity]!);
}

// A picture where it is not drawn: a link to it, named by its words.
function pictureLink({ href, text }: Tokens.Image): string {
  const words = pictureWords(text);

  return `<a href="${escapeHtml(href)}">${escapeHtml(words === "" ? href : words)}</a>`;
}

// Strong words whose closing pair CommonMark refuses: one standing after a bracket or a quote and
// before a letter, which is how a Korean sentence closes an emphasis and goes straight on into its
// particle. marked reads the ordinary form first, and this one only where that opened nothing, so
// what CommonMark sets is set as it always was.
const STRONG = /^\*\*(?=[^\s*])([^\n]*?[^\s*])\*\*(?!\*)/;

function koreanStrong(this: Tokenizer, src: string, masked: string, previous?: string): Tokens.Strong | Tokens.Em | undefined {
  const standard = Tokenizer.prototype.emStrong.call(this, src, masked, previous);

  if (standard !== undefined) return standard;

  const strong = STRONG.exec(src);

  return strong === null ? undefined : { type: "strong", raw: strong[0], text: strong[1]!, tokens: this.lexer.inlineTokens(strong[1]!) };
}

// A bare address ends where the sentence takes over — at a Korean letter written straight after it,
// and at a closing bracket it did not open — by the rule of web-address. GFM as marked reads it runs
// on through the letters.
function sentenceUrl(this: Tokenizer, src: string): Tokens.Link | undefined {
  const standard = Tokenizer.prototype.url.call(this, src);

  if (standard === undefined || standard.href.startsWith("mailto:")) return standard;

  const raw = addressAsWritten(standard.raw);

  if (raw === standard.raw) return standard;

  const www = /^www\./i.test(raw);

  if (!www && !isWebAddress(raw)) return undefined;

  const text = escapeHtml(raw);

  return { type: "link", raw, text, href: www ? `http://${raw}` : raw, tokens: [{ type: "text", raw: text, text }] };
}

// A table stands in a box of its own that scrolls it sideways, so the table can be laid out wider than
// the column it stands in while the page itself never moves sideways. The box carries how many
// columns the table has, since how wide a table may be laid out goes by that.
function scrollingTable(this: RendererThis, token: Tokens.Table): string {
  const table = Renderer.prototype.table.call(this as unknown as Renderer, token);

  return `<div class="markdown-table" style="--columns: ${token.header.length}">${table}</div>`;
}

// The language a fence names: the first word of its info string, as markdown reads it.
function fenceWord(info: string | undefined): string {
  return (info ?? "").trim().split(/\s+/)[0] ?? "";
}

// A fenced block stands in a box of its own with a copy button over its corner, as bmo-ui's code
// modules read it: what a press does is the component's that put the text on the screen, and a
// block whose fence names a language carries that word, which is what colours it. A block written
// in a diagram language stands as a diagram's place instead, with no copy button.
const COPY_ICON =
  '<svg class="markdown-copy-idle" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M9 18q-.825 0-1.412-.587T7 16V4q0-.825.588-1.412T9 2h9q.825 0 1.413.588T20 4v12q0 .825-.587 1.413T18 18zm0-2h9V4H9zm-4 6q-.825 0-1.412-.587T3 20V6h2v14h11v2zm4-6V4z"/></svg>';
const COPIED_ICON =
  '<svg class="markdown-copy-done" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="m9.55 18l-5.7-5.7l1.425-1.425L9.55 15.15l9.175-9.175L20.15 7.4z"/></svg>';

function copyableCode(this: RendererThis, token: Tokens.Code): string {
  if (isDiagram(token.lang)) {
    return `<div class="markdown-diagram" data-diagram><pre><code>${escapeHtml(token.text)}</code></pre></div>\n`;
  }

  const block = Renderer.prototype.code.call(this as unknown as Renderer, token).trimEnd();
  const language = escapeHtml(fenceWord(token.lang));
  const named = language === "" ? "" : ` data-language="${language}"`;
  const head = language === "" ? "" : `<div class="markdown-code-head"><span class="markdown-code-language">${language}</span></div>`;
  const label = escapeHtml(speak("common.action.copy"));

  return `<div class="markdown-code"${named}>${head}${block}<button type="button" class="markdown-copy" data-copy aria-label="${label}" data-tooltip="${label}">${COPY_ICON}${COPIED_ICON}</button></div>\n`;
}

// A rise or a fall a search marks with an up or a down triangle in front of a figure — a price change,
// a return, a forecast direction — stands in the colour a Korean market reads it in: red up, blue
// down. The colour takes the arrow and the figure written after it, an optional space between, and
// one bracketed change after that, so an arrow, a figure and its percentage in brackets are one
// coloured run. A table cell's bar, a footnote mark and a space end the run, and a figure ends at its
// last digit, percent sign or unit, so the particle or the comma a sentence goes on with stands
// outside the colour. The units a Korean figure is written with after its digits — won, a hundred
// million, a trillion, ten thousand, a thousand and times — are named by their code points.
const MOVE = /^([▲▼])([ \t]?[^\s|<>[\]()]*)/;
const UNITS = String.fromCodePoint(0xc6d0, 0xc5b5, 0xc870, 0xb9cc, 0xcc9c, 0xbc30);
const FIGURE = new RegExp(`^(.*[\\d%](?:%p|bp|p|[${UNITS}]+)?)`);
const CHANGE = /^[ \t]?\((?:[+\-−▲▼][^\s()]*|[^\s()]*%)\)/;

function marketMove(src: string): string | null {
  const move = MOVE.exec(src);

  if (move === null) return null;

  // An arrow with no figure after it is coloured alone, not the word it stands beside.
  const written = move[2]!;

  if (!/\d/.test(written)) return move[1]!;

  const figure = FIGURE.exec(written)?.[1] ?? written;
  const head = `${move[1]}${figure}`;

  return figure === written ? head + (CHANGE.exec(src.slice(head.length))?.[0] ?? "") : head;
}

const marketMoves: TokenizerAndRendererExtension = {
  name: "marketMove",
  level: "inline",
  start: (src) => {
    const at = src.search(/[▲▼]/);

    return at === -1 ? undefined : at;
  },
  tokenizer: (src) => {
    const raw = marketMove(src);

    return raw === null ? undefined : { type: "marketMove", raw, rise: raw.startsWith("▲") };
  },
  renderer: (token) => `<span class="${token["rise"] === true ? "search-rise" : "search-fall"}">${escapeHtml(token.raw)}</span>`
};

// A bare mark is read as a mark wherever it is written; whether it is drawn as one is settled once the
// whole text has been read, by whether the text carries a source entry for it (see sourceEntries).
const bareMarks: TokenizerAndRendererExtension = {
  name: "footnoteMark",
  level: "inline",
  start: (src) => {
    const at = src.search(/\[\d/);

    return at === -1 ? undefined : at;
  },
  tokenizer: (src) => {
    const mark = BARE_MARK.exec(src);

    return mark === null ? undefined : { type: "footnoteMark", raw: mark[0], name: mark[1]!, entry: false };
  },
  renderer: (token) => {
    const name = token["name"] as string;

    if (token["entry"] === true) return entryMark(name, token.raw);

    return findingEntries.has(name) ? textMark(name, `#mark-${encodeURIComponent(name)}`, token.raw) : escapeHtml(token.raw);
  }
};

// The picture sources of the finding being read, and the marks its headings take. Reading is
// synchronous, so the one being read is the one set here for the length of that call.
let findingImages: ImageSource = () => null;
let findingFiles: ImageSource = () => null;
let findingSlugs: readonly string[] = [];
let slugsTaken = 0;
// The marks the finding being read carries a source entry for, and those already drawn in its text,
// whose first place the entry's mark leads back to.
let findingEntries = new Set<string>();
let marksDrawn = new Set<string>();

// A mark in the text: a raised number whose link names the mark, which the screen answers by bringing
// the source forward. The first of a name with an entry carries the place its entry's mark leads back
// to. A snippet's mark says it was checked against the snippet only, in its popover where it has an
// entry and in its own title where it has none.
function textMark(name: string, href: string, text: string): string {
  const snippet = name.endsWith("†");
  const entry = findingEntries.has(name);
  const first = entry && !marksDrawn.has(name);

  marksDrawn.add(name);

  const place = first ? ` id="${escapeHtml(citeId(name))}"` : "";
  const note = snippet && !entry ? ` title="${escapeHtml(speak("artifact.label.snippet_only"))}"` : "";

  return `<sup class="search-mark${snippet ? " search-mark-snippet" : ""}"><a href="${escapeHtml(href)}" data-mark="${escapeHtml(name)}"${place}${note}>${escapeHtml(text)}</a></sup>`;
}

// The mark an entry opens with: a link back to the mark's first place in the text.
function entryMark(name: string, text: string): string {
  return `<a href="#${escapeHtml(encodeURIComponent(citeId(name)))}" class="search-mark-back" data-mark-back="${escapeHtml(name)}">${escapeHtml(text)}</a>`;
}

// The inline tokens a list item opens with, in a tight list and a loose one alike.
function itemInline(item: Tokens.ListItem): Token[] {
  const first = item.tokens[0] as { tokens?: Token[] } | undefined;

  return first?.tokens ?? [];
}

// The mark a list item is the source entry of: an item opening with a bare mark and going on with a link.
function entryOf(item: Tokens.ListItem): string | null {
  const [head, ...rest] = itemInline(item);

  return head?.type === "footnoteMark" && rest.some((token) => token.type === "link") ? (head["name"] as string) : null;
}

// Every source entry of a finding, wherever its list stands, each entry's opening mark taken as the
// entry's own rather than as a mark in the text.
function sourceEntries(tokens: readonly Token[], found: Set<string>): Set<string> {
  for (const token of tokens) {
    if (token.type === "list") {
      for (const item of (token as Tokens.List).items) {
        const name = entryOf(item);

        if (name !== null) {
          found.add(name);
          (itemInline(item)[0] as Tokens.Generic)["entry"] = true;
        }
      }
    }

    const inner = (token as { tokens?: Token[] }).tokens;
    const items = (token as { items?: Token[] }).items;

    if (inner !== undefined) sourceEntries(inner, found);
    if (items !== undefined) sourceEntries(items, found);
  }

  return found;
}

// The words the reading puts on what it draws — a copy button's name, a mark's note — asked of the
// translator the caller handed in, and held only while one finding is being built.
let speak: Translate = () => "";

function comparableAddress(href: string): string {
  try {
    return new URL(href).toString();
  } catch {
    return href;
  }
}

function pictureSource(href: string): string | null {
  return findingImages(comparableAddress(href)) ?? findingFiles(href);
}

// The pictures inside a block, in the order they stand.
function blockPictures(tokens: readonly Token[]): Tokens.Image[] {
  return tokens.flatMap((token) => (token.type === "image" ? [token as Tokens.Image] : "tokens" in token && Array.isArray(token.tokens) ? blockPictures(token.tokens) : []));
}

// The server's blocks, each drawn as a place the screen fills in with its own component once the
// text is on the page: a fold with its label and its body, a gallery with its slides, and a place's
// map with what it is found by. Until then a fold stands open and a gallery as its pictures one under
// another, so the text says the same before the components arrive.
const findingBlock: TokenizerAndRendererExtension = {
  name: "findingBlock",
  level: "block",
  // No start is named: a block always follows a blank line, where every block tokenizer is tried,
  // and a start would let one cut into a paragraph a line of the search's own words began.
  tokenizer(src) {
    const block = FINDING_BLOCK.exec(src);

    if (block !== null) {
      return { type: "findingBlock", raw: block[0], kind: block[1], label: block[2] ?? "", tokens: this.lexer.blockTokens(block[3] ?? "", []) };
    }

    const map = FINDING_MAP.exec(src);

    return map === null ? undefined : { type: "findingBlock", raw: map[0], kind: "map", label: map[1] ?? "", tokens: [] };
  },
  renderer(token) {
    const tokens = token.tokens ?? [];

    if (token.kind === "map") {
      return `<div class="finding-map" data-map-query="${escapeHtml(token.label as string)}"></div>`;
    }

    if (token.kind === "gallery") {
      const pictures = blockPictures(tokens);
      const slides: GallerySlide[] = pictures.map((picture) => ({ src: pictureSource(picture.href), href: picture.href, caption: pictureWords(picture.text) }));

      return `<div class="finding-gallery" data-slides="${escapeHtml(JSON.stringify(slides))}">${pictures.map((picture) => this.parser.parseInline([picture])).join("")}</div>`;
    }

    return `<div class="finding-fold" data-fold-label="${escapeHtml(token.label as string)}"><div class="finding-fold-body">${this.parser.parse(tokens)}</div></div>`;
  }
};

const findings = new Marked({
  async: false,
  gfm: true,
  breaks: true,
  extensions: [findingBlock, marketMoves, bareMarks],
  tokenizer: { emStrong: koreanStrong, url: sentenceUrl },
  renderer: {
    html: ({ text }) => escapeHtml(text),
    table: scrollingTable,
    code: copyableCode,
    // A finding is plain text, where a tilde is a range and never a strike.
    del: ({ raw }) => escapeHtml(raw),
    // A heading takes the mark the contents beside the text point at it, so pressing an entry there
    // finds the heading here. The marks are consumed in the order the headings stand, and a heading
    // with no words takes none — which is exactly the heading the contents skipped.
    heading({ tokens, depth }) {
      const inside = this.parser.parseInline(tokens);
      const words = this.parser.parseInline(tokens, this.parser.textRenderer).trim();
      const slug = words === "" ? undefined : findingSlugs[slugsTaken++];

      return `<h${depth}${slug === undefined ? "" : ` id="${escapeHtml(slug)}" data-heading="${escapeHtml(slug)}"`}>${inside}</h${depth}>`;
    },
    // A source entry stands where its mark leads, and says which mark it is the source of.
    listitem(item) {
      const html = Renderer.prototype.listitem.call(this as unknown as Renderer, item);
      const name = entryOf(item);

      return name === null ? html : html.replace(/^<li>/, `<li id="${escapeHtml(entryId(name))}" data-source="${escapeHtml(name)}">`);
    },
    // A footnote mark written as a link to its mark is drawn as a bare one is. A link naming one of the
    // document's own files goes to where that file is served.
    link({ href, text, tokens }) {
      const mark = FOOTNOTE_HREF.exec(href);

      if (mark === null) {
        const file = findingFiles(href);

        return file === null ? false : `<a href="${escapeHtml(file)}">${this.parser.parseInline(tokens)}</a>`;
      }

      return textMark(decodeURIComponent(mark[1] ?? ""), href, text);
    },
    image: (picture) => {
      const source = pictureSource(picture.href);

      if (source === null) {
        return pictureLink(picture);
      }

      const text = pictureWords(picture.text);
      const caption = text === "" ? "" : `<span class="markdown-caption">${escapeHtml(text)}</span>`;
      // Where the picture was found, which the picture viewer names in place of the console's copy.
      const found = isWebAddress(picture.href) ? comparableAddress(picture.href) : null;
      const address = found === null ? "" : ` data-picture-address="${escapeHtml(found)}"`;

      return `<span class="markdown-figure"><img src="${escapeHtml(source)}" alt="${escapeHtml(text)}" loading="lazy"${address}>${caption}</span>`;
    }
  }
});

// The HTML marked builds, before anything has cleaned it. It is apart from the cleaning only so the
// reading can be checked where there is no document; nothing puts it on a screen.
export function findingHtml(t: Translate, markdown: string, sources: FindingSources = {}): string {
  const before = speak;

  speak = t;
  findingImages = sources.images ?? (() => null);
  findingFiles = sources.files ?? (() => null);
  findingSlugs = sources.slugs ?? [];
  slugsTaken = 0;
  marksDrawn = new Set();

  try {
    const tokens = findings.lexer(markdown);

    findingEntries = sourceEntries(tokens, new Set());

    return findings.parser(tokens);
  } finally {
    speak = before;
    findingImages = () => null;
    findingFiles = () => null;
    findingSlugs = [];
    findingEntries = new Set();
    marksDrawn = new Set();
  }
}

// A link opens in a tab of its own, and the page it opens is given no hold on the console's window.
// A link to a place inside the same text stays where it is. The hook is put on the first time
// anything is cleaned rather than when the module loads, since a DOMPurify with no document under it
// has no hooks to take.
let linksHooked = false;

function hookLinks(): void {
  if (linksHooked) {
    return;
  }

  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    const href = node.getAttribute("href");

    if (node.tagName === "A" && href !== null && !href.startsWith("#")) {
      node.setAttribute("target", "_blank");
      node.setAttribute("rel", "noopener noreferrer");
    }
  });
  linksHooked = true;
}

// A finding's markdown to HTML, and the HTML through DOMPurify before any of it reaches the page. The
// escaping above already keeps typed HTML out, but a link's address is still whatever was written,
// and a text that can run code in the console runs it as the session that is signed in.
export function renderFinding(t: Translate, markdown: string, sources: FindingSources = {}): string {
  hookLinks();

  return DOMPurify.sanitize(findingHtml(t, markdown, sources));
}
