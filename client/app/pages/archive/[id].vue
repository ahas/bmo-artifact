<template>
  <!-- One document, with its contents and the pages it refers to beside it. The two are separate
       parts that scroll on their own with a boundary between them; on a phone the side rises from the
       bottom edge over the text. A page fills the screen in its frame. -->
  <UDashboardPanel id="artifact-document" :ui="{ body: 'p-0 sm:p-0 gap-0 overflow-hidden [&>*]:max-w-none' }">
    <!-- Under a console that offers its title bar the console draws it, and the way back to the list, the
         document's number and its being in progress stand beside its name; alone, the screen draws its own bar
         holding them. -->
    <template v-if="barOffer === null" #header>
      <UDashboardNavbar :title="$t('artifact.title.document', { id: documentId })" icon="i-material-symbols-archive-outline">
        <template #leading>
          <UTooltip :text="$t('artifact.action.back_to_list')">
            <UButton to="/archive" size="sm" color="neutral" variant="ghost" icon="i-material-symbols-arrow-back" :aria-label="$t('artifact.action.back_to_list')" />
          </UTooltip>
        </template>

        <template v-if="held !== null" #trailing>
          <ArtifactPendingBadge :document="held.document" />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <!-- The way back to the list and the document's number, beside the console bar's name. -->
      <Teleport v-if="barOffer !== null" defer :to="barOffer.screenActions">
        <UTooltip :text="$t('artifact.action.back_to_list')">
          <UButton to="/archive" size="sm" color="neutral" variant="ghost" icon="i-material-symbols-arrow-back" :aria-label="$t('artifact.action.back_to_list')" />
        </UTooltip>
        <span class="truncate text-sm text-muted">{{ $t("artifact.title.document", { id: documentId }) }}</span>
        <ArtifactPendingBadge v-if="held !== null" :document="held.document" />
      </Teleport>
      <div v-if="failed !== ''" role="alert" class="flex flex-wrap items-center gap-2 p-4">
        <p class="text-sm text-error">{{ failed }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('artifact.action.refresh')" @click="load" />
      </div>

      <ContentSkeleton v-else-if="held === null" class="p-4" :label="$t('artifact.status.document_loading')">
        <SkeletonText :lines="8" size="base" lastWidth="w-2/5" />
      </ContentSkeleton>

      <ArtifactPageFrame
        v-else-if="hosted"
        :title="held.document.title"
        :kind="kindText"
        :page="(asked) => contentAddress(documentId, asked)"
        :properties="properties"
        :share="openShare"
      />

      <div v-else-if="narrow" class="flex min-h-0 flex-1 flex-col">
        <div class="document-paper min-h-0 flex-1 overflow-y-auto">
          <ArtifactBody v-bind="bodyProps" :references="openSide" />
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
          <div class="document-paper min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArtifactBody v-bind="bodyProps" />
          </div>
        </template>

        <template #side>
          <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArtifactReferences v-bind="sideProps" @go="goTo" />
          </div>
        </template>
      </USplitter>

      <ShareLinkDialog v-model:open="shareOpen" :road="road" :spans="spans" :subject="held?.document.title" />
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
import { contentsOf, kindOf } from "../../domain/document-view";

import type { SplitterItem } from "@nuxt/ui";
import type { BarProperty } from "bmo-ui/app/domain/tabs";
import type { ArtifactAttachment, ArtifactDocument, ArtifactKind, ArtifactReference } from "../../../shared/types";

// One document: its head, its file drawn as its kind reads it, the files it keeps and the pages it
// refers to, each read from the service through the console's relay.

const { t } = useI18n();
// The console's title bar, where a console offers it under the key bmo-ui's bundle contract names: the place in it
// the screen lays what acts on it as a whole.
const barOffer = inject<{ screenActions: string } | null>("bmo-console-title-bar", null);
const SIDES: SplitterItem[] = [
  { slot: "body", minSize: 40, defaultSize: 72 },
  { slot: "side", minSize: 16, defaultSize: 28 }
];

const route = useRoute();
const narrow = useNarrow("(max-width: 1023px)");
const sideOpen = ref(false);
const shareOpen = ref(false);

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

const contents = computed(() => contentsOf(held.value?.markdown ?? "", kind.value?.contentsDepth ?? 2));

// What the document is, for the properties viewer beside the title: who filed it, what it is held under and its
// times, and for a page, which stands with no masthead, its labels too; a written document's labels stand on its
// masthead's secondary line.
const properties = computed<BarProperty[]>(() => {
  const document = held.value?.document;

  if (document === undefined) return [];

  return [
    ...(hosted.value ? Object.entries(document.labels).map(([label, value]) => ({ label, value })) : []),
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

function openSide(): void {
  sideOpen.value = true;
}

const bodyProps = computed(() => ({
  title: held.value?.document.title ?? "",
  kind: kindText.value,
  markdown: held.value?.markdown ?? "",
  slugs: contents.value.slugs,
  attachments: held.value?.attachments ?? [],
  address: fileAddress,
  properties: properties.value,
  labels: held.value?.document.labels ?? null,
  documentUrl: contentAddress(documentId.value, { download: true }),
  share: openShare
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
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(`artifact.label.span_${span.value}`) })));

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
