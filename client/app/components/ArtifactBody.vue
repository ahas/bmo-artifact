<template>
  <!-- A document's text in the document design (docs/artifact.md, "How a document is drawn"): the
       masthead — its kind over its title, what its translation says of itself, a rule in the ink closing
       it — then the sounds marked to be heard at its head, its body, and the other files it keeps after
       that. The reading width is the text's own, so the gutter stands outside it. -->
  <div class="w-full min-w-0 p-4 sm:p-6" data-document-pictures @click="onPress">
    <article class="mx-auto flex w-full max-w-3xl min-w-0 flex-col gap-8">
      <header class="artifact-masthead flex flex-col gap-4 pb-6">
        <div class="flex flex-col gap-2">
          <p v-if="props.kind !== null" class="artifact-eyebrow">{{ props.kind }}</p>
          <h1 class="artifact-title">{{ props.title }}</h1>
          <!-- What it still lacks, a still mark under the title (docs/artifact.md, 「A document that lacks a part」). -->
          <ArtifactLackMark v-if="props.labels !== null" class="self-start" :document="{ labels: props.labels }" />
          <!-- Its labels, the names a filer finds it by, on a secondary line that wraps (docs/artifact.md, 「Where a person
               reads it」). -->
          <p v-if="labelLine !== ''" class="text-sm break-words text-muted" data-document-labels>{{ labelLine }}</p>
        </div>

        <!-- Where no row of the screen's stands over the document, as on a share page, the one action its reader has
             stands under the title (docs/artifact.md, 「Where a person reads it」). -->
        <div v-if="actions.length > 0" class="flex items-center justify-end">
          <ActionGroup :actions="actions" :narrow-fold="false" :label="$t('artifact.aria.document_menu')" />
        </div>

        <!-- What the reader's language says of itself, which the translation toggle on the screen's row turns on: what
             did not come stays said, with the button that asks for it again spinning while it is asked. -->
        <div v-if="missed !== null" role="status" class="flex flex-wrap items-center gap-2" :data-reading-state="readingState">
          <p v-if="missed === 'missed'" class="text-sm text-error">{{ $t("artifact.error.translate_failed") }}</p>
          <p v-else class="text-sm text-muted">{{ $t("artifact.status.partly_original") }}</p>
          <UButton
            class="ms-auto"
            size="sm"
            color="neutral"
            variant="subtle"
            icon="i-material-symbols-refresh"
            :label="$t('artifact.action.ask_again')"
            :loading="readingState === 'asking'"
            @click="askAgain"
          />
        </div>
        <p v-else-if="readingState === 'asking'" role="status" class="flex items-center gap-2 text-sm text-muted" data-reading-state="asking">
          <UIcon name="i-material-symbols-progress-activity" class="size-4 shrink-0 animate-spin" />
          {{ $t("artifact.status.translating") }}
        </p>
      </header>

      <!-- A sound marked for the head is heard before the text is read, so it stands under the
           masthead rather than among the files after the body. -->
      <ArtifactListen v-if="files.head.length > 0" :sounds="files.head" :address="props.address" />

      <p v-if="props.markdown.trim() === ''" class="text-sm text-muted">{{ $t("artifact.status.empty") }}</p>
      <ArtifactText v-else :markdown="props.markdown" :files="named" :slugs="props.slugs" />

      <ArtifactAttachments v-if="files.after.length > 0" :attachments="files.after" :address="props.address" @pictures="showPictures" />
    </article>

    <ArtifactPictureViewer ref="viewer" :title="props.title" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../shared/document-design";
import { VIEWER_PICTURES } from "../../shared/document-pictures";
import { plainPress, pressedPictures } from "../domain/document-pictures";
import { PENDING_LABEL, fileAddresses, filesAround } from "../domain/document-view";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { PictureRun } from "../../shared/document-pictures";
import type { DocumentReading } from "../domain/document-reading";
import type { ArtifactAttachment } from "../../shared/types";

// The document itself and nothing of the screen it stands on: what it is called, what kind of
// document it is, its body and its files. The actions on it are the screen's row's, but for the referred
// pages a screen with no such row hands here.
interface Props {
  title: string;
  // What kind of document it is, in the kind's own words.
  kind?: string | null;
  markdown: string;
  // The mark each heading takes, in the order they stand, so the contents beside the text can steer
  // to one.
  slugs?: readonly string[];
  attachments?: readonly ArtifactAttachment[];
  // Where one of the document's files is served, and where it is taken to be saved.
  address: (attachment: ArtifactAttachment, download?: boolean) => string;
  // The document's labels, each name and value, shown on the masthead's secondary line; none where it stands for
  // somebody holding its link.
  labels?: Readonly<Record<string, string>> | null;
  // The pages it refers to, where they do not stand beside it and no row of the screen's offers them.
  references?: (() => void) | null;
  // The document read in its own language and in the reader's, where it is written in another; the title and
  // the markdown handed are the ones the language shown reads.
  reading?: DocumentReading | null;
}

const { t } = useI18n();

const props = withDefaults(defineProps<Props>(), {
  kind: null,
  slugs: () => [],
  attachments: () => [],
  labels: null,
  references: null,
  reading: null
});

const readingState = computed(() => props.reading?.state.value ?? "none");

// What did not come the last time the reader's language was asked for, kept while it is asked for again; none once all came.
const missed = ref<"partial" | "missed" | null>(null);

watch(readingState, (state) => {
  if (state === "partial" || state === "missed") missed.value = state;
  else if (state !== "asking") missed.value = null;
});

function askAgain(): void {
  props.reading?.again();
}

// The labels but what it lacks, which stands in its own mark above them.
const labelLine = computed(() =>
  Object.entries(props.labels ?? {})
    .filter(([name]) => name !== PENDING_LABEL)
    .map(([name, value]) => (value === "" ? name : `${name}: ${value}`))
    .join(" · ")
);

// A picture or a link in the body may name one of the document's files by its name, or by a path ending
// in it; a picture the body draws that way still stands among the files, small, so it can be saved.
const named = computed(() => fileAddresses(props.attachments, (attachment) => props.address(attachment)));
const files = computed(() => filesAround(props.attachments));

// The document's colours, in the scheme the console stands in, and the pointer that says a picture
// opens whole.
useHead({
  style: [
    { key: "document-palette", innerHTML: DOCUMENT_PALETTE },
    { key: "document-pictures", innerHTML: `[data-document-pictures] ${VIEWER_PICTURES}{cursor:zoom-in}` }
  ]
});

const viewer = useTemplateRef("viewer");

// A picture the document lets open opens whole, among the others it draws.
function onPress(event: MouseEvent): void {
  const target = event.target as HTMLElement | null;
  const run = plainPress(event) ? pressedPictures(event.currentTarget as HTMLElement, target) : null;

  if (run === null) return;

  event.preventDefault();
  viewer.value?.show(run);
}

// A picture pressed among the files opens whole among the pictures the files hold.
function showPictures(run: PictureRun): void {
  viewer.value?.show(run);
}

const actions = computed<BarAction[]>(() => [
  ...(props.references === null ? [] : [{ id: "references", label: t("artifact.label.references"), icon: "i-material-symbols-link", onSelect: props.references }])
]);
</script>
