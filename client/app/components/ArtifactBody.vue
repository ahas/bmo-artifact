<template>
  <!-- A document's text in the document design (docs/artifact.md, "How a document is drawn"): the
       masthead — its kind over its title, the actions on it on the row under it, a rule in the ink
       closing it — then the sounds marked to be heard at its head, its body, and the other files it
       keeps after that. The reading width is the text's own, so the gutter stands outside it. -->
  <div class="w-full min-w-0 p-4 sm:p-6" data-document-pictures @click="onPress">
    <article class="mx-auto flex w-full max-w-3xl min-w-0 flex-col gap-8">
      <header class="artifact-masthead flex flex-col gap-4 pb-6">
        <div class="flex flex-col gap-2">
          <p v-if="props.kind !== null" class="artifact-eyebrow">{{ props.kind }}</p>
          <h1 class="artifact-title">{{ props.title }}</h1>
          <!-- Its labels, the names a filer finds it by, on a secondary line that wraps (docs/artifact.md, 「Where a person
               reads it」). -->
          <p v-if="labelLine !== ''" class="text-sm break-words text-muted" data-document-labels>{{ labelLine }}</p>
        </div>

        <div v-if="tabs.length > 0 || actions.length > 0 || props.properties !== null" class="flex items-center gap-2">
          <!-- A document in another language than the reader's reads in its own and in the reader's, the reader's
               filled the first time it is opened (docs/artifact.md, 「Where a person reads it」). -->
          <UTabs v-if="tabs.length > 0" v-model="tab" :items="tabs" :content="false" size="sm" variant="pill" :aria-label="$t('artifact.label.reading')" data-reading-tabs />
          <ActionGroup class="ms-auto" :actions="actions" :properties="props.properties" :label="$t('artifact.aria.document_menu')" />
        </div>

        <p v-if="readingState === 'asking'" role="status" class="flex items-center gap-2 text-sm text-muted" data-reading-state="asking">
          <UIcon name="i-material-symbols-progress-activity" class="size-4 shrink-0 animate-spin" />
          {{ $t("artifact.status.translating") }}
        </p>
        <div v-else-if="readingState === 'partial' || readingState === 'missed'" role="status" class="flex flex-wrap items-center gap-2" :data-reading-state="readingState">
          <p v-if="readingState === 'missed'" class="text-sm text-error">{{ $t("artifact.error.translate_failed") }}</p>
          <p v-else class="text-sm text-muted">{{ $t("artifact.status.partly_original") }}</p>
          <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('artifact.action.ask_again')" @click="props.reading?.again()" />
        </div>
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
import { fileAddresses, filesAround, languageName } from "../domain/document-view";

import type { BarAction, BarProperty } from "bmo-ui/app/domain/tabs";
import type { PictureRun } from "../../shared/document-pictures";
import type { DocumentReading, ReadingTab } from "../domain/document-reading";
import type { ArtifactAttachment } from "../../shared/types";

// The document itself and nothing of the screen it stands on: what it is called, what kind of
// document it is, its body and its files. The actions are the caller's to offer, and a share page
// offers none of the console's own.
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
  properties?: readonly BarProperty[] | null;
  // The document's labels, each name and value, shown on the masthead's secondary line; none where it stands for
  // somebody holding its link.
  labels?: Readonly<Record<string, string>> | null;
  // Where the document's file is saved from.
  documentUrl?: string | null;
  // The pages it refers to, where they do not stand beside it.
  references?: (() => void) | null;
  // Where the link that hands this document to somebody outside is made.
  share?: (() => void) | null;
  // The document read in its own language and in the reader's, where it is written in another; the title and
  // the markdown handed are the ones its open tab shows.
  reading?: DocumentReading | null;
}

const { t } = useI18n();

const props = withDefaults(defineProps<Props>(), {
  kind: null,
  slugs: () => [],
  attachments: () => [],
  properties: null,
  labels: null,
  documentUrl: null,
  references: null,
  share: null,
  reading: null
});

// The two tabs, each named in its language's own words: the document's own first, then the reader's.
const tabs = computed(() => {
  const written = props.reading?.written.value ?? null;

  if (props.reading === null || written === null) return [];

  return [
    { label: languageName(written) ?? written, value: "original" },
    { label: languageName(props.reading.reader) ?? props.reading.reader, value: "reader" }
  ];
});

const tab = computed<ReadingTab>({
  get: () => props.reading?.tab.value ?? "original",
  set: (open) => {
    if (props.reading) props.reading.tab.value = open;
  }
});

const readingState = computed(() => props.reading?.state.value ?? "none");

const labelLine = computed(() =>
  Object.entries(props.labels ?? {})
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

// The document's file is downloaded, as its link would be.
function download(url: string): void {
  const link = document.createElement("a");

  link.href = url;
  link.download = "";
  link.click();
}

const actions = computed<BarAction[]>(() => [
  ...(props.references === null ? [] : [{ id: "references", label: t("artifact.label.references"), icon: "i-material-symbols-link", onSelect: props.references }]),
  ...(props.documentUrl === null ? [] : [{ id: "document", label: t("artifact.action.document_original"), icon: "i-material-symbols-file-save-outline", onSelect: () => download(props.documentUrl!) }]),
  ...(props.share === null ? [] : [{ id: "share", label: t("artifact.action.share"), icon: "i-material-symbols-share-outline", onSelect: props.share }])
]);
</script>
