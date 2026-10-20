<template>
  <!-- The map of a place a finding names, at the column's width and a fixed height, held by a
       skeleton until it has loaded. -->
  <div class="relative h-64 w-full overflow-hidden rounded-md border border-default bg-elevated sm:h-80" :aria-busy="!loaded">
    <SkeletonStatus v-if="!loaded" class="absolute inset-0 flex" :label="$t('artifact.status.map_loading')">
      <SkeletonBar class="size-full" />
    </SkeletonStatus>

    <iframe
      :src="address"
      :title="$t('artifact.label.link_map')"
      loading="lazy"
      referrerpolicy="no-referrer-when-downgrade"
      :sandbox="MAP_SANDBOX"
      class="block size-full border-0"
      @load="loaded = true"
    />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

// Where the map is drawn from. The map is Google's own embedded one, which needs no key, found by the
// words the finding names the place with. The service is this component's and never the text's: a
// finding chooses only the place, so a line written in it does not make the reader's browser call a
// server of its choosing.
const MAP_EMBED = "https://www.google.com/maps";

// What the embedded map may do in its frame: run its own scripts on its own origin, and open the
// map's own page in a tab of its own.
const MAP_SANDBOX = "allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox";

const props = defineProps<{ query: string }>();
const { locale } = useI18n();
const loaded = ref(false);

const address = computed(() => `${MAP_EMBED}?${new URLSearchParams({ q: props.query, hl: locale.value, output: "embed" }).toString()}`);
</script>
