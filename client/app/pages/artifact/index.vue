<template>
  <UDashboardPanel id="artifact-documents">
    <template #header>
      <!-- Under a console that offers its title bar the console draws it; alone, the screen draws its own. -->
      <UDashboardNavbar v-if="!barOffered" :title="$t('artifact.title.screen')" icon="i-material-symbols-archive-outline" />

      <!-- The search takes what width the row has, and the filters and the reread keep theirs. -->
      <UDashboardToolbar :ui="{ left: 'min-w-0 flex-1', right: 'shrink-0' }">
        <template #left>
          <UInput
            v-model="typed"
            size="sm"
            class="min-w-0 flex-1"
            icon="i-material-symbols-search"
            :placeholder="$t('artifact.placeholder.search')"
            :aria-label="$t('artifact.aria.search')"
            :maxlength="DOCUMENTS_PHRASE_LIMIT"
            :loading="loading && searched !== ''"
            :ui="{ trailing: 'pe-1' }"
            @update:model-value="settleSearch"
          >
            <template v-if="typed !== ''" #trailing>
              <UTooltip :text="$t('artifact.action.clear_search')">
                <UButton icon="i-material-symbols-close" color="neutral" variant="ghost" size="sm" :aria-label="$t('artifact.action.clear_search')" @click="clearSearch" />
              </UTooltip>
            </template>
          </UInput>
        </template>

        <template #right>
          <span v-if="loading && loaded" role="status" class="sr-only">{{ $t("artifact.status.refreshing") }}</span>

          <USelect
            :model-value="kindChoice"
            :items="kindChoices"
            size="sm"
            color="neutral"
            variant="ghost"
            :aria-label="$t('artifact.aria.kind')"
            class="w-auto"
            @update:model-value="holdToKind"
          />

          <USelect
            :model-value="categoryChoice"
            :items="categoryChoices"
            size="sm"
            color="neutral"
            variant="ghost"
            :aria-label="$t('artifact.aria.category')"
            class="w-auto"
            @update:model-value="holdToCategory"
          />

          <UButton
            size="sm"
            color="neutral"
            :variant="inProgressOnly ? 'soft' : 'ghost'"
            icon="i-material-symbols-progress-activity"
            :label="$t('artifact.label.in_progress_only')"
            :aria-pressed="inProgressOnly"
            :ui="{ label: 'max-sm:sr-only' }"
            @click="holdToInProgress"
          />

          <UTooltip :text="$t('artifact.action.refresh')">
            <UButton size="sm" color="neutral" variant="ghost" icon="i-material-symbols-refresh" :loading="loading" :aria-label="$t('artifact.action.refresh')" @click="reread" />
          </UTooltip>
        </template>
      </UDashboardToolbar>
    </template>

    <template #body>
      <div class="flex flex-col gap-3">
        <div v-if="failed !== ''" role="alert" class="flex flex-wrap items-center gap-2">
          <p class="text-sm text-error">{{ failed }}</p>
          <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" icon="i-material-symbols-refresh" :label="$t('artifact.action.refresh')" :loading="loading" @click="reread" />
        </div>

        <ContentSkeleton v-else-if="!loaded" :rows="4" listClass="flex flex-col gap-2" :label="$t('artifact.status.document_loading')">
          <ArtifactRowSkeleton />
        </ContentSkeleton>
        <p v-else-if="items.length === 0 && !loading" class="text-sm text-muted">{{ emptyText }}</p>

        <ul :aria-busy="loading" class="flex flex-col gap-2">
          <!-- The whole row opens its document through the title's link stretched over it, and the number
               stands above that link, so pressing it copies the number and opens nothing. -->
          <li v-for="item in items" :key="item.id">
            <div class="relative flex items-start gap-3 rounded-lg border border-default px-4 py-3 transition-colors hover:bg-elevated/50 active:bg-accented/50">
              <img
                v-if="item.hasPicture && !unpictured.has(item.id)"
                :src="pictureAddress(item.id)"
                alt=""
                loading="lazy"
                decoding="async"
                class="size-16 shrink-0 rounded-md border border-default bg-elevated object-cover sm:size-20"
                @error="unpictured.add(item.id)"
              >

              <div class="flex min-w-0 flex-1 flex-col gap-2">
                <NuxtLink :to="`/artifact/${item.id}`" class="line-clamp-2 text-sm font-medium break-words text-highlighted before:absolute before:inset-0 before:rounded-lg before:content-['']">
                  <SearchText :text="item.title" :query="searched" />
                </NuxtLink>
                <p v-if="item.description !== null" class="line-clamp-2 text-sm break-words text-muted"><SearchText :text="item.description" :query="searched" /></p>

                <!-- A row carries at most four badges, its kind, its category, its progress and whether it is public; its other
                     labels are search metadata and stand in the document's own view (docs/artifact.md, 「Where a person
                     reads it」). -->
                <span class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                  <UTooltip :text="copiedId === item.id ? $t('common.status.copied') : $t('artifact.action.copy_number')">
                    <button
                      type="button"
                      class="relative rounded-sm text-sm font-medium tabular-nums text-muted hover:text-highlighted"
                      :aria-label="`${$t('artifact.action.copy_number')} ${numberText(item.id)}`"
                      data-number
                      @click="copyNumber(item.id)"
                    >
                      <SearchText :text="numberText(item.id)" :query="searched" />
                    </button>
                  </UTooltip>
                  <UBadge color="neutral" variant="outline" size="sm" :label="kindLabel(item.kind)" />
                  <UBadge
                    v-if="categoryOf(item) !== null"
                    color="info"
                    variant="soft"
                    size="sm"
                    class="max-w-full"
                    :label="categoryLabel(categoryOf(item)!)"
                    data-category
                  />
                  <ArtifactPendingBadge :document="item" />
                  <span v-if="item.attachmentCount > 0" class="text-sm text-muted tabular-nums">
                    {{ $t("artifact.label.files_count", { n: countText(item.attachmentCount) }) }}
                  </span>
                  <!-- Right of the day, whether the document is public: a live link to it makes it public, until the
                       day the link ends where it has one; with no link it is private. The two keep to the row's
                       trailing edge together, the badge going under the day where the row runs out of width. -->
                  <span class="ms-auto flex min-w-0 flex-wrap items-center justify-end gap-x-2 gap-y-1">
                    <span class="text-sm tabular-nums text-muted">{{ formatStamp(Math.floor(item.createdAt / 1000)) }}</span>
                    <UBadge
                      v-if="shares?.has(item.id)"
                      color="primary"
                      variant="subtle"
                      size="sm"
                      icon="i-material-symbols-public"
                      class="max-w-full"
                      :label="visibilityLabel(shareEnd(shares!.get(item.id)!))"
                      data-shared
                    />
                    <UBadge
                      v-else-if="shares !== null"
                      color="neutral"
                      variant="outline"
                      size="sm"
                      icon="i-material-symbols-lock-outline"
                      :label="$t('artifact.label.private')"
                      data-private
                    />
                  </span>
                </span>
              </div>
            </div>
          </li>
        </ul>

        <div v-if="more" class="flex justify-center">
          <UButton size="sm" color="neutral" variant="subtle" :label="$t('artifact.action.more')" :loading="loading" @click="readMore" />
        </div>
      </div>
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { countText } from "bmo-ui/app/domain/numbers";
import { formatShortDay, formatStamp } from "bmo-ui/app/domain/time";
import { inject } from "vue";
import { useI18n } from "vue-i18n";

import { DOCUMENTS_PAGE_SIZE, DOCUMENTS_PHRASE_LIMIT } from "../../../shared/config";
import { pictureAddress, readDocuments, readLabelValues, readLiveShares, useKinds } from "../../domain/artifact";
import { copyText } from "../../domain/copy";
import { CATEGORY_LABEL, PENDING_LABEL, categoryFilter, categoryOf, kindOf, numberText } from "../../domain/document-view";

import type { ArtifactDocument, ArtifactLabelValue, ArtifactShare } from "../../../shared/types";

// The documents the service keeps, newest first, narrowed by kind, by category or to those in progress, and
// found by a phrase; each narrowing is held in the address, so a reload or a shared address shows the same list.

const { t } = useI18n();
// Whether a console offers its title bar, under the key bmo-ui's bundle contract names.
const barOffered = inject("bmo-console-title-bar", null) !== null;
const route = useRoute();
const router = useRouter();
const kinds = useKinds();

const ALL = "all";
const typed = ref(typeof route.query.q === "string" ? route.query.q : "");
const searched = ref(typed.value.trim());
const kindChoice = ref(typeof route.query.kind === "string" ? route.query.kind : ALL);
const categoryChoice = ref(typeof route.query.category === "string" ? route.query.category : ALL);
const inProgressOnly = ref(route.query.pending === "1" && categoryChoice.value === ALL);
// The categories the documents carry, the most carried first; a list read with none of them offers the
// filter's one choice of every category rather than failing.
const categories = ref<ArtifactLabelValue[]>([]);
// The document whose number was just copied, said so in its tooltip for a moment.
const copiedId = ref<number | null>(null);
let copiedFor: ReturnType<typeof setTimeout> | null = null;

const items = ref<ArtifactDocument[]>([]);
const more = ref(false);
const loading = ref(false);
const loaded = ref(false);
const failed = ref("");
// The documents whose cover picture did not load, each left without one rather than drawn broken.
const unpictured = ref(new Set<number>());
// The live links by the document each opens, read beside every reading of the list. Until they are read, and where
// reading them failed, no row says whether it is public, rather than calling every document private.
const shares = ref<Map<number, ArtifactShare> | null>(null);

// When a link ends, in the seconds the share words read, or null for a link with no end.
function shareEnd(link: ArtifactShare): number | null {
  return link.expiresAt === null ? null : Math.floor(link.expiresAt / 1000);
}

// What a document with a live link is: public, and public until the day its link ends where it has one. The
// day is named, never the time left, which reads a day short or long as the hours turn.
function visibilityLabel(end: number | null): string {
  return end === null ? t("artifact.label.public") : t("artifact.label.public_until", { date: formatShortDay(t, end) });
}
let generation = 0;

onScopeDispose(() => {
  generation++;
  if (copiedFor !== null) clearTimeout(copiedFor);
});

// A kind as the list names it: in its own words where the pack has them, and as its name where not.
function kindLabel(name: string): string {
  const key = `artifact.kind.${name}`;
  const said = t(key);

  return said === key ? (kindOf(kinds.value, name)?.name ?? name) : said;
}

// A category as the list names it: in the pack's word where it has one, and as the filer wrote it where not.
function categoryLabel(word: string): string {
  const key = `artifact.category.${word}`;
  const said = t(key);

  return said === key ? word : said;
}

// The categories offered, the one held in the address among them though no document carries it now.
const categoryChoices = computed(() => {
  const words = categories.value.map((category) => category.value);

  if (categoryChoice.value !== ALL && !words.includes(categoryChoice.value)) words.push(categoryChoice.value);

  return [{ label: t("artifact.label.all_categories"), value: ALL }, ...words.map((word) => ({ label: categoryLabel(word), value: word }))];
});

async function copyNumber(id: number): Promise<void> {
  if (!(await copyText(numberText(id)))) return;

  copiedId.value = id;
  if (copiedFor !== null) clearTimeout(copiedFor);
  copiedFor = setTimeout(() => {
    copiedFor = null;
    copiedId.value = null;
  }, 1500);
}

const kindChoices = computed(() => [
  { label: t("artifact.label.all_kinds"), value: ALL },
  ...kinds.value.map((kind) => ({ label: kindLabel(kind.name), value: kind.name }))
]);

const emptyText = computed(() => (searched.value === "" ? t("artifact.status.no_documents") : t("artifact.status.no_match", { phrase: searched.value })));

// The one label the list is narrowed by: the documents in progress while the switch is on, else the category chosen.
function listedLabel(): string | undefined {
  if (inProgressOnly.value) return PENDING_LABEL;

  return categoryChoice.value === ALL ? undefined : categoryFilter(categoryChoice.value);
}

async function read(before?: number): Promise<void> {
  const mine = ++generation;

  loading.value = true;
  failed.value = "";

  if (before === undefined) {
    void readLiveShares().then(
      (links) => {
        if (mine === generation) shares.value = links;
      },
      () => undefined
    );
    void readLabelValues(CATEGORY_LABEL).then(
      (words) => {
        if (mine === generation) categories.value = words;
      },
      () => undefined
    );
  }

  try {
    const page = await readDocuments({
      kind: kindChoice.value === ALL ? undefined : kindChoice.value,
      label: listedLabel(),
      q: searched.value === "" ? undefined : searched.value,
      before,
      limit: DOCUMENTS_PAGE_SIZE
    });

    if (mine !== generation) return;

    items.value = before === undefined ? page.items : [...items.value, ...page.items];
    more.value = page.more;
    loaded.value = true;
  } catch (error) {
    if (mine === generation) failed.value = failureText(error, t("artifact.error.list_failed"));
  } finally {
    if (mine === generation) loading.value = false;
  }
}

function hold(): void {
  void router.replace({
    query: {
      ...(searched.value === "" ? {} : { q: searched.value }),
      ...(kindChoice.value === ALL ? {} : { kind: kindChoice.value }),
      ...(categoryChoice.value === ALL ? {} : { category: categoryChoice.value }),
      ...(inProgressOnly.value ? { pending: "1" } : {})
    }
  });
  void read();
}

// The list is asked again once typing has paused, so a phrase is not searched a letter at a time.
let settling: ReturnType<typeof setTimeout> | null = null;

function settleSearch(): void {
  if (settling !== null) clearTimeout(settling);

  settling = setTimeout(() => {
    settling = null;
    searched.value = typed.value.trim();
    hold();
  }, 300);
}

function clearSearch(): void {
  typed.value = "";
  searched.value = "";
  hold();
}

function holdToKind(value: string): void {
  kindChoice.value = value;
  hold();
}

// The route takes one label, so the category and the in-progress switch narrow the list one at a time.
function holdToCategory(value: string): void {
  categoryChoice.value = value;
  if (value !== ALL) inProgressOnly.value = false;
  hold();
}

function holdToInProgress(): void {
  inProgressOnly.value = !inProgressOnly.value;
  if (inProgressOnly.value) categoryChoice.value = ALL;
  hold();
}

function reread(): void {
  void read();
}

function readMore(): void {
  void read(items.value[items.value.length - 1]?.id);
}

useHead({ title: computed(() => t("artifact.title.screen")) });

onMounted(() => void read());
</script>
