<template>
  <!-- What a document still lacks, said as missing in one still, quiet badge on that document alone; it never turns,
       since nothing says anybody is at work on it (docs/artifact.md, 「A document that lacks a part」). -->
  <UBadge v-if="lacks !== null" color="neutral" variant="soft" size="sm" class="max-w-full" :label="said" data-lacks />
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { lacksOf } from "../domain/document-view";

import type { ArtifactDocument } from "../../shared/types";

const props = defineProps<{ document: Pick<ArtifactDocument, "labels"> }>();
const { t } = useI18n();

const lacks = computed(() => lacksOf(props.document));

// One lacking part in the pack's words where it has them, and as the filer wrote it where not.
function partLabel(word: string): string {
  const key = `artifact.pending.${word}`;
  const said = t(key);

  return said === key ? word : said;
}

const said = computed(() => t("artifact.label.lacks", { lacks: (lacks.value ?? []).map(partLabel).join(", ") }));
</script>
