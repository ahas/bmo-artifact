<template>
  <!-- One document, with its contents and the pages it refers to beside it. The two are separate
       parts that scroll on their own with a boundary between them; on a phone the side rises from the
       bottom edge over the text. A page fills the screen in its frame. -->
  <UDashboardPanel id="artifact-document" :ui="{ body: 'p-0 sm:p-0 gap-0 overflow-hidden [&>*]:max-w-none' }">
    <!-- Under a console that offers its title bar the console draws it; alone, the screen draws its own, holding its
         name and mark alone. Under either stands the screen's own row (docs/artifact.md, 「Where a person reads it」):
         the way back to the list at its left end, then the document's number and a page's kind; at its right end
         all that acts on the document: the translation toggle, then the actions on it as one group. -->
    <template #header>
      <UDashboardNavbar v-if="!barOffered" :title="$t('artifact.title.screen')" icon="i-material-symbols-archive-outline" />

      <UDashboardToolbar :ui="{ left: 'min-w-0 flex-1', right: 'shrink-0' }" data-document-row>
        <template #left>
          <HintTip class="flex" :text="$t('artifact.action.back_to_list')" press>
            <UButton to="/artifact" size="sm" color="neutral" variant="ghost" icon="i-material-symbols-arrow-back" :aria-label="$t('artifact.action.back_to_list')" />
          </HintTip>
          <span class="shrink-0 text-sm tabular-nums text-muted" data-document-number>{{ numberText(documentId) }}</span>
          <UBadge v-if="hosted" color="neutral" variant="outline" size="sm" class="max-w-full" :label="kindText" />
        </template>

        <!-- The toggle stands on its own at every width, as the view it switches is read rather than acted on. The
             actions fold by their count alone: the row's right end is only as wide as what it holds, so it has no
             room of its own to read, and on a phone a document offers four or more, which fold there anyway. -->
        <template #right>
          <ActionGroup v-if="translation !== null" :actions="[translation]" :narrow-fold="false" />
          <ActionGroup
            :actions="actions"
            :properties="held === null ? null : properties"
            :properties-title="held?.document.title ?? null"
            :narrow-fold="false"
            :label="$t('artifact.aria.document_menu')"
          />
        </template>
      </UDashboardToolbar>
    </template>

    <template #body>
      <div v-if="failed !== ''" role="alert" class="flex flex-wrap items-center gap-2 p-4">
        <p class="text-sm text-error">{{ failed }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('artifact.action.refresh')" @click="load" />
      </div>

      <ContentSkeleton v-else-if="held === null" class="p-4" :label="$t('artifact.status.document_loading')">
        <SkeletonText :lines="8" size="base" lastWidth="w-2/5" />
      </ContentSkeleton>

      <ArtifactPageFrame v-else-if="hosted" :title="held.document.title" :labels="held.document.labels" :page="page" />

      <div v-else-if="narrow" class="flex min-h-0 flex-1 flex-col">
        <div class="artifact-paper min-h-0 flex-1 overflow-y-auto">
          <ArtifactBody v-bind="bodyProps" />
        </div>

        <UDrawer v-model:open="sideOpen" :title="$t('artifact.title.references_count', { n: countText(held.references.length) })" :ui="FULL_SHEET">
          <template #body>
            <div class="min-h-0 flex-1 overflow-y-auto">
              <ArtifactReferences v-bind="sideProps" @go="goTo" />
            </div>
          </template>
        </UDrawer>
      </div>

      <USplitter v-else id="artifact-splitter" auto-save-id="bmo-artifact-splitter" :items="SIDES" :ui="{ handle: SPLIT_HANDLE }" class="min-h-0 flex-1">
        <template #body>
          <div class="artifact-paper min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArtifactBody v-bind="bodyProps" />
          </div>
        </template>

        <template #side>
          <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArtifactReferences v-bind="sideProps" @go="goTo" />
          </div>
        </template>
      </USplitter>

      <ArtifactInstruction v-if="instruction !== null" v-model:open="instructionOpen" :text="instruction" />

      <ShareDialog v-model:open="shareOpen" :road="road" :members="members" :explain="explainShare" :spans="spans" :subject="held?.document.title" />

      <ArtifactUnitShareDialog v-if="unitShareable" v-model:open="unitShareOpen" :document-id="documentId" :subject="held?.document.title" />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { formatStamp } from "bmo-ui/app/domain/time";
import { TRANSLATION_BATCH, hasTranslator, translated, translatorOf } from "bmo-ui/app/domain/translation";
import { inject } from "vue";
import { useI18n } from "vue-i18n";

import { SHARE_SPANS } from "../../../shared/config";
import {
  attachmentAddress,
  contentAddress,
  readAttachments,
  readContent,
  readDocument,
  readReference,
  readKinds,
  readReferences,
  referenceImageAddress,
  shareRoad
} from "../../domain/artifact";
import { documentReading, scriptOf } from "../../domain/document-reading";
import { contentsOf, instructionOf, kindOf, labelsBesideInstruction, numberText } from "../../domain/document-view";
import { usePageActions } from "../../domain/hosted-page";
import { memberRoad, shareFailure, usePeopleDirectory } from "../../domain/members";
import { useTranslationToggle, useWrittenActions } from "../../domain/written-document";

import type { SplitterItem } from "@nuxt/ui";
import type { BarAction, BarProperty } from "bmo-ui/app/domain/tabs";
import type { PageServed } from "../../domain/hosted-page";
import type { Script } from "../../domain/document-reading";
import type { ArtifactAttachment, ArtifactDocument, ArtifactKind, ArtifactReference } from "../../../shared/types";

// One document: its head, its file drawn as its kind reads it, the files it keeps and the pages it
// refers to, each read from the service through the console's relay.

const { t, te, locale, availableLocales } = useI18n();
// Whether a console offers its title bar, under the key bmo-ui's bundle contract names.
const barOffered = inject("bmo-console-title-bar", null) !== null;
const SIDES: SplitterItem[] = [
  { slot: "body", minSize: 40, defaultSize: 72 },
  { slot: "side", minSize: 16, defaultSize: 28 }
];

const route = useRoute();
const narrow = useNarrow("(max-width: 1023px)");
const sideOpen = ref(false);
const shareOpen = ref(false);
const unitShareOpen = ref(false);
const instructionOpen = ref(false);

const documentId = computed(() => Number(route.params.id));

interface Held {
  document: ArtifactDocument;
  // The document's kind as the catalogue answers it, null for a word the catalogue does not hold.
  kind: ArtifactKind | null;
  markdown: string;
  attachments: ArtifactAttachment[];
  references: ArtifactReference[];
}

const held = ref<Held | null>(null);
const failed = ref("");
let generation = 0;

onScopeDispose(() => {
  generation++;
});

const kind = computed(() => held.value?.kind ?? null);
const hosted = computed(() => kind.value?.hosted === true);
const kindText = computed(() => {
  const name = held.value?.document.kind ?? "";
  const key = `artifact.kind.${name}`;
  const said = t(key);

  return said === key ? name : said;
});

// The key each locale of the pack names the script its words are written in under (docs/artifact.md, 「Where a
// person reads it」).
const SCRIPT_KEY = "artifact.reading.script";

// The language a tag names, by its primary subtag.
function primary(tag: string): string {
  return tag.split("-")[0]!.toLowerCase();
}

const scripts: Script[] = availableLocales.flatMap((code) => {
  const script = te(SCRIPT_KEY, code) ? scriptOf(primary(code), t(SCRIPT_KEY, {}, { locale: code })) : null;

  return script === null ? [] : [script];
});

// A markdown document read in its own language and, where that is not the reader's, in the reader's through the
// translator the console names.
const reading = documentReading(() => (held.value === null || hosted.value ? null : { title: held.value.document.title, markdown: held.value.markdown }), {
  reader: primary(String(locale.value)),
  scripts,
  translate: hasTranslator() ? translated : null,
  batch: translatorOf()?.batch ?? TRANSLATION_BATCH
});

const contents = computed(() => contentsOf(reading.markdown.value, kind.value?.contentsDepth ?? 2));

// The switch between the document's own language and the reader's, where it is written in another.
const translation = useTranslationToggle(() => reading);

// What the document is, for the properties among the actions on it: who filed it, what it is held under and its
// times, and for a page, which stands with no masthead, its labels too; a written document's labels stand on its
// masthead's secondary line.
const properties = computed<BarProperty[]>(() => {
  const document = held.value?.document;

  if (document === undefined) return [];

  return [
    ...(hosted.value ? Object.entries(labelsBesideInstruction(document.labels)).map(([label, value]) => ({ label, value })) : []),
    { label: t("artifact.label.actor"), value: document.actorName },
    ...(document.scope === null ? [] : [{ label: t("artifact.label.scope"), value: document.scope, mono: true }]),
    { label: t("artifact.label.created"), value: formatStamp(Math.floor(document.createdAt / 1000)) },
    { label: t("artifact.label.changed"), value: formatStamp(Math.floor(document.changedAt / 1000)) }
  ];
});

function fileAddress(attachment: ArtifactAttachment, download = false): string {
  return attachmentAddress(documentId.value, attachment.id, download);
}

function openShare(): void {
  shareOpen.value = true;
}

function openUnitShare(): void {
  unitShareOpen.value = true;
}

// The instruction the document was asked for with, offered behind an action of its own rather than drawn under
// its title.
const instruction = computed(() => (held.value === null ? null : instructionOf(held.value.document)));

function openInstruction(): void {
  instructionOpen.value = true;
}

// Where the document is served when it is a page of its own.
const page: PageServed = (asked) => contentAddress(documentId.value, asked);
const pageActions = usePageActions(() => page);

function openSide(): void {
  sideOpen.value = true;
}

// What a written document offers of its own: its referred pages where they do not stand beside it, and its file.
const writtenActions = useWrittenActions({
  file: () => contentAddress(documentId.value, { download: true }),
  references: () => (narrow.value ? openSide : null)
});

// Each kind of document offers what it holds of its own.
const ownActions = computed(() => (hosted.value ? pageActions.value : writtenActions.value));

// A document read under a permission is one the reader reads alone, through a share or as the administrator reading
// one another account owns, and shares nothing of (`docs/artifact.md`, "Owners").
const shareable = computed(() => held.value !== null && (held.value.document.permission ?? null) === null);
// Only its owner shares a document with units of the owner's company; a document with no owner belongs to none
// (`docs/artifact.md`, "Unit shares").
const unitShareable = computed(() => shareable.value && (held.value?.document.owner ?? null) !== null);

// All the actions on the document the screen's row offers, so none stands anywhere else on it: its instruction where
// it carries one, what its kind offers of its own, and its sharing and its sharing with units where the reader may
// share it; its properties close the group.
const actions = computed<BarAction[]>(() =>
  held.value === null
    ? []
    : [
        ...(instruction.value === null ? [] : [{ id: "instruction", label: t("artifact.action.instruction"), icon: "i-material-symbols-assignment-outline", onSelect: openInstruction }]),
        ...ownActions.value,
        ...(shareable.value ? [{ id: "share", label: t("artifact.action.share"), icon: "i-material-symbols-share-outline", onSelect: openShare }] : []),
        ...(unitShareable.value ? [{ id: "share-units", label: t("artifact.action.share_units"), icon: "i-material-symbols-account-tree-outline", onSelect: openUnitShare }] : [])
      ]
);

const bodyProps = computed(() => ({
  title: reading.title.value,
  kind: kindText.value,
  markdown: reading.markdown.value,
  reading: reading.written.value === null ? null : reading,
  slugs: contents.value.slugs,
  attachments: held.value?.attachments ?? [],
  address: fileAddress,
  labels: held.value === null ? null : labelsBesideInstruction(held.value.document.labels)
}));

const sideProps = computed(() => ({
  contents: contents.value.entries,
  references: held.value?.references ?? [],
  image: (reference: ArtifactReference) => referenceImageAddress(documentId.value, reference.id),
  read: (reference: ArtifactReference) => readReference(documentId.value, reference.id)
}));

// A contents entry names a heading; only the body's own scroller moves, never the frame it stands in.
async function goTo(slug: string): Promise<void> {
  sideOpen.value = false;
  await nextTick();

  const target = document.querySelector(`[data-heading="${CSS.escape(slug)}"]`);

  if (target !== null) scrollWithin(target, "start");
}

const road = computed(() => shareRoad(documentId.value));
// The members a document is shared with go by the console's people directory; where it provides none, the dialog
// shares by link alone.
const people = usePeopleDirectory();
const members = computed(() => (people === null ? undefined : memberRoad(documentId.value, people)));
const explainShare = shareFailure(t);
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

useHead({ title: computed(() => (held.value === null ? t("artifact.title.screen") : held.value.document.title.slice(0, 60))) });

async function load(): Promise<void> {
  const mine = ++generation;
  const id = documentId.value;

  failed.value = "";

  if (!Number.isSafeInteger(id) || id < 1) {
    failed.value = t("artifact.error.bad_id");
    return;
  }

  try {
    const [document, catalogue] = await Promise.all([readDocument(id), readKinds()]);
    // The kind says whether the document is the page it keeps, opened in its frame rather than read as
    // a body.
    const kind = kindOf(catalogue, document.kind);
    const [markdown, attachments, references] = await Promise.all([
      kind?.hosted === true ? Promise.resolve("") : readContent(id),
      readAttachments(id),
      readReferences(id)
    ]);

    if (mine === generation) held.value = { document, kind, markdown, attachments, references };
  } catch (error) {
    if (mine === generation) failed.value = failureText(error, t("artifact.error.read_failed"));
  }
}

watch(documentId, () => {
  held.value = null;
  void load();
});

onMounted(() => void load());
</script>
