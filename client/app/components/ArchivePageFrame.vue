<template>
  <!-- A document that is a page of its own (docs/archive.md, "An HTML page"): its title on a row
       of its own, its kind, its link and the actions on it on the row under it, and under them the
       page filling the rest of the screen. The page's own scroll is the only one here. The frame
       names the scheme the console stands in, so the page is drawn in it from its first frame and
       again when the screen mode changes (docs/archive.md, "How a document is drawn"). -->
  <div class="flex min-h-0 flex-1 flex-col">
    <header class="flex shrink-0 flex-col gap-2 border-b border-default px-4 py-3">
      <h1 class="truncate text-xl font-semibold text-highlighted" :title="props.title">{{ props.title }}</h1>

      <div class="flex items-center gap-2">
        <UBadge color="neutral" variant="outline" :label="props.kind" />
        <!-- Whether it is out on a link, pressed to see or change that link. -->
        <UButton
          v-if="props.shared !== null"
          color="info"
          variant="subtle"
          size="sm"
          icon="i-material-symbols-share-outline"
          class="tabular-nums"
          :label="props.shared"
          @click="props.share()"
        />
        <ActionGroup class="ms-auto" :actions="actions" :properties="props.properties" :properties-title="props.title" :label="$t('common.aria.document_menu')" />
      </div>
    </header>

    <HostedPageFrame :src="framed" :title="props.title" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../shared/document-design";
import { schemedPage } from "../domain/archive";

import type { BarAction, BarProperty } from "bmo-ui/app/domain/tabs";

interface Props {
  title: string;
  // The kind's label, and whether the document is out on a link in the words the chip reads.
  kind: string;
  shared: string | null;
  // Where the page is served, under the console's sign-in.
  pageUrl: string;
  properties: BarProperty[];
  // Opens the share dialog; the share action is the caller's to offer.
  share: () => void;
}

const { t } = useI18n();
const props = defineProps<Props>();
const colorMode = useColorMode();

useHead({ style: [{ key: "document-palette", innerHTML: DOCUMENT_PALETTE }] });

// The page in the scheme the console stands in.
const framed = computed(() => schemedPage(props.pageUrl, colorMode.value));

function openPage(): void {
  window.open(framed.value, "_blank", "noopener,noreferrer");
}

// The page is saved as the file it was kept as.
function download(): void {
  const link = document.createElement("a");

  link.href = `${props.pageUrl}?download=1`;
  link.download = "";
  link.click();
}

const actions = computed<BarAction[]>(() => [
  { id: "open", label: t("archive.action.open_new_tab"), icon: "i-material-symbols-open-in-new", onSelect: openPage },
  { id: "download", label: t("archive.action.download"), icon: "i-material-symbols-download", onSelect: download },
  { id: "share", label: t("common.action.share"), icon: "i-material-symbols-share-outline", onSelect: props.share }
]);
</script>
