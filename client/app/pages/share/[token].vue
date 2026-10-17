<template>
  <!-- One document of the Archive, read by somebody who is not the owner. It is the body, the files it
       keeps and the pages it stands on, and nothing else: no sidebar, no title bar, and no control
       that leads anywhere in this console. On a phone the side rises from the bottom edge over the
       text, as it does on the console's own screen. A document that is a page of its own is that
       page filling the window, and nothing around it. -->
  <ScreenFrame class="document-paper">
    <div v-if="failed !== ''" role="alert" class="flex min-h-0 flex-1 items-center justify-center p-4">
      <p class="text-sm text-error">{{ failed }}</p>
    </div>

    <ContentSkeleton v-else-if="detail === null" class="p-4" :label="$t('archive.status.document_loading')">
      <SkeletonText :lines="8" size="base" lastWidth="w-2/5" />
    </ContentSkeleton>

    <HostedPageFrame v-else-if="pageUrl !== null" :src="pageUrl" :title="detail.title" />

    <div v-else-if="narrow" class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 overflow-y-auto">
        <ArchiveBody v-bind="bodyProps" :sources="() => { sourcesOpen = true; }" @mark="showMark" />
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
      id="share-splitter"
      auto-save-id="bmo-share-splitter"
      :items="SIDES"
      :ui="{ handle: SPLIT_HANDLE }"
      class="min-h-0 flex-1"
    >
      <template #body>
        <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
          <ArchiveBody v-bind="bodyProps" @mark="showMark" />
        </div>
      </template>

      <template #sources>
        <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
          <ArchiveSources v-bind="sourceProps" />
        </div>
      </template>
    </USplitter>
  </ScreenFrame>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../../shared/document-design";
import { SHARE_ROBOTS, shareHeadTags } from "../../../shared/share-head";
import { attachmentUrl, findingImages, schemedPage, sourceGroups } from "../../domain/archive";
import { appRoot, appUrl } from "../../domain/urls";

import type { SplitterItem } from "@nuxt/ui";
import type { SharedDocumentHead, SharedSearchDetail } from "../../../shared/types";

// What a share link opens. The page knows its document by the token it arrived holding and by
// nothing else: it has no document number, asks for none, and every road it takes is under that one
// token. So there is no other document to reach from here, and no screen of the console either —
// none of the console's own frame is on this page to reach one by.
//
// The three the console's screen carries and this one does not — the state, the properties, the
// document's own file — are absent from the answer rather than from the template, so the parts
// below simply have none of them to offer.

definePageMeta({ layout: false });

const { t } = useI18n();
const SIDES: SplitterItem[] = [
  { slot: "body", minSize: 40, defaultSize: 72 },
  { slot: "sources", minSize: 16, defaultSize: 28 }
];

const route = useRoute();
const narrow = useNarrow("(max-width: 1023px)");
const sourcesOpen = ref(false);

const token = computed(() => String(route.params.token ?? ""));

const detail = ref<SharedSearchDetail | null>(null);
const failed = ref("");
let generation = 0;
onScopeDispose(() => { generation++; });

const base = appUrl("");
// Where this document's refs are answered: the link's own road. The document's number is nowhere in
// it, so nothing this page renders can be pointed at another document.
const refBase = computed(() => `/api/share/${encodeURIComponent(token.value)}/refs`);
const attachmentBase = computed(() => `${base}/api/share/${encodeURIComponent(token.value)}/attachments`);
// What kind of document the link opens, as the share API answers it.
const kind = computed(() => detail.value?.kindEntry ?? null);
const colorMode = useColorMode();

// A page document's page under the link's own road, in the reader's scheme, carrying on to the place
// in the page the link names. The document is read once the page is up, so the address is the
// browser's.
const pageUrl = computed(() => {
  const page = kind.value?.hosted === true ? detail.value?.attachments.find((attachment) => attachment.media === "page") : undefined;

  return page === undefined ? null : `${schemedPage(attachmentUrl(attachmentBase.value, page), colorMode.value)}${window.location.hash}`;
});

const pictures = computed(
  () => new Map((detail.value?.findings ?? []).map((finding) => [finding.place, findingImages(refBase.value, finding, (path) => `${base}${path}`)]))
);

const citedGroups = computed(() => sourceGroups(t, detail.value?.findings ?? []));

// The contents as the document's kind outlines them, and the marks its headings are stamped with in
// the text, as the share API answers both, the same as on the console's own screen.
const contents = computed(() => detail.value?.outline.entries ?? []);
const marks = computed(() => new Map(Object.entries(detail.value?.outline.marks ?? {}).map(([place, slugs]) => [Number(place), slugs])));

const bodyLinks = computed(() => (detail.value?.findings ?? []).flatMap((finding) => finding.links));

const pageCount = computed(
  () =>
    citedGroups.value.reduce((sum, group) => sum + group.sources.length, 0) +
    (detail.value?.opened.length ?? 0) +
    bodyLinks.value.length
);

// No status, no properties and no document file are handed over, so the parts that would show them
// stand nowhere at all.
const bodyProps = computed(() => ({
  title: detail.value?.title ?? "",
  kind: kind.value === null ? null : t(kind.value.labelKey),
  hosted: kind.value?.hosted ?? false,
  findings: detail.value?.findings ?? [],
  runCount: detail.value?.runCount ?? 0,
  attachments: detail.value?.attachments ?? [],
  attachmentBase: attachmentBase.value,
  pictures: pictures.value,
  marks: marks.value
}));

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

// A contents entry names a section inside a finding, or the finding itself where it has no heading of
// its own. Only the body's own scroller moves, never the frame it stands in.
async function goTo(slug: string): Promise<void> {
  sourcesOpen.value = false;
  await nextTick();

  const target = document.querySelector(`[data-heading="${CSS.escape(slug)}"], [data-finding="${CSS.escape(slug)}"]`);

  if (target !== null) scrollWithin(target, "start");
}

const activeMark = ref<string | null>(null);

async function showMark(mark: string): Promise<void> {
  activeMark.value = mark;
  if (narrow.value) sourcesOpen.value = true;
  await nextTick();
  requestAnimationFrame(() => {
    const source = document.querySelector(`[data-source-mark="${CSS.escape(mark)}"]`);

    if (source !== null) scrollWithin(source, "center", "smooth");
  });
}

// What the link reads as before it is opened is read while the page is rendered on the server, so the
// page goes out with the document's own head and a chat app drawing a preview of the link reads it
// without running anything (`docs/archive.md`, "The share page"). The document itself is still read
// once the page is up; a link that opens nothing has no head of its own and says so there.
const { data: shared } = await useFetch<SharedDocumentHead | null>(() => `/api/share/${encodeURIComponent(token.value)}/head`, {
  key: `share-head:${token.value}`,
  default: () => null
});

const root = appRoot(useRequestURL({ xForwardedHost: true }).origin);
const tags = computed(() => (shared.value === null ? null : shareHeadTags(shared.value, root, `${root}share/${encodeURIComponent(token.value)}`)));

// The page stands on the document paper from its first frame, the skeleton included.
useHead({
  title: computed(() => (tags.value === null ? t("screens.label.archive") : tags.value.title.slice(0, 60))),
  meta: computed(() => tags.value?.meta ?? [SHARE_ROBOTS]),
  link: computed(() => (tags.value === null ? [] : [{ rel: "canonical", href: tags.value.canonical }])),
  style: [{ key: "document-palette", innerHTML: DOCUMENT_PALETTE }]
});

async function load(): Promise<void> {
  const mine = ++generation;
  failed.value = "";

  if (token.value === "") {
    failed.value = t("common.error.share_not_found");
    return;
  }

  try {
    const answer = await $fetch<SharedSearchDetail>(`/api/share/${encodeURIComponent(token.value)}`);

    if (mine === generation) detail.value = answer;
  } catch (error) {
    if (mine === generation) failed.value = failureText(error, t("archive.error.open_failed"));
  }
}

onMounted(() => void load());
</script>
