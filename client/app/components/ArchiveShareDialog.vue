<template>
  <!-- The share link of one document of the Archive, on the one dialog every share link stands in. -->
  <ShareLinkDialog v-model:open="open" :road="road" @shared="(mark) => emit('shared', mark)" />
</template>

<script setup lang="ts">
import type { ShareRoad } from "../domain/share-links";
import type { SearchShareMark, SearchShareView } from "../../shared/types";

// One link per document, read, made and taken away under the document's own number through the share
// API; its page is the console's `/share/<token>` (`docs/api.md`, "Share links (HTTP)"). Who makes it is
// the signed-in user, whom the console names on the way.

interface Props {
  searchId: number;
}

const props = defineProps<Props>();
const open = defineModel<boolean>("open", { default: false });
const emit = defineEmits<{ shared: [mark: SearchShareMark | null] }>();

const road = computed<ShareRoad>(() => {
  const at = `/api/archive/${props.searchId}/share`;

  return {
    page: (token) => `/share/${token}`,
    read: async () => (await $fetch<{ share: SearchShareView | null }>(at)).share,
    make: async (span) => (await $fetch<{ share: SearchShareView }>(at, { method: "POST", body: { span } })).share,
    remove: async () => {
      await $fetch(at, { method: "DELETE" });
    }
  };
});
</script>
