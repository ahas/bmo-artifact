import { Lexer } from "marked";
import { computed, ref, shallowReactive, watch } from "vue";

import type { Reading } from "bmo-ui/app/domain/translation";
import type { Token, Tokens } from "marked";
import type { ComputedRef, Ref } from "vue";

// A document read in the reader's language beside its own (docs/artifact.md, 「Where a person reads it」): which
// language its words are written in, the sections its body is asked for in, and the reader's tab, filled through
// the console's translator the first time it is opened.

/** A language of the bundle's pack and the script its words are written in. */
export interface Script {
  language: string;
  // Every run of letters in the script, each one word.
  words: RegExp;
}

/**
 * A language and the Unicode scripts its pack names, comma-separated, a run of letters in any of them one word;
 * none where a name is no script's.
 */
export function scriptOf(language: string, script: string): Script | null {
  const names = script.split(",").map((name) => name.trim());

  try {
    return { language, words: new RegExp(`[${names.map((name) => `\\p{Script=${name}}`).join("")}]+`, "gu") };
  } catch {
    return null;
  }
}

// The tokens whose words nobody reads as the text's own: code, an inline code span, markup, and the marks that
// hold no words.
const UNREAD = new Set(["code", "codespan", "html", "space", "hr", "br", "def"]);

// The words of a run of tokens a reader reads, gathered from their leaves; a table's cells are reached through
// its head and its rows.
function prose(tokens: readonly Token[], out: string[]): string[] {
  for (const token of tokens) {
    if (UNREAD.has(token.type)) continue;

    if (token.type === "table") {
      const table = token as Tokens.Table;

      for (const cell of [...table.header, ...table.rows.flat()]) prose(cell.tokens, out);
      continue;
    }

    const items = (token as { items?: Token[] }).items;
    const inner = (token as { tokens?: Token[] }).tokens;

    if (items !== undefined) prose(items, out);
    else if (inner !== undefined) prose(inner, out);
    else if (typeof (token as { text?: unknown }).text === "string") out.push((token as { text: string }).text);
  }

  return out;
}

/**
 * The language a markdown text is written in: the one whose script writes the most of its words, code left out;
 * none where no word is in any of the scripts.
 */
export function writtenIn(markdown: string, scripts: readonly Script[]): string | null {
  const words = prose(new Lexer().lex(markdown), []).join("\n");
  let most: { language: string; count: number } | null = null;

  for (const script of scripts) {
    const count = words.match(script.words)?.length ?? 0;

    if (count > (most?.count ?? 0)) most = { language: script.language, count };
  }

  return most?.language ?? null;
}

/**
 * A markdown body's sections, in order: each heading with what stands under it to the next heading, and the text
 * before the first heading one of its own. Joined, they are the body, its line ends read as marked reads them.
 */
export function sectionsOf(markdown: string): string[] {
  // The text the lexer reads, every line ending in a line feed, which is what each token's raw text is cut from.
  const source = markdown.replaceAll("\r\n", "\n").replaceAll("\r", "\n");
  const tokens = new Lexer().lex(source);
  const starts = [0];
  let cursor = 0;

  for (const token of tokens) {
    const at = source.indexOf(token.raw, cursor);

    if (at < 0) continue;
    if (token.type === "heading" && at > 0) starts.push(at);

    cursor = at + token.raw.length;
  }

  return starts.map((start, index) => source.slice(start, starts[index + 1] ?? source.length)).filter((section) => section !== "");
}

/** A section as it stands once read: the reading in the room the original's own blank lines and indent leave. */
export function shownSection(original: string, reading: string): string {
  const lead = original.length - original.trimStart().length;
  const trail = original.length - original.trimEnd().length;

  return original.slice(0, lead) + reading.trim() + original.slice(original.length - trail);
}

/** What a document's text is read from: its title and its markdown body. */
export interface ReadingSource {
  title: string;
  markdown: string;
}

/** The texts handed to the translator and their readings, in order, in a language. */
export type Translate = (texts: readonly string[], language: string) => Promise<Reading[]>;

/** The tab a document stands in: its own language, or the reader's. */
export type ReadingTab = "original" | "reader";

/** Where the reader's tab stands. */
export type ReadingState =
  // The original stands, or nothing in the document was asked for.
  | "none"
  // Some part is still asked for.
  | "asking"
  // Every part asked for came whole.
  | "whole"
  // Some part stands in the original: not rendered whole, too busy, or refused.
  | "partial"
  // No part asked for came.
  | "missed";

// One text's reading, where it stands.
type Asked = { kind: "asking" } | { kind: "read"; reading: Reading } | { kind: "missed" };

/** A document read in its own language and in the reader's. */
export interface DocumentReading {
  // The language the document is written in where it offers the reader's tab, none where it offers none.
  written: ComputedRef<string | null>;
  // The reader's language.
  reader: string;
  tab: Ref<ReadingTab>;
  // The title and the body as the tab open shows them.
  title: ComputedRef<string>;
  markdown: ComputedRef<string>;
  state: ComputedRef<ReadingState>;
  // Asks again for every part that did not come whole.
  again: () => void;
}

export interface ReadingOptions {
  // The reader's language, a language tag.
  reader: string;
  // Each language of the bundle's pack with the script its words are written in.
  scripts: readonly Script[];
  // The console's translator; none where the console names none, and nothing is translated.
  translate: Translate | null;
  // How many texts one call of `translate` carries.
  batch: number;
}

/**
 * A document's reading: the reader's tab offered where its words are mostly in another language than the
 * reader's and a translator is named, asked for the first time it is opened, each section standing in the
 * original until its reading comes.
 */
export function documentReading(read: () => ReadingSource | null, options: ReadingOptions): DocumentReading {
  const { reader, scripts, translate } = options;
  const source = computed(read);
  const batch = Math.max(1, options.batch);
  const tab = ref<ReadingTab>("original");
  const asked = shallowReactive(new Map<string, Asked>());
  let generation = 0;
  let opened = false;

  const sections = computed(() => sectionsOf(source.value?.markdown ?? ""));
  const written = computed(() => {
    if (translate === null || source.value === null) return null;

    const language = writtenIn(source.value.markdown, scripts);

    return language === null || language === reader ? null : language;
  });

  // The texts the reader's tab asks for: the title and every section written in another language than the
  // reader's, each once.
  const wanted = computed(() => {
    const texts = source.value === null ? [] : [source.value.title, ...sections.value];

    return [...new Set(texts)].filter((text) => {
      const language = text.trim() === "" ? null : writtenIn(text, scripts);

      return language !== null && language !== reader;
    });
  });

  function ask(texts: readonly string[]): void {
    if (translate === null || texts.length === 0) return;

    const mine = generation;

    for (const text of texts) asked.set(text, { kind: "asking" });

    for (let at = 0; at < texts.length; at += batch) {
      const chunk = texts.slice(at, at + batch);

      translate(chunk, reader).then(
        (readings) => {
          if (mine !== generation) return;

          // A text the translator stayed too busy to read comes back as itself, untranslated: it did not come.
          chunk.forEach((text, index) => {
            const reading = readings[index];

            asked.set(text, reading === undefined || reading.untranslated === true ? { kind: "missed" } : { kind: "read", reading });
          });
        },
        () => {
          if (mine !== generation) return;

          for (const text of chunk) asked.set(text, { kind: "missed" });
        }
      );
    }
  }

  watch(
    source,
    () => {
      generation += 1;
      opened = false;
      asked.clear();
      tab.value = "original";
    },
    { flush: "sync" }
  );

  watch(
    tab,
    (open) => {
      if (open !== "reader" || opened || written.value === null) return;

      opened = true;
      ask(wanted.value);
    },
    { flush: "sync" }
  );

  function shown(text: string): string {
    const one = asked.get(text);

    return one?.kind === "read" ? shownSection(text, one.reading.text) : text;
  }

  const reading = computed(() => tab.value === "reader" && written.value !== null);
  const title = computed(() => (reading.value ? shown(source.value?.title ?? "") : (source.value?.title ?? "")));
  const markdown = computed(() => (reading.value ? sections.value.map(shown).join("") : (source.value?.markdown ?? "")));

  const state = computed<ReadingState>(() => {
    if (!reading.value || wanted.value.length === 0) return "none";

    const all = wanted.value.map((text) => asked.get(text));

    if (all.some((one) => one === undefined || one.kind === "asking")) return "asking";
    if (all.every((one) => one?.kind === "missed")) return "missed";

    return all.every((one) => one?.kind === "read" && one.reading.whole) ? "whole" : "partial";
  });

  function again(): void {
    ask(
      wanted.value.filter((text) => {
        const one = asked.get(text);

        return one?.kind === "missed" || (one?.kind === "read" && !one.reading.whole);
      })
    );
  }

  return { written, reader, tab, title, markdown, state, again };
}
