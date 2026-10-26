<template>
  <UDashboardPanel id="artifact-documents">
    <template #header>
      <!-- Under a console that offers its title bar the console draws it; alone, the screen draws its own. -->
      <UDashboardNavbar v-if="!barOffered" :title="$t('artifact.title.screen')" icon="i-material-symbols-archive-outline" />

      <!-- The search takes what width the row has, and the kind filter and the reread keep theirs. -->
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
          <li v-for="item in items" :key="item.id">
            <NuxtLink
              :to="`/archive/${item.id}`"
              class="flex items-start gap-3 rounded-lg border border-default px-4 py-3 transition-colors hover:bg-elevated/50 active:bg-accented/50"
            >
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
                <SearchText class="line-clamp-2 text-sm font-medium break-words text-highlighted" :text="item.title" :query="searched" />
                <p v-if="item.description !== null" class="line-clamp-2 text-sm text-muted"><SearchText :text="item.description" :query="searched" /></p>

                <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
                  <UBadge color="neutral" variant="outline" size="sm" :label="kindLabel(item.kind)" />
                  <UBadge
                    v-for="[name, value] in Object.entries(item.labels)"
                    :key="name"
                    color="neutral"
                    variant="subtle"
                    size="sm"
                    :label="`${name}: ${value}`"
                  />
                  <span v-if="item.attachmentCount > 0" class="text-sm text-muted tabular-nums">
                    {{ $t("artifact.label.files_count", { n: countText(item.attachmentCount) }) }}
                  </span>
                  <span class="ms-auto text-sm tabular-nums text-muted">{{ formatStamp(Math.floor(item.createdAt / 1000)) }}</span>
                </span>
              </div>
            </NuxtLink>
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
import { formatStamp } from "bmo-ui/app/domain/time";
import { inject } from "vue";
import { useI18n } from "vue-i18n";

import { DOCUMENTS_PAGE_SIZE, DOCUMENTS_PHRASE_LIMIT } from "../../../shared/config";
import { pictureAddress, readDocuments, useKinds } from "../../domain/artifact";
import { kindOf } from "../../domain/document-view";

import type { ArtifactDocument } from "../../../shared/types";

// The documents the service keeps, newest first, narrowed by kind and found by a phrase; each
// narrowing is held in the address, so a reload or a shared address shows the same list.

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

const items = ref<ArtifactDocument[]>([]);
const more = ref(false);
const loading = ref(false);
const loaded = ref(false);
const failed = ref("");
// The documents whose cover picture did not load, each left without one rather than drawn broken.
const unpictured = ref(new Set<number>());
let generation = 0;

onScopeDispose(() => {
  generation++;
});

// A kind as the list names it: in its own words where the pack has them, and as its name where not.
function kindLabel(name: string): string {
  const key = `artifact.kind.${name}`;
  const said = t(key);

  return said === key ? (kindOf(kinds.value, name)?.name ?? name) : said;
}

const kindChoices = computed(() => [
  { label: t("artifact.label.all_kinds"), value: ALL },
  ...kinds.value.map((kind) => ({ label: kindLabel(kind.name), value: kind.name }))
]);

const emptyText = computed(() => (searched.value === "" ? t("artifact.status.no_documents") : t("artifact.status.no_match", { phrase: searched.value })));

async function read(before?: number): Promise<void> {
  const mine = ++generation;

  loading.value = true;
  failed.value = "";

  try {
    const page = await readDocuments({
      kind: kindChoice.value === ALL ? undefined : kindChoice.value,
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
      ...(kindChoice.value === ALL ? {} : { kind: kindChoice.value })
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

function reread(): void {
  void read();
}

function readMore(): void {
  void read(items.value[items.value.length - 1]?.id);
}

useHead({ title: computed(() => t("artifact.title.screen")) });

onMounted(() => void read());
</script>
