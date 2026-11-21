<!-- The front page of the documents (docs/artifact.md, 「The front page」): the newest at its head and a section for each
     kind under it, and beside them from the top a side column of the kinds as a navigation list and the documents most
     read. It is laid out by the width it is given, never the window's: two columns where the room holds them, and one
     where it does not, with the kinds as a row of chips first and the most read after the newest. Each part reads on
     its own and keeps a skeleton of its own shape until it has read. -->
<template>
  <div class="@container/front min-w-0" data-front>
    <div class="grid min-w-0 gap-6 @4xl/front:grid-cols-[minmax(0,1fr)_16rem] @4xl/front:grid-rows-[auto_1fr]">
      <!-- Where there is no room for the side column, the categories stand first, as a row of chips under the bar. -->
      <nav class="flex min-w-0 flex-wrap gap-2 @4xl/front:hidden" :aria-label="$t('artifact.front.kinds')" data-kind-chips>
        <UButton v-for="item in kindItems" :key="item.label" :to="item.to" :icon="item.icon" :label="item.label" size="sm" color="neutral" variant="outline" />
      </nav>

      <section class="@container/newest flex min-w-0 flex-col gap-3" :aria-labelledby="`${id}-newest`" data-newest>
        <header class="flex h-9 min-w-0 items-center gap-2">
          <h2 :id="`${id}-newest`" class="min-w-0 truncate text-xl font-semibold text-highlighted">{{ $t("artifact.front.newest") }}</h2>
          <UButton to="/artifact?view=list" color="neutral" variant="ghost" size="sm" trailing-icon="i-material-symbols-chevron-right" :label="$t('common.action.see_all')" class="shrink-0" data-see-all />
        </header>

        <p v-if="failed.newest" role="alert" class="text-sm text-error">{{ $t("artifact.error.list_failed") }}</p>

        <SkeletonStatus v-if="newest === null && !failed.newest" as="div" :label="$t('artifact.status.document_loading')" :class="NEWEST">
          <ArtifactCardSkeleton lead />
          <div class="@container/cards min-w-0">
            <div :class="BESIDE_LEAD">
              <ArtifactCardSkeleton v-for="n in NEWEST_BESIDE" :key="n" />
            </div>
          </div>
        </SkeletonStatus>
        <p v-else-if="newest !== null && newest.length === 0" class="text-sm text-muted">{{ $t("artifact.status.no_documents") }}</p>
        <div v-else-if="newest !== null" :class="NEWEST">
          <ArtifactCard :document="newest[0]!" :kind="kindOf(catalogue, newest[0]!.kind)" :kinds="catalogue" :public="isPublic(newest[0]!.id)" lead with-kind />
          <div v-if="newest.length > 1" class="@container/cards min-w-0">
            <div :class="BESIDE_LEAD">
              <ArtifactCard
                v-for="document in newest.slice(1)"
                :key="document.id"
                :document="document"
                :kind="kindOf(catalogue, document.kind)"
                :kinds="catalogue"
                :public="isPublic(document.id)"
                with-kind
              />
            </div>
          </div>
        </div>
      </section>

      <!-- The side column, from the top of the page beside the newest down: the categories first, then the most read, each
           a titled list with no frame. Where there is no room for it, the most read stands alone after the newest. -->
      <aside class="flex min-w-0 flex-col gap-6 @4xl/front:col-start-2 @4xl/front:row-span-2 @4xl/front:row-start-1" data-side>
        <nav class="hidden min-w-0 flex-col gap-2 @4xl/front:flex" :aria-labelledby="`${id}-kinds`" data-kinds>
          <header class="flex h-9 min-w-0 items-center border-b border-default">
            <h2 :id="`${id}-kinds`" class="min-w-0 truncate text-xl font-semibold text-highlighted">{{ $t("artifact.front.kinds") }}</h2>
          </header>
          <UNavigationMenu orientation="vertical" :items="kindItems" class="w-full" />
        </nav>

        <section class="flex min-w-0 flex-col gap-2" :aria-labelledby="`${id}-most-read`" data-most-read-part>
          <header class="flex h-9 min-w-0 items-center gap-2 border-b border-default">
            <h2 :id="`${id}-most-read`" class="min-w-0 truncate text-xl font-semibold text-highlighted">{{ $t("artifact.front.most_read") }}</h2>
            <span class="shrink-0 text-sm text-muted">{{ $t("artifact.front.most_read_span") }}</span>
          </header>
          <p v-if="failed.mostRead" role="alert" class="text-sm text-error">{{ $t("artifact.error.list_failed") }}</p>
          <SkeletonStatus v-if="mostRead === null && !failed.mostRead" as="div" :label="$t('artifact.status.document_loading')" class="flex flex-col">
            <div v-for="n in MOST_READ_UNREAD" :key="n" class="flex items-center gap-3 px-2 py-2">
              <SkeletonBar class="h-4 w-5 shrink-0" />
              <SkeletonLine width="w-3/4" class="flex-1" />
            </div>
          </SkeletonStatus>
          <p v-else-if="mostRead !== null && mostRead.length === 0" class="text-sm text-muted" data-no-reads>{{ $t("artifact.front.no_reads") }}</p>
          <ArtifactMostRead v-else-if="mostRead !== null" :items="mostRead" />
        </section>
      </aside>

      <div class="flex min-w-0 flex-col gap-6 @4xl/front:col-start-1 @4xl/front:row-start-2" data-sections>
        <p v-if="failed.sections" role="alert" class="text-sm text-error">{{ $t("artifact.error.list_failed") }}</p>

        <SkeletonStatus v-if="strips === null && !failed.sections" as="div" :label="$t('artifact.status.document_loading')" class="flex flex-col gap-6">
          <div v-for="n in SECTIONS_UNREAD" :key="n" class="flex min-w-0 flex-col gap-3">
            <div class="flex h-9 items-center"><SkeletonBar class="h-6 w-28" /></div>
            <div class="@container/cards min-w-0">
              <div :class="STRIP">
                <ArtifactCardSkeleton v-for="(_, index) in STRIP_LENGTH" :key="index" :class="stripPlace(index)" />
              </div>
            </div>
          </div>
        </SkeletonStatus>

        <section v-for="section in sections" :key="section.kind.name" class="flex min-w-0 flex-col gap-3" :aria-labelledby="`${id}-${section.kind.name}`" :data-section="section.kind.name">
          <header class="flex h-9 min-w-0 items-center gap-2">
            <UIcon :name="kindIcon(section.kind)" class="size-5 shrink-0 text-muted" />
            <h2 :id="`${id}-${section.kind.name}`" class="min-w-0 truncate text-xl font-semibold text-highlighted">{{ kindLabel(section.kind.name) }}</h2>
            <UButton
              :to="{ path: '/artifact', query: { kind: section.kind.name } }"
              color="neutral"
              variant="ghost"
              size="sm"
              trailing-icon="i-material-symbols-chevron-right"
              :label="$t('common.action.see_all')"
              class="shrink-0"
              data-see-all
            />
          </header>
          <div class="@container/cards min-w-0">
            <div :class="STRIP">
              <ArtifactCard
                v-for="(document, index) in section.documents"
                :key="document.id"
                :document="document"
                :kind="section.kind"
                :kinds="catalogue"
                :public="isPublic(document.id)"
                :class="stripPlace(index)"
              />
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { readDocuments, readKinds, readLiveShares, readMostRead } from "../domain/artifact";
import { kindOf } from "../domain/document-view";
import { kindIcon, useDocumentWords } from "../domain/document-words";
import { lastFront } from "../domain/front-page";

import type { NavigationMenuItem } from "@nuxt/ui";
import type { ArtifactDocument, ArtifactDocumentReads, ArtifactKind, ArtifactShare } from "../../shared/types";

// How many of the newest stand beside the lead card.
const NEWEST_BESIDE = 4;
// How many documents a kind's strip asks for: the most one row of it holds.
const STRIP_LENGTH = 4;
// How many of the most read stand.
const MOST_READ_LENGTH = 10;
// The days a lead card's picture is looked for in.
const LEAD_DAYS = 7;
// How many sections and most read lines a skeleton keeps the place of, before the catalogue and the reads say.
const SECTIONS_UNREAD = 2;
const MOST_READ_UNREAD = 5;
// A strip of cards: one column of rows on a phone, then two, three and four to a row as the width holds them, four
// beside the side column of a wide window.
const STRIP = "grid grid-cols-1 gap-4 @md/cards:grid-cols-2 @lg/cards:grid-cols-3 @xl/cards:grid-cols-4 @max-md/cards:gap-2";
// The newest: the lead card over the rest where the column is narrow, and beside them where it holds both.
const NEWEST = "grid gap-4 @xl/newest:grid-cols-[3fr_2fr]";
// The newest beside the lead card: rows one under another, and two by two where their column is wide.
const BESIDE_LEAD = "grid grid-cols-1 gap-4 @md/cards:grid-cols-2 @max-md/cards:gap-2";

const { t } = useI18n();
const id = useId();

// Every part starts from the tab's last reading of it, so a return draws at once and only a first visit waits.
// The catalogue as read, empty until it is.
const kinds = ref<ArtifactKind[]>(lastFront.kinds);
const catalogue = computed<readonly ArtifactKind[]>(() => kinds.value);
const { kindLabel } = useDocumentWords(kinds);

const newest = ref<ArtifactDocument[] | null>(lastFront.newest);
// Each kind's newest documents, by the kind's name; null until every kind's are read.
const strips = ref<Map<string, ArtifactDocument[]> | null>(lastFront.strips);
const mostRead = ref<ArtifactDocumentReads[] | null>(lastFront.mostRead);
// The live links by the document each opens; until they are read no card says it is public.
const shares = ref<Map<number, ArtifactShare> | null>(lastFront.shares);
const failed = reactive({ newest: false, sections: false, mostRead: false });
const loading = ref(false);
let generation = 0;

onScopeDispose(() => {
  generation++;
});

function isPublic(documentId: number): boolean {
  return shares.value?.has(documentId) ?? false;
}

// A strip stands as one row: its third card goes where two stand to a row and its fourth wherever fewer than four
// do, but for a phone's column of rows, which keeps three.
function stripPlace(index: number): string {
  if (index === 2) return "@md/cards:@max-lg/cards:hidden";
  if (index === 3) return "@max-xl/cards:hidden";

  return "";
}

// The sections that stand: each kind of the catalogue, in its order, that the reader has a document of.
const sections = computed(() =>
  strips.value === null
    ? []
    : kinds.value.flatMap((kind) => {
        const documents = strips.value?.get(kind.name) ?? [];

        return documents.length === 0 ? [] : [{ kind, documents }];
      })
);

const kindItems = computed<NavigationMenuItem[]>(() => [
  { label: t("artifact.front.all_kinds"), icon: "i-material-symbols-apps", to: "/artifact?view=list", active: false },
  ...kinds.value.map((kind) => ({
    label: kindLabel(kind.name),
    icon: kindIcon(kind),
    to: { path: "/artifact", query: { kind: kind.name } },
    active: false
  }))
]);

// The newest: the newest document of the last days that keeps a cover picture leading, the newest of all where none
// does, and the next newest beside it.
async function readNewest(): Promise<ArtifactDocument[]> {
  const [pictured, latest] = await Promise.all([readDocuments({ picture: true, limit: 1 }), readDocuments({ limit: NEWEST_BESIDE + 2 })]);
  const since = Date.now() - LEAD_DAYS * 24 * 60 * 60 * 1000;
  const lead = pictured.items.find((document) => document.createdAt >= since) ?? latest.items[0];

  return lead === undefined ? [] : [lead, ...latest.items.filter((document) => document.id !== lead.id)].slice(0, NEWEST_BESIDE + 1);
}

async function readStrips(): Promise<{ catalogue: ArtifactKind[]; strips: Map<string, ArtifactDocument[]> }> {
  const catalogue = await readKinds();
  const pages = await Promise.all(catalogue.map(async (kind) => [kind.name, (await readDocuments({ kind: kind.name, limit: STRIP_LENGTH })).items] as const));

  return { catalogue, strips: new Map(pages) };
}

/** Every part read again, each standing as it was until its own read lands. */
async function read(): Promise<void> {
  const mine = ++generation;
  const current = () => mine === generation;

  loading.value = true;

  await Promise.allSettled([
    readNewest().then(
      (read) => {
        if (current()) [newest.value, lastFront.newest, failed.newest] = [read, read, false];
      },
      () => {
        if (current()) failed.newest = true;
      }
    ),
    readStrips().then(
      (read) => {
        if (current()) {
          [kinds.value, strips.value, failed.sections] = [read.catalogue, read.strips, false];
          [lastFront.kinds, lastFront.strips] = [read.catalogue, read.strips];
        }
      },
      () => {
        if (current()) failed.sections = true;
      }
    ),
    readMostRead(MOST_READ_LENGTH).then(
      (read) => {
        if (current()) [mostRead.value, lastFront.mostRead, failed.mostRead] = [read, read, false];
      },
      () => {
        if (current()) failed.mostRead = true;
      }
    ),
    readLiveShares().then(
      (links) => {
        if (current()) [shares.value, lastFront.shares] = [links, links];
      },
      () => undefined
    )
  ]);

  if (current()) loading.value = false;
}

defineExpose({ read, loading });

onMounted(() => void read());
</script>
