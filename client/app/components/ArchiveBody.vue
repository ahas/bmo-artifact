<template>
  <!-- A document's text in the archive's document design (docs/archive.md, "How a document is
       drawn"): the masthead — its kind over its title, the title on a row of its own, under it its
       status and the actions on it, a rule in the ink closing it — then the sounds it keeps, its body
       in the order it was written and the other files it keeps after that. The details of the
       document open from the properties action rather than standing over the text. The reading
       width is the text's own, so the gutter stands outside it rather than being taken out of it. -->
  <div class="w-full min-w-0 p-4 sm:p-6" data-document-pictures @click="onPress">
    <article class="mx-auto flex w-full max-w-3xl min-w-0 flex-col gap-8">
      <header class="document-masthead flex flex-col gap-4 pb-6">
        <div class="flex flex-col gap-2">
          <p v-if="props.kind !== null" class="document-eyebrow">{{ props.kind }}</p>
          <h1 class="document-title" :class="{ 'document-title--page': props.hosted }">{{ props.title }}</h1>
        </div>

        <div v-if="props.status !== null || props.shared !== null || actions.length > 0 || props.properties !== null" class="flex items-center gap-2">
          <UBadge v-if="props.status !== null" :color="badgeColor" variant="subtle" :icon="props.status.icon" :label="props.status.label" />

          <UBadge
            v-if="props.shared !== null"
            color="info"
            variant="subtle"
            icon="i-material-symbols-share-outline"
            class="tabular-nums"
            :label="props.shared"
          />

          <ActionGroup class="ms-auto" :actions="actions" :properties="props.properties" :label="$t('common.aria.document_menu')" />
        </div>
      </header>

      <!-- A sound the document keeps is heard before it is read, so it stands under the masthead
           rather than among the files after the body. -->
      <section v-if="split.sounds.length > 0 && props.attachmentBase !== null" :aria-label="$t('archive.label.listen')" class="flex flex-col gap-2">
        <p class="document-eyebrow">{{ $t("archive.label.listen") }}</p>
        <AudioPlayer
          v-for="sound in split.sounds"
          :key="sound.id"
          :src="attachmentUrl(props.attachmentBase, sound)"
          :download="attachmentUrl(props.attachmentBase, sound, true)"
          :name="sound.name"
          :size="sizeText(sound.bytes)"
        />
      </section>

      <p v-if="props.findings.length === 0" class="text-sm text-muted">{{ props.empty ?? $t("archive.status.empty") }}</p>

      <section
        v-for="finding in props.findings"
        :key="finding.place"
        :aria-label="finding.title"
        :data-finding="finding.place"
        class="flex scroll-mt-4 flex-col gap-2 border-b border-(--doc-rule) pb-8 last:border-b-0"
      >
        <span v-if="finding.branch !== null" class="text-sm text-muted">
          {{
            props.runCount > 1 && finding.runId !== null
              ? $t("archive.label.branch_run", { branch: finding.branch, run: finding.runId })
              : $t("archive.label.branch", { branch: finding.branch })
          }}
        </span>
        <FindingText :markdown="finding.markdown" :images="props.pictures.get(finding.place)" :files="files" :slugs="props.marks.get(finding.place)" />

        <UCollapsible v-if="finding.foldedMarkdown !== null" class="flex flex-col gap-1">
          <UButton color="neutral" variant="ghost" size="sm" :label="$t('archive.action.details')" trailing-icon="i-material-symbols-keyboard-arrow-down" class="self-start group" :ui="{ trailingIcon: 'group-data-[state=open]:rotate-180 transition-transform' }" />

          <template #content>
            <FindingText :markdown="finding.foldedMarkdown ?? ''" :images="props.pictures.get(finding.place)" :files="files" />
          </template>
        </UCollapsible>
      </section>

      <ArchiveAttachments v-if="split.others.length > 0 && props.attachmentBase !== null" :attachments="split.others" :base="props.attachmentBase" />
    </article>

    <DocumentPictureViewer ref="viewer" :title="props.title" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../shared/document-design";
import { VIEWER_PICTURES } from "../../shared/document-pictures";
import { attachmentAddresses, attachmentUrl, soundsFirst, undrawnAttachments } from "../domain/archive";
import { plainPress, pressedPictures } from "../domain/document-pictures";
import { sizeText } from "../domain/sizes";

import type { BadgeProps } from "@nuxt/ui";
import type { BarAction, BarProperty } from "bmo-ui/app/domain/tabs";
import type { findingImages } from "../domain/archive";
import type { ArchiveAttachmentView, SearchFindingView } from "../../shared/types";

// The document itself and nothing of whose screen it stands on: what it is called, what kind of
// document it is, what it holds and how many runs wrote it. The ones below them are the console's
// own and are absent on a share page, which is why each of them is a caller's to offer rather than
// a part of the document.
interface Props {
  title: string;
  // What kind of document it is, in the kind's own label.
  kind?: string | null;
  // Whether the document is a page of its own, which is drawn here only while the page is still
  // being written; its title then stands at the step the page frame gives it once the page is there.
  hosted?: boolean;
  findings: readonly SearchFindingView[];
  // How many runs the document was written by. A finding names the run it came from only where
  // there was more than one.
  runCount: number;
  pictures: ReadonlyMap<number, ReturnType<typeof findingImages>>;
  // The mark each of a finding's headings is stamped with, so the table of contents beside the
  // text can steer to one. The same list the contents were built from.
  marks: ReadonlyMap<number, readonly string[]>;
  // The files the document keeps beside it, and the road they are answered under.
  attachments?: readonly ArchiveAttachmentView[];
  attachmentBase?: string | null;
  // The state of the document, its properties, and where its original file is fetched. Null each
  // where the reader is not to be shown it.
  status?: { label: string; color: string; icon: string } | null;
  // What says the document is out on a link, already in the words it is to be read in. Null where
  // it is on no link, and on a share page, whose reader is never told about the link they hold.
  shared?: string | null;
  properties?: readonly BarProperty[] | null;
  documentUrl?: string | null;
  // The pages the text stands on, where they do not stand beside it. Null where they do.
  sources?: (() => void) | null;
  // Where the link that hands this document to somebody outside is made. It is the owner's own,
  // so a share page offers none.
  share?: (() => void) | null;
  // What stands in place of the findings where there are none. The default says the search found
  // nothing to keep; a search that answered one value and kept no document says so instead.
  empty?: string;
}

const { t } = useI18n();
const BADGE_COLORS = ["success", "info", "warning", "error", "neutral", "primary"] as const;

const props = withDefaults(defineProps<Props>(), {
  kind: null,
  hosted: false,
  attachments: () => [],
  attachmentBase: null,
  status: null,
  shared: null,
  properties: null,
  documentUrl: null,
  sources: null,
  share: null
});
const emit = defineEmits<{ mark: [mark: string] }>();

// A picture or a link in the body may name one of the document's files by its name, and a picture
// the body draws that way does not stand again among the attachments.
const files = computed(() => (props.attachmentBase === null ? undefined : attachmentAddresses(props.attachmentBase, props.attachments)));
const standing = computed(() => undrawnAttachments(props.attachments));
const split = computed(() => soundsFirst(standing.value));

// The document's colours, in the scheme the console stands in, and the pointer that says a picture
// opens whole.
useHead({
  style: [
    { key: "document-palette", innerHTML: DOCUMENT_PALETTE },
    { key: "document-pictures", innerHTML: `[data-document-pictures] ${VIEWER_PICTURES}{cursor:zoom-in}` }
  ]
});

const viewer = useTemplateRef("viewer");

// A footnote mark is an in-page link; pressing it brings its source forward instead of moving the
// text. A picture the document lets open opens whole, among the others it draws.
function onPress(event: MouseEvent): void {
  const target = event.target as HTMLElement | null;
  const link = target?.closest<HTMLAnchorElement>("a[data-mark]");

  if (link !== null && link !== undefined) {
    event.preventDefault();
    emit("mark", link.dataset.mark ?? "");

    return;
  }

  const run = plainPress(event) ? pressedPictures(event.currentTarget as HTMLElement, target) : null;

  if (run === null) return;

  event.preventDefault();
  viewer.value?.show(run);
}

// A status stands as a chip in its own colour.
const badgeColor = computed<BadgeProps["color"]>(() =>
  (BADGE_COLORS as readonly string[]).includes(props.status?.color ?? "") ? (props.status?.color as BadgeProps["color"]) : "neutral"
);

// The original of the document is downloaded, as its link would be.
function download(url: string): void {
  const link = document.createElement("a");

  link.href = url;
  link.download = "";
  link.click();
}

const actions = computed<BarAction[]>(() => [
  ...(props.sources === null ? [] : [{ id: "sources", label: t("archive.label.sources"), icon: "i-material-symbols-link", onSelect: props.sources }]),
  ...(props.documentUrl === null ? [] : [{ id: "document", label: t("archive.action.document_original"), icon: "i-material-symbols-file-save-outline", onSelect: () => download(props.documentUrl!) }]),
  ...(props.share === null ? [] : [{ id: "share", label: t("common.action.share"), icon: "i-material-symbols-share-outline", onSelect: props.share }])
]);
</script>
