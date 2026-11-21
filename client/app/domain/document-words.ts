import { useI18n } from "vue-i18n";

import { kindOf } from "./document-view";

import type { Ref } from "vue";
import type { ArtifactKind } from "../../shared/types";

// The words a document's kind and category are shown in: the pack's where it has them, and the catalogue's name or
// the filer's own word where not, so a new kind or category stands at once (`docs/artifact.md`, "Where a person
// reads it").

/** The kind's and the category's words, over the catalogue as it has been read. */
export function useDocumentWords(kinds: Ref<readonly ArtifactKind[]>): {
  kindLabel: (name: string) => string;
  categoryLabel: (word: string) => string;
} {
  const { t } = useI18n();

  function kindLabel(name: string): string {
    const key = `artifact.kind.${name}`;
    const said = t(key);

    return said === key ? (kindOf(kinds.value, name)?.name ?? name) : said;
  }

  function categoryLabel(word: string): string {
    const key = `artifact.category.${word}`;
    const said = t(key);

    return said === key ? word : said;
  }

  return { kindLabel, categoryLabel };
}

/** The icon a kind is drawn by, as the component library names it; a kind not yet read takes a plain document's. */
export function kindIcon(kind: ArtifactKind | null): string {
  return `i-material-symbols-${kind?.icon ?? "description-outline"}`;
}
