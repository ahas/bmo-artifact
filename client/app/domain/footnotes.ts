// How a finding's footnote marks answer a reader, read off the text the finding reading drew
// (`docs/archive.md`, "How a document is drawn"): a mark in the text names the source entry it cites,
// and an entry's own mark leads back to the mark's first place in the text.

/** Where a mark's source entry stands. */
export function entryId(name: string): string {
  return `mark-${name}`;
}

/** Where a mark first stands in the text, which its entry's mark leads back to. */
export function citeId(name: string): string {
  return `cite-${name}`;
}

/** The mark in the text a press or a pointer landed on, inside the text it was drawn in. */
export function markAt(target: EventTarget | null, root: Element): HTMLAnchorElement | null {
  const mark = target instanceof Element ? target.closest<HTMLAnchorElement>("a[data-mark]") : null;

  return mark !== null && root.contains(mark) ? mark : null;
}

/** The entry's own mark a press landed on, inside the text it was drawn in. */
export function backMarkAt(target: EventTarget | null, root: Element): HTMLAnchorElement | null {
  const mark = target instanceof Element ? target.closest<HTMLAnchorElement>("a[data-mark-back]") : null;

  return mark !== null && root.contains(mark) ? mark : null;
}

/** What a source popover holds: the entry's words, formatted as drawn, and the pages it links to. */
export interface MarkSource {
  // The mark as the text shows it, which heads the source as its entry heads it.
  mark: string;
  html: string;
  addresses: string[];
  snippet: boolean;
}

/** The address of a page as a reader reads it: no scheme, and its letters rather than their escapes. */
export function addressText(href: string): string {
  const bare = href.replace(/^https?:\/\//i, "");

  try {
    return decodeURI(bare);
  } catch {
    return bare;
  }
}

/**
 * The source a mark cites, read off its entry in the same text: the entry's words with its own mark
 * left out and each link to a page standing as its words, the pages' addresses apart so they stand as
 * links of their own. Null where the text carries no entry for the mark. The entry was cleaned with the
 * rest of the text, so what is read off it is already clean.
 */
export function sourceOf(root: Element, name: string, mark = `[${name}]`): MarkSource | null {
  const entry = root.querySelector(`[data-source="${CSS.escape(name)}"]`);

  if (entry === null) return null;

  const words = entry.cloneNode(true) as Element;
  const addresses: string[] = [];

  words.querySelectorAll("[data-mark-back]").forEach((mark) => mark.remove());
  words.querySelectorAll<HTMLAnchorElement>("a[href]").forEach((link) => {
    const href = link.getAttribute("href") ?? "";

    if (!/^https?:\/\//i.test(href)) return;

    if (!addresses.includes(href)) addresses.push(href);
    link.replaceWith(...link.childNodes);
  });

  return { mark, html: words.innerHTML.trim(), addresses, snippet: name.endsWith("†") };
}
