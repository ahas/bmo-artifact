import { useDarkScheme } from "bmo-ui/app/domain/scheme";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { schemeOf } from "./document-view";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { ComputedRef } from "vue";
import type { PageAsked } from "./artifact";

// A document that is a page of its own (docs/artifact.md, "An HTML page"), as its frame draws it and as the
// screen's row acts on it.

/** Where a page is served as asked: in a scheme and a language, or as the file to save. */
export type PageServed = (asked: PageAsked) => string;

/** The page in the scheme and the language the console stands in, following both as they change. */
export function useFramedPage(page: () => PageServed): ComputedRef<string> {
  const { locale } = useI18n();
  const dark = useDarkScheme();

  return computed(() => page()({ scheme: schemeOf(dark.value ? "dark" : "light"), language: locale.value }));
}

/** The actions a page offers of its own: opening it in a tab of its own, and saving it as the file it was kept as. */
export function usePageActions(page: () => PageServed): ComputedRef<BarAction[]> {
  const { t } = useI18n();
  const framed = useFramedPage(page);

  function openPage(): void {
    window.open(framed.value, "_blank", "noopener,noreferrer");
  }

  function download(): void {
    const link = document.createElement("a");

    link.href = page()({ download: true });
    link.download = "";
    link.click();
  }

  return computed(() => [
    { id: "open", label: t("artifact.action.open_new_tab"), icon: "i-material-symbols-open-in-new", onSelect: openPage },
    { id: "download", label: t("artifact.action.download"), icon: "i-material-symbols-download", onSelect: download }
  ]);
}
