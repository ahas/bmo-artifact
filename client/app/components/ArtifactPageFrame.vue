<template>
  <!-- A document that is a page of its own (docs/archive.md, "An HTML page"): its title on a row
       of its own, its kind and the actions on it on the row under it, and under them the page filling
       the rest of the screen. The page's own scroll is the only one here. The frame names the scheme
       the console stands in, so the page is drawn in it from its first frame and again when the
       screen mode changes (docs/archive.md, "How a document is drawn"). A share page has no header:
       the page fills the window. -->
  <div class="flex min-h-0 flex-1 flex-col">
    <header v-if="props.chrome" class="flex shrink-0 flex-col gap-2 border-b border-default px-4 py-3">
      <h1 class="truncate text-xl font-semibold text-highlighted" :title="props.title">{{ props.title }}</h1>

      <div class="flex items-center gap-2">
        <UBadge v-if="props.kind !== null" color="neutral" variant="outline" :label="props.kind" />
        <ActionGroup class="ms-auto" :actions="actions" :properties="props.properties" :properties-title="props.title" :label="$t('artifact.aria.document_menu')" />
      </div>
    </header>

    <ArtifactHostedFrame :src="framed" :title="props.title" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../shared/document-design";
import { schemeOf } from "../domain/document-view";

import type { BarAction, BarProperty } from "bmo-ui/app/domain/tabs";

interface Props {
  title: string;
  kind?: string | null;
  // Where the page is served in a scheme, and where it is saved.
  page: (scheme: string, download?: boolean) => string;
  properties?: BarProperty[];
  // Opens the share dialog, where the screen offers one.
  share?: (() => void) | null;
  // Whether the page stands under its title and actions, or fills the window alone.
  chrome?: boolean;
}

const { t } = useI18n();
const props = withDefaults(defineProps<Props>(), { kind: null, properties: () => [], share: null, chrome: true });
const colorMode = useColorMode();

useHead({ style: [{ key: "document-palette", innerHTML: DOCUMENT_PALETTE }] });

// The page in the scheme the console stands in.
const framed = computed(() => props.page(schemeOf(colorMode.value)));

function openPage(): void {
  window.open(framed.value, "_blank", "noopener,noreferrer");
}

// The page is saved as the file it was kept as.
function download(): void {
  const link = document.createElement("a");

  link.href = props.page(schemeOf(colorMode.value), true);
  link.download = "";
  link.click();
}

const actions = computed<BarAction[]>(() => [
  { id: "open", label: t("artifact.action.open_new_tab"), icon: "i-material-symbols-open-in-new", onSelect: openPage },
  { id: "download", label: t("artifact.action.download"), icon: "i-material-symbols-download", onSelect: download },
  ...(props.share === null ? [] : [{ id: "share", label: t("artifact.action.share"), icon: "i-material-symbols-share-outline", onSelect: props.share }])
]);
</script>
