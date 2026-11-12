import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { saveFile } from "./save-file";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { ComputedRef } from "vue";
import type { DocumentReading } from "./document-reading";

// A document written as markdown (docs/artifact.md, "Where a person reads it"), as the screen's row acts on it.

export interface WrittenActionsOptions {
  // Where its file is saved from.
  file: () => string;
  // Opens the pages it refers to, where they do not stand beside it; null where they do.
  references: () => (() => void) | null;
}

/** The actions a written document offers of its own: its referred pages where they do not stand beside it, and saving its file. */
export function useWrittenActions(options: WrittenActionsOptions): ComputedRef<BarAction[]> {
  const { t } = useI18n();

  return computed(() => {
    const references = options.references();

    return [
      ...(references === null ? [] : [{ id: "references", label: t("artifact.label.references"), icon: "i-material-symbols-link", onSelect: references }]),
      { id: "document", label: t("artifact.action.document_original"), icon: "i-material-symbols-file-save-outline", onSelect: () => saveFile(options.file()) }
    ];
  });
}

/**
 * The toggle between the document's own language and the reader's, on while the reader's is shown and spinning while
 * it is asked for; none where the document is written in the reader's language or no reading is offered.
 */
export function useTranslationToggle(reading: () => DocumentReading | null): ComputedRef<BarAction | null> {
  const { t } = useI18n();

  return computed(() => {
    const held = reading();

    if (held === null || held.written.value === null) return null;

    const shown = held.tab.value === "reader";

    return {
      id: "translate",
      label: t("artifact.action.translate"),
      icon: "i-material-symbols-translate",
      pressed: shown,
      loading: held.state.value === "asking",
      onSelect: () => {
        held.tab.value = shown ? "original" : "reader";
      }
    };
  });
}
