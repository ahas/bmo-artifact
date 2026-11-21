<!-- The documents most read in the last seven days (docs/artifact.md, 「The front page」 and 「Reads」): each its rank
     and its title on one line, the whole line opening the document. A title cut at the line's end shows whole in the
     console's tooltip for a cut line. -->
<template>
  <ol class="flex flex-col" data-most-read>
    <li v-for="(item, index) in props.items" :key="item.document.id">
      <NuxtLink
        :to="`/artifact/${item.document.id}`"
        class="flex min-w-0 items-center gap-3 rounded-md px-2 py-2 text-sm transition-colors select-none hover:bg-elevated/50 active:bg-accented/50"
        data-most-read-item
      >
        <span class="w-5 shrink-0 text-center font-semibold tabular-nums text-highlighted">{{ countText(index + 1) }}</span>
        <span class="min-w-0 truncate text-default">{{ item.document.title }}</span>
      </NuxtLink>
    </li>
  </ol>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import type { ArtifactDocumentReads } from "../../shared/types";

const props = defineProps<{ items: readonly ArtifactDocumentReads[] }>();
</script>
