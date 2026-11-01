<template>
  <!-- A document in progress, with what its filer still has to give it (docs/archive.md, 「Where a person reads it」). -->
  <UBadge v-if="lacks !== null" color="warning" variant="subtle" size="sm" icon="i-material-symbols-progress-activity" class="max-w-full" :label="said" data-pending />
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { pendingOf } from "../domain/document-view";

import type { ArtifactDocument } from "../../shared/types";

const props = defineProps<{ document: Pick<ArtifactDocument, "labels"> }>();
const { t } = useI18n();

const lacks = computed(() => pendingOf(props.document));

// One lacking part in the pack's words where it has them, and as the filer wrote it where not.
function lackLabel(word: string): string {
  const key = `artifact.pending.${word}`;
  const said = t(key);

  return said === key ? word : said;
}

const said = computed(() => t("artifact.label.in_progress", { lacks: (lacks.value ?? []).map(lackLabel).join(", ") }));
</script>
