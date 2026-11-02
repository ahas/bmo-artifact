<template>
  <!-- A document's instruction, the words it was asked for with (docs/artifact.md, 「Where a person reads it」),
       drawn as markdown in a sheet: from the side on a wide screen, from the bottom edge on a phone. -->
  <UDrawer v-if="phone" v-model:open="open" :title="$t('artifact.title.instruction')" :ui="FULL_SHEET">
    <template #body>
      <div class="min-h-0 flex-1 overflow-y-auto p-4">
        <ArtifactText :markdown="props.text" />
      </div>
    </template>
  </UDrawer>

  <USlideover v-else v-model:open="open" :title="$t('artifact.title.instruction')" :ui="{ body: 'p-4 sm:p-6' }">
    <template #body>
      <ArtifactText :markdown="props.text" />
    </template>
  </USlideover>
</template>

<script setup lang="ts">
import { useNarrow } from "bmo-ui/app/domain/screen";
import { FULL_SHEET } from "bmo-ui/app/domain/side-parts";

interface Props {
  // The instruction as it was given, in markdown.
  text: string;
}

const props = defineProps<Props>();
const open = defineModel<boolean>("open", { default: false });
const phone = useNarrow();
</script>
