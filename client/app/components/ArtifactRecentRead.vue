<!-- The documents the reader read last (docs/artifact.md, 「The front page」 and 「Reads」), the one opened last first:
     each its title on one line, the whole line opening the document. A title cut at the line's end shows whole in
     the console's tooltip for a cut line. -->
<template>
  <ol class="flex flex-col" data-recent-read>
    <li v-for="item in props.items" :key="item.document.id">
      <NuxtLink
        :to="`/artifact/${item.document.id}`"
        class="flex min-w-0 items-center gap-3 rounded-md px-2 py-2 text-sm transition-colors select-none hover:bg-elevated/50 active:bg-accented/50"
        data-recent-read-item
      >
        <span class="min-w-0 truncate text-default">{{ item.document.title }}</span>
      </NuxtLink>
    </li>
  </ol>
</template>

<script setup lang="ts">
import type { ArtifactRecentRead } from "../../shared/types";

const props = defineProps<{ items: readonly ArtifactRecentRead[] }>();
</script>
