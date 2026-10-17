<template>
  <!-- One document of the Archive, with the pages it stands on and its contents beside it. The two
       are separate parts that scroll on their own with a boundary between them; on a phone the side
       rises from the bottom edge over the text. -->
  <UDashboardPanel id="search" :ui="{ body: 'p-0 sm:p-0 gap-0 overflow-hidden [&>*]:max-w-none' }">
    <template #header>
      <UDashboardNavbar :title="$t('archive.title.document', { id: searchId })" icon="i-material-symbols-archive-outline">
        <template #leading>
          <UTooltip :text="$t('archive.action.back_to_list')">
            <UButton to="/archive" size="sm" color="neutral" variant="ghost" icon="i-material-symbols-arrow-back" :aria-label="$t('archive.action.back_to_list')" />
          </UTooltip>
        </template>

        <template #right>
          <DeployMark />
          <TodoOpenButton />
          <UsageMeters />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <div v-if="failed !== ''" role="alert" class="flex flex-wrap items-center gap-2 p-4">
        <p class="text-sm text-error">{{ failed }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('common.action.refresh')" @click="load" />
      </div>

      <ContentSkeleton v-else-if="detail === null" class="p-4" :label="$t('archive.status.document_loading')">
        <SkeletonText :lines="8" size="base" lastWidth="w-2/5" />
      </ContentSkeleton>

      <ArchivePageFrame
        v-else-if="pageUrl !== null"
        :title="bodyProps.title"
        :kind="bodyProps.kind ?? ''"
        :shared="sharedText"
        :page-url="pageUrl"
        :properties="properties"
        :share="() => { shareOpen = true; }"
      />

      <div v-else-if="narrow" class="flex min-h-0 flex-1 flex-col">
        <div class="document-paper min-h-0 flex-1 overflow-y-auto">
          <ArchiveBody v-bind="bodyProps" :sources="() => { sourcesOpen = true; }" :share="shareAction" @mark="showMark" />
        </div>

        <UDrawer v-model:open="sourcesOpen" :title="$t('archive.title.sources_count', { n: countText(pageCount) })" :ui="FULL_SHEET">
          <template #body>
            <div class="min-h-0 flex-1 overflow-y-auto">
              <ArchiveSources v-bind="sourceProps" />
            </div>
          </template>
        </UDrawer>
      </div>

      <USplitter
        v-else
        id="search-splitter"
        auto-save-id="bmo-search-splitter"
        :items="SIDES"
        :ui="{ handle: SPLIT_HANDLE }"
        class="min-h-0 flex-1"
      >
        <template #body>
          <div class="document-paper min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArchiveBody v-bind="bodyProps" :share="shareAction" @mark="showMark" />
          </div>
        </template>

        <template #sources>
          <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
            <ArchiveSources v-bind="sourceProps" />
          </div>
        </template>
      </USplitter>

      <ArchiveShareDialog v-model:open="shareOpen" :search-id="searchId" @shared="markShared" />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { formatStamp, shareLeftText, useNowSeconds } from "bmo-ui/app/domain/time";
import { useI18n } from "vue-i18n";

import { searchTitle } from "../../../shared/config";
import { findingImages, headProperties, roomName, sourceGroups, statusView } from "../../domain/archive";
import { appUrl } from "../../domain/urls";

import type { SplitterItem } from "@nuxt/ui";
import type { BarProperty } from "bmo-ui/app/domain/tabs";
import type { SearchDetail, SearchShareMark } from "../../../shared/types";

// One document of the Archive: its body in the order it was written and the files it keeps, and
// beside it its contents as its kind outlines them, the pages each finding stands on, the pages it
// opened and cited nothing from, and the addresses its text mentions without having opened them. The
// read API answers all of it, the kind and its outline among it.

const { t } = useI18n();
const SIDES: SplitterItem[] = [
  { slot: "body", minSize: 40, defaultSize: 72 },
  { slot: "sources", minSize: 16, defaultSize: 28 }
];

const route = useRoute();
const narrow = useNarrow("(max-width: 1023px)");
const sourcesOpen = ref(false);
const shareOpen = ref(false);

const searchId = computed(() => Number(route.params.id));

const detail = ref<SearchDetail | null>(null);
const failed = ref("");
let generation = 0;
onScopeDispose(() => { generation++; });

const status = computed(() => statusView(t, detail.value?.search.status ?? ""));
// What kind of document this is, which says what its head shows and whether it is a page of its own.
const kind = computed(() => detail.value?.kind ?? null);

// Whether this document is out on a link, in the words the chip beside the state reads. The dialog
// is what says so: it is the one place a link is made and taken away, so the chip follows it rather
// than the screen asking the server again once the dialog has closed.
const { now, sync } = useNowSeconds();
const sharedText = computed(() => {
  const mark = detail.value?.search.shared ?? null;

  return mark === null ? null : t("archive.label.sharing", { left: shareLeftText(t, mark.expiresAt, now.value) });
});

function markShared(mark: SearchShareMark | null): void {
  sync();

  if (detail.value !== null) detail.value.search.shared = mark;
}

const citedGroups = computed(() => sourceGroups(t, detail.value?.findings ?? []));

// Each finding's pictures, answered with the console's copies. Built once per reading, so a text is
// not rendered again for nothing more than a new function.
const base = appUrl("");
// Where this document's refs and attachments are answered: the console's own road, named by the
// document. A share page names its token here instead, and the parts below cannot tell the two apart.
const refBase = computed(() => `/api/archive/${searchId.value}/refs`);
const attachmentBase = computed(() => `${base}/api/archive/${searchId.value}/attachments`);
const pictures = computed(
  () => new Map((detail.value?.findings ?? []).map((finding) => [finding.place, findingImages(refBase.value, finding, (path) => `${base}${path}`)]))
);

// The document's contents as its kind outlines them, and the marks its headings are stamped with in
// the text. The read API answers both together, so an entry and the heading it goes to can never be
// steering by different marks.
const contents = computed(() => detail.value?.outline.entries ?? []);
const marks = computed(() => new Map(Object.entries(detail.value?.outline.marks ?? {}).map(([place, slugs]) => [Number(place), slugs])));

// A contents entry names a section inside a finding, or the finding itself where it has no heading of
// its own. Only the body's own scroller moves, never the frame it stands in.
async function goTo(slug: string): Promise<void> {
  sourcesOpen.value = false;
  await nextTick();

  const target = document.querySelector(`[data-heading="${CSS.escape(slug)}"], [data-finding="${CSS.escape(slug)}"]`);

  if (target !== null) scrollWithin(target, "start");
}

// What the document is, for the properties viewer beside the title: what its kind's head shows, and
// the state, the room and the times every document carries. The kind itself stands as a chip under
// the title.
const properties = computed<BarProperty[]>(() => {
  const search = detail.value?.search;

  if (search === undefined) return [];

  return [
    ...headProperties(t, kind.value?.head ?? [], search),
    { label: t("archive.label.state"), value: status.value.label },
    { label: t("common.label.room"), value: roomName(search) },
    { label: t("archive.label.start"), value: formatStamp(search.startedAt) },
    ...(search.finishedAt === null ? [] : [{ label: t("archive.label.end"), value: formatStamp(search.finishedAt) }])
  ];
});

const bodyProps = computed(() => {
  const held = detail.value;
  // A search that answered one value keeps no document: it is in no list and there is nothing to hand
  // out on a link, so the screen says that rather than showing it as a search that found nothing.
  const documented = held?.search.documented !== false;

  return {
    title: held === null ? "" : searchTitle(held.search),
    kind: kind.value === null ? null : t(kind.value.labelKey),
    hosted: kind.value?.hosted ?? false,
    findings: held?.findings ?? [],
    runCount: held?.search.runs.length ?? 0,
    attachments: held?.attachments ?? [],
    attachmentBase: attachmentBase.value,
    status: status.value,
    shared: sharedText.value,
    properties: properties.value,
    pictures: pictures.value,
    marks: marks.value,
    empty: documented ? t("archive.status.empty") : t("archive.status.value_only"),
    documentUrl: held?.search.document === true ? `${base}/api/archive/${searchId.value}/document` : null
  };
});

// Where the page a page document is stands, and null for every other document, which is read as a
// body. The kind says which it is.
const pageUrl = computed(() => {
  const page = kind.value?.hosted === true ? detail.value?.attachments.find((attachment) => attachment.media === "page") : undefined;

  return page === undefined ? null : `${attachmentBase.value}/${page.id}`;
});

// The share action, and none at all for a search that kept no document to hand out.
const shareAction = computed(() => (detail.value?.search.documented === false ? null : () => { shareOpen.value = true; }));

const sourceProps = computed(() => ({
  refBase: refBase.value,
  contents: contents.value,
  citedGroups: citedGroups.value,
  opened: detail.value?.opened ?? [],
  bodyLinks: bodyLinks.value,
  pageCount: pageCount.value,
  activeMark: activeMark.value,
  onGo: goTo
}));

// The source a footnote mark in the text names, brought forward in the source list — which a phone
// opens first — and marked there until another is pressed.
const activeMark = ref<string | null>(null);

async function showMark(mark: string): Promise<void> {
  activeMark.value = mark;
  if (narrow.value) sourcesOpen.value = true;
  await nextTick();
  // The drawer is still rising on its first frame, so the list is looked for once it stands.
  requestAnimationFrame(() => {
    const source = document.querySelector(`[data-source-mark="${CSS.escape(mark)}"]`);

    if (source !== null) scrollWithin(source, "center", "smooth");
  });
}

const bodyLinks = computed(() => (detail.value?.findings ?? []).flatMap((finding) => finding.links));

const pageCount = computed(
  () =>
    citedGroups.value.reduce((sum, group) => sum + group.sources.length, 0) +
    (detail.value?.opened.length ?? 0) +
    bodyLinks.value.length
);

useHead({ title: computed(() => (detail.value === null ? t("screens.label.archive") : searchTitle(detail.value.search).slice(0, 60))) });

async function load(): Promise<void> {
  const mine = ++generation;
  failed.value = "";

  if (!Number.isSafeInteger(searchId.value) || searchId.value < 1) {
    failed.value = t("archive.error.bad_id");
    return;
  }

  try {
    const answer = await $fetch<SearchDetail>(`/api/archive/${searchId.value}`);

    if (mine === generation) detail.value = answer;
  } catch (error) {
    if (mine === generation) failed.value = failureText(error, t("archive.error.read_failed"));
  }
}

watch(searchId, () => {
  detail.value = null;
  void load();
});

onMounted(() => void load());
</script>
