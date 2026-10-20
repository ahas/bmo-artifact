<template>
  <!-- What stands beside a document's text: its contents, by its kind's depth, and the pages it refers
       to. -->
  <aside :aria-label="$t('artifact.label.references')" class="flex min-w-0 flex-col gap-4 py-4">
    <nav v-if="props.contents.length > 1" class="flex flex-col gap-1" :aria-label="$t('artifact.label.contents')">
      <h2 class="px-4 text-sm font-semibold text-muted">{{ $t("artifact.label.contents") }}</h2>
      <ul class="flex flex-col px-2">
        <li v-for="entry in props.contents" :key="entry.slug">
          <button
            type="button"
            class="w-full truncate rounded-md py-2 text-start text-sm text-default transition-colors hover:bg-elevated active:bg-accented"
            :style="{ paddingInlineStart: `${0.5 + (entry.depth - 1) * 0.75}rem` }"
            @click="emit('go', entry.slug)"
          >
            {{ entry.text }}
          </button>
        </li>
      </ul>
    </nav>

    <section class="flex flex-col gap-1">
      <h2 class="px-4 text-sm font-semibold text-muted">{{ $t("artifact.title.references_count", { n: countText(props.references.length) }) }}</h2>
      <p v-if="props.references.length === 0" class="px-4 text-sm text-muted">{{ $t("artifact.status.no_references") }}</p>
      <ul v-else class="flex flex-col px-2">
        <ArtifactReferenceItem
          v-for="reference in props.references"
          :key="reference.id"
          :reference="reference"
          :image="props.image(reference)"
          :read="props.read"
        />
      </ul>
    </section>
  </aside>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import type { ContentsEntry } from "../domain/document-view";
import type { ArtifactReference } from "../../shared/types";

interface Props {
  contents: readonly ContentsEntry[];
  references: readonly ArtifactReference[];
  // Where a reference's preview picture is served, and how one is read with its preview.
  image: (reference: ArtifactReference) => string;
  read: (reference: ArtifactReference) => Promise<ArtifactReference>;
}

const props = defineProps<Props>();
const emit = defineEmits<{ go: [slug: string] }>();
</script>
