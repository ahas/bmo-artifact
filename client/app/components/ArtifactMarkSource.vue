<template>
  <!-- What a footnote mark's source entry says (docs/artifact.md, "How a document is drawn"): its mark, then its
       words as the text formats them, each page it names as a link that opens in a tab of its own, and, for a snippet's mark, that
       the value was checked against the snippet only. The words were read off the text, which was cleaned whole. -->
  <div class="flex flex-col gap-2 text-sm">
    <p class="break-words text-default [&_code]:font-mono [&_strong]:font-semibold">
      <span class="me-1 font-medium text-highlighted tabular-nums" data-mark-label>{{ props.source.mark }}</span>
      <span v-html="props.source.html" />
    </p>
    <a
      v-for="address in props.source.addresses"
      :key="address"
      :href="address"
      target="_blank"
      rel="noopener noreferrer"
      class="break-all text-default underline decoration-primary underline-offset-2 hover:text-highlighted active:text-highlighted"
      data-mark-address
    >
      {{ addressText(address) }}
    </a>
    <p v-if="props.source.snippet" class="text-muted">{{ $t("artifact.label.snippet_only") }}</p>
  </div>
</template>

<script setup lang="ts">
import { addressText } from "../domain/footnotes";

import type { MarkSource } from "../domain/footnotes";

const props = defineProps<{ source: MarkSource }>();
</script>
