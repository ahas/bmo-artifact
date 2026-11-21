<!-- One document as a card of the front page (docs/artifact.md, 「The front page」): its cover, its category and what it
     lacks, its title, its description, the day it was filed and, where a live link stands, that it is public. The
     title's link is stretched over the whole card, so the card opens the document wherever it is pressed.
     A card stands in a group named `cards` and turns into a row, its cover at the leading edge, where that group is
     narrow; the lead card of the newest stands over its words at every width and fills the height beside it. -->
<template>
  <article
    class="relative flex min-w-0 flex-col overflow-hidden rounded-lg border border-default transition-colors select-none hover:bg-elevated/50 active:bg-accented/50"
    :class="props.lead ? 'h-full' : '@max-md/cards:flex-row'"
    :data-card="props.document.id"
  >
    <ArtifactCover
      :document="props.document"
      :kind="props.kind"
      :class="props.lead ? 'aspect-video w-full @xl/newest:aspect-auto @xl/newest:min-h-48 @xl/newest:flex-1' : 'aspect-video w-full @max-md/cards:aspect-[4/3] @max-md/cards:w-28'"
    />

    <div class="flex min-w-0 flex-col gap-2 p-3" :class="props.lead ? '' : '@max-md/cards:flex-1 @max-md/cards:justify-center @max-md/cards:gap-1'">
      <span v-if="badged" class="flex min-w-0 flex-wrap items-center gap-2" :class="props.lead ? '' : '@max-md/cards:hidden'">
        <UBadge v-if="props.withKind" color="neutral" variant="outline" size="sm" :label="kindLabel(props.document.kind)" data-card-kind />
        <UBadge v-if="category !== null" color="info" variant="soft" size="sm" class="max-w-full" :label="categoryLabel(category)" data-category />
        <ArtifactLackMark :document="props.document" />
      </span>

      <NuxtLink
        :to="`/artifact/${props.document.id}`"
        class="line-clamp-2 font-medium break-words text-highlighted focus-visible:outline-none before:absolute before:inset-0 before:rounded-lg before:content-[''] focus-visible:before:outline-solid"
        :class="props.lead ? 'text-xl' : 'text-sm'"
        data-card-title
      >
        {{ props.document.title }}
      </NuxtLink>

      <p
        v-if="props.document.description !== null"
        class="break-words text-muted"
        :class="props.lead ? 'line-clamp-2 text-base' : 'line-clamp-2 text-sm @max-md/cards:line-clamp-1'"
        data-card-description
      >
        {{ props.document.description }}
      </p>

      <span class="flex min-w-0 flex-wrap items-center gap-2 text-sm text-muted tabular-nums">
        <span data-card-day>{{ formatShortDay(t, Math.floor(props.document.createdAt / 1000)) }}</span>
        <UBadge v-if="props.public" color="primary" variant="subtle" size="sm" icon="i-material-symbols-public" :label="$t('artifact.label.public')" data-shared />
      </span>
    </div>
  </article>
</template>

<script setup lang="ts">
import { formatShortDay } from "bmo-ui/app/domain/time";
import { useI18n } from "vue-i18n";

import { categoryOf, lacksOf } from "../domain/document-view";
import { useDocumentWords } from "../domain/document-words";

import type { ArtifactDocument, ArtifactKind } from "../../shared/types";

const props = defineProps<{
  document: ArtifactDocument;
  // Its kind in the catalogue, null while the catalogue is unread.
  kind: ArtifactKind | null;
  // The catalogue, for the kind's word.
  kinds: readonly ArtifactKind[];
  // Whether a live link to it stands.
  public: boolean;
  // Whether it leads the newest.
  lead?: boolean;
  // Whether it names its kind, as the newest's cards do, which mix the kinds.
  withKind?: boolean;
}>();

const { t } = useI18n();
const { kindLabel, categoryLabel } = useDocumentWords(toRef(props, "kinds"));
const category = computed(() => categoryOf(props.document));
// Whether any badge stands over the title.
const badged = computed(() => props.withKind === true || category.value !== null || lacksOf(props.document) !== null);
</script>
