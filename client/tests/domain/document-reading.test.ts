import { describe, expect, test } from "bun:test";
import { ref } from "vue";

import { documentReading, scriptOf, sectionsOf, shownSection, writtenIn } from "../../app/domain/document-reading";

import type { Reading } from "bmo-ui/app/domain/translation";
import type { ReadingSource, Script } from "../../app/domain/document-reading";

// A document read in its own language and in the reader's (docs/artifact.md, 「Where a person reads it」).

const SCRIPTS: Script[] = [scriptOf("en", "Latin")!, scriptOf("el", "Greek")!];

// Words in Greek, the reader's language here, written as escapes so the file stays in English.
const GREEK = (count: number): string => Array.from({ length: count }, () => "\u03bb\u03cc\u03b3\u03bf\u03c2").join(" ");

const ENGLISH_BODY = ["Lead words before any heading.", "", "## First", "", "The first section's words.", "", "```sh", `echo ${GREEK(4)}`, "```", "", "## Second", "", "- one item", "- two items", ""].join("\n");

// Every promise already settled has run its handlers.
async function settled(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await Promise.resolve();
}

// A translator whose every call waits until the test answers it, each call kept in order.
function heldTranslator() {
  const calls: { texts: readonly string[]; language: string; answer: (readings: Reading[]) => void; refuse: () => void }[] = [];
  const translate = (texts: readonly string[], language: string) =>
    new Promise<Reading[]>((answer, refuse) => calls.push({ texts, language, answer, refuse: () => refuse(new Error("refused")) }));

  return { calls, translate };
}

describe("the language a text is written in", () => {
  test("is the one whose script writes the most of its words, code left out", () => {
    expect(writtenIn(ENGLISH_BODY, SCRIPTS)).toBe("en");
    expect(writtenIn(`## ${GREEK(1)}\n\n${GREEK(5)} \`parse_config\` API.\n`, SCRIPTS)).toBe("el");
    expect(writtenIn(`| ${GREEK(1)} | ${GREEK(1)} |\n|---|---|\n| ${GREEK(1)} | x |\n`, SCRIPTS)).toBe("el");
  });

  test("is none where no word is in any script the pack names", () => {
    expect(writtenIn("```\nconst a = 1;\n```\n\n`b`\n", SCRIPTS)).toBeNull();
    expect(writtenIn("12 34", SCRIPTS)).toBeNull();
  });

  test("leaves out a script no Unicode script goes by", () => {
    expect(scriptOf("xx", "Not a script")).toBeNull();
  });
});

describe("a body's sections", () => {
  test("start at each heading, the text before the first one of its own, and join back into the body", () => {
    const sections = sectionsOf(ENGLISH_BODY);

    expect(sections.map((section) => section.split("\n")[0])).toEqual(["Lead words before any heading.", "## First", "## Second"]);
    expect(sections.join("")).toBe(ENGLISH_BODY);
  });

  test("open with the first heading where the body does, its line ends read as line feeds", () => {
    expect(sectionsOf("# One\r\nwords\r\n# Two\r\nmore\r\n")).toEqual(["# One\nwords\n", "# Two\nmore\n"]);
    expect(sectionsOf("")).toEqual([]);
  });

  test("keep the original's blank lines around the reading the translator trimmed", () => {
    expect(shownSection("\n## First\n\nwords\n\n", "## Read\n\nread words")).toBe("\n## Read\n\nread words\n\n");
  });
});

describe("a document's reading", () => {
  const source = (markdown = ENGLISH_BODY): ReadingSource => ({ title: "A plain title", markdown });

  test("offers the reader's tab only for a document written in another language, with a translator named", () => {
    const korean = documentReading(() => source(`## ${GREEK(1)}\n\n${GREEK(4)}\n`), { reader: "el", scripts: SCRIPTS, translate: async () => [], batch: 4 });
    const untranslated = documentReading(() => source(), { reader: "el", scripts: SCRIPTS, translate: null, batch: 4 });
    const english = documentReading(() => source(), { reader: "el", scripts: SCRIPTS, translate: async () => [], batch: 4 });

    expect(korean.written.value).toBeNull();
    expect(untranslated.written.value).toBeNull();
    expect(english.written.value).toBe("en");
  });

  test("asks nothing until the reader's tab is opened, then each wanted text once, in batches", async () => {
    const { calls, translate } = heldTranslator();
    const reading = documentReading(() => source(), { reader: "el", scripts: SCRIPTS, translate, batch: 2 });

    expect(calls).toEqual([]);
    expect(reading.state.value).toBe("none");

    reading.tab.value = "reader";

    const sections = sectionsOf(ENGLISH_BODY);

    expect(calls.map((call) => call.texts)).toEqual([["A plain title", sections[0]!], [sections[1]!, sections[2]!]]);
    expect(calls.every((call) => call.language === "el")).toBe(true);
    expect(reading.state.value).toBe("asking");

    // The original stands in place until a reading comes, and each takes its place the moment it does.
    expect(reading.markdown.value).toBe(ENGLISH_BODY);

    calls[0]!.answer([
      { text: "A plain title [el]", whole: true },
      { text: "Lead words [el]", whole: true }
    ]);
    await settled();

    expect(reading.title.value).toBe("A plain title [el]");
    expect(reading.markdown.value).toBe(`Lead words [el]\n\n${sections[1]}${sections[2]}`);
    expect(reading.state.value).toBe("asking");

    reading.tab.value = "original";
    reading.tab.value = "reader";

    expect(calls.length).toBe(2);
  });

  test("says when parts stand in the original, and asks again for those alone", async () => {
    const { calls, translate } = heldTranslator();
    const reading = documentReading(() => source(), { reader: "el", scripts: SCRIPTS, translate, batch: 4 });
    const sections = sectionsOf(ENGLISH_BODY);

    reading.tab.value = "reader";
    calls[0]!.answer([
      { text: "A plain title [el]", whole: true },
      { text: "Lead words [el]", whole: true },
      { text: "## First [el]\n\nThe first section's words.", whole: false },
      { text: sections[2]!, whole: false, untranslated: true }
    ]);
    await settled();

    expect(reading.state.value).toBe("partial");

    reading.again();

    expect(calls.map((call) => call.texts)).toEqual([["A plain title", ...sections], [sections[1]!, sections[2]!]]);

    calls[1]!.refuse();
    await settled();

    expect(reading.state.value).toBe("partial");
    expect(reading.markdown.value).toBe(`Lead words [el]\n\n${sections[1]}${sections[2]}`);
  });

  test("says nothing came where every ask was refused", async () => {
    const { calls, translate } = heldTranslator();
    const reading = documentReading(() => source(), { reader: "el", scripts: SCRIPTS, translate, batch: 16 });

    reading.tab.value = "reader";
    calls[0]!.refuse();
    await settled();

    expect(reading.state.value).toBe("missed");
    expect(reading.markdown.value).toBe(ENGLISH_BODY);
  });

  test("starts over in the original for another document, dropping what the last one was still asked for", async () => {
    const { calls, translate } = heldTranslator();
    const held = ref<ReadingSource | null>(source());
    const reading = documentReading(() => held.value, { reader: "el", scripts: SCRIPTS, translate, batch: 16 });

    reading.tab.value = "reader";
    held.value = source("# Another\n\nOther words.\n");

    expect(reading.tab.value as string).toBe("original");

    calls[0]!.answer(calls[0]!.texts.map((text) => ({ text: `${text} [el]`, whole: true })));
    await settled();

    reading.tab.value = "reader";

    expect(calls.length).toBe(2);
    expect(reading.markdown.value).toBe("# Another\n\nOther words.\n");
  });
});
