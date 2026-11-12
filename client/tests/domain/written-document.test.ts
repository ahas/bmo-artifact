import { describe, expect, test } from "bun:test";
import { computed, createApp, ref } from "vue";
import { createI18n } from "vue-i18n";

import { useTranslationToggle, useWrittenActions } from "../../app/domain/written-document";

import type { DocumentReading, ReadingState, ReadingTab } from "../../app/domain/document-reading";

// What a written document's row offers of its own (`docs/artifact.md`, "Where a person reads it"): its referred pages
// where they do not stand beside it, its file, and the toggle between its own language and the reader's.

// Runs a composable inside a mounted app holding the pack's words it reads, and hands back what it answered.
function within<T>(use: () => T): T {
  let answered: T | undefined;
  const i18n = createI18n({ legacy: false, locale: "en", messages: { en: { artifact: { action: { translate: "Translation", document_original: "Document file" }, label: { references: "Referred pages" } } } } });

  createApp({
    setup() {
      answered = use();

      return () => null;
    }
  })
    .use(i18n)
    .mount(document.createElement("div"));

  return answered!;
}

function readingOf(written: string | null, state: ReadingState = "none"): DocumentReading {
  const tab = ref<ReadingTab>("original");

  return {
    written: computed(() => written),
    reader: "ko",
    tab,
    title: computed(() => ""),
    markdown: computed(() => ""),
    state: computed(() => state),
    again: () => {}
  };
}

describe("the translation toggle", () => {
  test("stands nowhere for a document written in the reader's language, or with no reading", () => {
    expect(within(() => useTranslationToggle(() => readingOf(null))).value).toBeNull();
    expect(within(() => useTranslationToggle(() => null)).value).toBeNull();
  });

  test("opens off, turns the reader's language on and off again, and is pressed while it is shown", () => {
    const reading = readingOf("en");
    const toggle = within(() => useTranslationToggle(() => reading));

    expect(toggle.value).toMatchObject({ label: "Translation", pressed: false, loading: false });
    toggle.value!.onSelect();
    expect(reading.tab.value).toBe("reader");
    expect(toggle.value!.pressed).toBe(true);
    toggle.value!.onSelect();
    expect(reading.tab.value).toBe("original");
  });

  test("spins while the reader's language is asked for", () => {
    expect(within(() => useTranslationToggle(() => readingOf("en", "asking"))).value!.loading).toBe(true);
  });
});

describe("a written document's own actions", () => {
  test("offer its referred pages only where they do not stand beside it, then its file", () => {
    const beside = within(() => useWrittenActions({ file: () => "/file", references: () => null }));
    const apart = within(() => useWrittenActions({ file: () => "/file", references: () => () => {} }));

    expect(beside.value.map((action) => action.id)).toEqual(["document"]);
    expect(apart.value.map((action) => action.id)).toEqual(["references", "document"]);
  });
});
