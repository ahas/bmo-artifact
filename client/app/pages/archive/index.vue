<template>
  <UDashboardPanel id="searches">
    <template #header>
      <UDashboardNavbar :title="$t('screens.label.archive')" icon="i-material-symbols-archive-outline">
        <template #right>
          <DeployMark />
          <TodoOpenButton />
          <UsageMeters />
        </template>
      </UDashboardNavbar>

      <!-- The search takes what width the row has, and the filters and the reread keep theirs. -->
      <UDashboardToolbar :ui="{ left: 'min-w-0 flex-1', right: 'shrink-0' }">
        <template #left>
          <UInput
            v-model="typed"
            size="sm"
            class="min-w-0 flex-1"
            icon="i-material-symbols-search"
            :placeholder="$t('archive.placeholder.search')"
            :aria-label="$t('archive.aria.search')"
            :maxlength="SEARCHES_PHRASE_LIMIT"
            :loading="filtering === 'phrase'"
            :ui="{ trailing: 'pe-1' }"
            @update:model-value="settleSearch"
          >
            <template v-if="typed !== ''" #trailing>
              <UTooltip :text="$t('common.action.clear_search')">
                <UButton
                  icon="i-material-symbols-close"
                  color="neutral"
                  variant="ghost"
                  size="sm"
                  :aria-label="$t('common.action.clear_search')"
                  @click="clearSearch"
                />
              </UTooltip>
            </template>
          </UInput>
        </template>

        <template #right>
          <span v-if="loading && loaded" role="status" class="sr-only">{{ $t("archive.status.refreshing") }}</span>

          <USelect
            :model-value="kindChoice"
            :items="kindChoices"
            size="sm"
            color="neutral"
            variant="ghost"
            :aria-label="$t('archive.aria.kind')"
            :loading="filtering === 'kind'"
            class="w-auto"
            @update:model-value="holdToKind"
          />

          <UTooltip :text="sharedLabel">
            <UButton
              size="sm"
              :color="sharedOnly ? 'primary' : 'neutral'"
              :variant="sharedOnly ? 'solid' : 'ghost'"
              square
              :icon="sharedOnly ? 'i-material-symbols-filter-alt' : 'i-material-symbols-filter-alt-outline'"
              :loading="filtering === 'shared'"
              :aria-label="sharedLabel"
              :aria-pressed="sharedOnly"
              @click="holdToShared(!sharedOnly)"
            />
          </UTooltip>

          <UTooltip :text="$t('common.action.refresh')">
            <UButton
              size="sm"
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-refresh"
              :loading="loading && filtering === null"
              :aria-label="$t('common.action.refresh')"
              @click="reread"
            />
          </UTooltip>
        </template>
      </UDashboardToolbar>
    </template>

    <template #body>
      <div ref="body" class="flex flex-col gap-3">
        <div v-if="failed !== ''" role="alert" class="flex flex-wrap items-center gap-2">
          <p class="text-sm text-error">{{ failed }}</p>
          <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('common.action.refresh')" :loading="loading" @click="reread" />
        </div>

        <ContentSkeleton v-else-if="!loaded" :rows="4" listClass="flex flex-col gap-2" :label="$t('archive.status.document_loading')">
          <ArchiveRowSkeleton />
        </ContentSkeleton>
        <p v-else-if="items.length === 0 && !loading" class="text-sm text-muted">{{ emptyText }}</p>

        <ul :aria-busy="loading" class="flex flex-col gap-2">
          <li v-for="item in items" :key="item.id">
            <NuxtLink
              :to="`/archive/${item.id}`"
              class="flex gap-3 rounded-lg border border-default px-4 py-3 transition-colors hover:bg-elevated/50 active:bg-accented/50"
            >
              <UIcon :name="statusView($t, item.status).icon" class="mt-0.5 size-5 shrink-0" :class="STATUS_TEXT[statusView($t, item.status).color]" />

              <span class="flex min-w-0 grow flex-col gap-2">
              <SearchText class="line-clamp-2 text-sm font-medium break-words text-highlighted" :text="searchTitle(item)" :query="searched" />

              <!-- What was asked is the asker's own sentence, and the document itself opens on a
                   press with the whole of it, so the row shows what it has room for. -->
              <p v-if="subjectText(item) !== ''" class="truncate text-sm text-muted"><SearchText :text="subjectLine(item, searched)" :query="searched" /></p>

              <SearchHit :match="rowMatch(item)" :query="searched" />

              <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
                <UBadge
                  :color="statusView($t, item.status).color"
                  variant="subtle"
                  size="sm"
                  :label="statusView($t, item.status).label"
                />
                <UBadge v-if="kindOf(item) !== null" color="neutral" variant="outline" size="sm" :label="$t(kindOf(item)!.labelKey)" />
                <UBadge v-if="kindOf(item)?.head.includes('depth')" color="neutral" variant="outline" size="sm" :label="depthText($t, item.startedDepth, item.endedDepth)" />
                <UBadge
                  v-if="item.shared !== null"
                  color="info"
                  variant="subtle"
                  size="sm"
                  icon="i-material-symbols-share-outline"
                  class="tabular-nums"
                  :label="$t('archive.label.sharing', { left: shareLeftText($t, item.shared.expiresAt, now) })"
                />
                <SearchText class="text-sm text-muted" :text="roomName(item)" :query="searched" />
                <span v-if="item.sources > 0" class="text-sm text-muted tabular-nums">{{ $t("archive.label.findings_pages", { findings: countText(item.findings), pages: countText(item.sources) }) }}</span>
                <span class="ms-auto text-sm tabular-nums text-muted">{{ timeText(item) }}</span>
              </span>
              </span>
            </NuxtLink>
          </li>
        </ul>

        <div ref="foot" class="py-1 text-center">
          <ContentSkeleton v-if="loading && adding" :rows="2" listClass="flex flex-col gap-2 text-start" :label="$t('archive.status.more_loading')">
          <ArchiveRowSkeleton />
          </ContentSkeleton>
          <p v-else-if="items.length > 0 && !hasMore" class="text-sm text-muted">{{ $t("archive.status.end") }}</p>
        </div>
      </div>
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { useFootInView } from "bmo-ui/app/domain/foot";
import { useListPlace } from "bmo-ui/app/domain/list-place";
import { countText } from "bmo-ui/app/domain/numbers";
import { formatStamp, shareLeftText, useNowSeconds } from "bmo-ui/app/domain/time";
import { matchFields, searchPhrase } from "bmo-ui/shared/search";
import { useI18n } from "vue-i18n";

import { SEARCHES_PAGE_SIZE, SEARCHES_PHRASE_LIMIT, searchTitle } from "../../../shared/config";
import { archiveKind, archiveSearchFields, depthText, roomName, statusView, subjectLine, subjectText } from "../../domain/archive";
import { useArchiveKinds } from "../../domain/kinds";

import type { SearchMatch } from "bmo-ui/shared/search";
import type { ArchiveKind, SearchPage, SearchSummary } from "../../../shared/types";

// The documents of the Archive, newest first: what a search found, a page imported whole, study
// material and the records the agent keeps. Each row opens its document to read, with its contents
// and the pages it stands on beside it.

// The kinds a reader may hold the list to, out of the catalogue the read API answers, and every kind
// at once. The catalogue decides what there is, so a kind added there is a choice here without this
// screen being told.
const { t } = useI18n();
const ALL_KINDS = "all";
const kinds = useArchiveKinds();
const kindChoices = computed(() => [{ label: t("common.label.all"), value: ALL_KINDS }, ...kinds.value.map((kind) => ({ label: t(kind.labelKey), value: kind.id }))]);

// How long typing pauses before what is typed is searched for.
const SEARCH_SETTLE_MS = 250;

// A status's mark stands in the status's own colour.
const STATUS_TEXT: Record<string, string> = {
  success: "text-success",
  info: "text-info",
  warning: "text-warning",
  error: "text-error",
  neutral: "text-muted",
  primary: "text-primary"
};

// Opening a document and coming back is one move, so the list is not built again for it: the router
// keeps the page, which keeps every page of rows already read, and the scroll goes back with it.
definePageMeta({ keepalive: true });

const route = useRoute();
const router = useRouter();
const bodyRef = useTemplateRef<HTMLElement>("body");

// Which documents the list is held to, as the address spells it. It is in the address rather than in
// this visit alone, so the filtered list opens the same way from a link and is still filtered on
// coming back to the screen.
const sharedOnly = computed(() => route.query.shared === "1");
// A toggle is named by the list it would leave you with, not by the list you are on.
const sharedLabel = computed(() => (sharedOnly.value ? t("archive.label.show_all") : t("archive.label.shared_only")));
const { now } = useNowSeconds();
// Which filter asked for the read now running, so the control that asked carries the spinner and the
// reread button beside it does not. Null while no filter's read is running.
const filtering = ref<"shared" | "kind" | "phrase" | null>(null);

function holdToShared(only: boolean): void {
  filtering.value = "shared";
  void router.replace({ query: only ? { ...route.query, shared: "1" } : { ...route.query, shared: undefined } });
}

// Which kind of document the list is held to, in the address for the same reason the link filter is.
// A word the catalogue does not know holds it to nothing and reads as every kind. Until the catalogue
// is read the word stands as it is, which the read API reads the same way, so the list does not wait.
const askedKind = computed(() => (typeof route.query.kind === "string" && route.query.kind !== "" ? route.query.kind : ALL_KINDS));
const kindChoice = computed(() => (kinds.value.length === 0 ? askedKind.value : (kinds.value.find((kind) => kind.id === askedKind.value)?.id ?? ALL_KINDS)));

function holdToKind(value: string): void {
  if (value === kindChoice.value) return;

  filtering.value = "kind";
  void router.replace({ query: { ...route.query, kind: value === ALL_KINDS ? undefined : value } });
}

// The kind one row is, in the catalogue's own words; none while the catalogue is unread.
function kindOf(item: SearchSummary): ArchiveKind | null {
  return archiveKind(kinds.value, item.kind);
}

// The phrase the list is held to, in the address like the other filters. What is typed reaches the
// address once typing pauses, so a read is not asked for on every keystroke.
const phrase = computed(() => searchPhrase(typeof route.query.q === "string" ? route.query.q : ""));
const typed = ref(phrase.value);
// The phrase the rows on the screen were found by, which is what their marks are drawn from.
const searched = ref("");
let settling: ReturnType<typeof setTimeout> | null = null;

onScopeDispose(() => {
  if (settling !== null) clearTimeout(settling);
});

function holdToPhrase(): void {
  if (settling !== null) clearTimeout(settling);
  settling = null;

  const asked = searchPhrase(typed.value);

  if (asked === phrase.value) return;

  filtering.value = "phrase";
  void router.replace({ query: { ...route.query, q: asked === "" ? undefined : asked } });
}

function settleSearch(): void {
  if (settling !== null) clearTimeout(settling);

  settling = setTimeout(holdToPhrase, SEARCH_SETTLE_MS);
}

function clearSearch(): void {
  typed.value = "";
  holdToPhrase();
}

// An address that moves on its own — a link, the back button — carries the field with it.
watch(phrase, (held) => {
  if (searchPhrase(typed.value) !== held) typed.value = held;
});

const emptyText = computed(() => {
  if (searched.value !== "") return t("archive.status.no_match", { phrase: searched.value });

  return sharedOnly.value ? t("archive.status.no_shared") : t("archive.status.no_documents");
});

useListPlace("archive", () => bodyRef.value?.parentElement ?? null);

const items = ref<SearchSummary[]>([]);
const hasMore = ref(false);
const loading = ref(false);
const loaded = ref(false), adding = ref(false);
const failed = ref("");
let generation = 0;
onScopeDispose(() => { generation++; });

const foot = useTemplateRef<HTMLElement>("foot");

// When it started, and when it ended — the end's date only where it is not the start's.
function timeText(item: SearchSummary): string {
  const started = formatStamp(item.startedAt);

  if (item.finishedAt === null) {
    return t("archive.label.started_at", { time: started });
  }

  const finished = formatStamp(item.finishedAt);

  return `${started} ~ ${finished.slice(0, 10) === started.slice(0, 10) ? finished.slice(11) : finished}`;
}

// Where one row matched the phrase it was found by.
function rowMatch(item: SearchSummary): SearchMatch | null {
  return matchFields(archiveSearchFields(t, item, roomName(item)), searched.value);
}

async function load(before: number | null): Promise<void> {
  const mine = ++generation;
  const asked = phrase.value;
  loading.value = true;
  adding.value = before !== null;
  failed.value = "";

  try {
    const answer = await $fetch<SearchPage>("/api/archive", {
      query: {
        limit: SEARCHES_PAGE_SIZE,
        before: before === null ? undefined : before,
        shared: sharedOnly.value ? "1" : undefined,
        kind: kindChoice.value === ALL_KINDS ? undefined : kindChoice.value,
        q: asked === "" ? undefined : asked
      }
    });

    if (mine !== generation) return;
    loaded.value = true;
    hasMore.value = answer.hasMore;
    items.value = before === null ? answer.items : [...items.value, ...answer.items];
    searched.value = asked;
  } catch (error) {
    if (mine === generation) failed.value = failureText(error, t("archive.error.list_failed"));
  } finally {
    if (mine === generation) {
      loading.value = false;
      filtering.value = null;
    }
  }
}

function reread(): void {
  void load(null);
}

// The foot of the list standing in view is what asks for the next page.
const atFoot = useFootInView(foot, loading);

watch([atFoot, loading, hasMore], () => {
  const last = items.value[items.value.length - 1];

  if (atFoot.value && !loading.value && hasMore.value && last !== undefined && failed.value === "") {
    void load(last.id);
  }
});

// The filter moving is the list being read again from its head, since a page of it is a page of what
// the filter names rather than a page of everything with the rest taken out.
watch([sharedOnly, kindChoice], () => {
  items.value = [];
  loaded.value = false;
  hasMore.value = false;
  void load(null);
});

// A new phrase is read from the head as well, but the rows found by the last one stay, marked as they
// were found, until the new ones arrive: the field spins meanwhile, rather than the list dropping to a
// skeleton every time typing pauses.
watch(phrase, () => {
  hasMore.value = false;
  void load(null);
});

onMounted(() => {
  void load(null);
});
</script>
