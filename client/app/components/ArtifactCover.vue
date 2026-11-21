<!-- A document's cover on the front page (docs/artifact.md, 「The front page」): its cover picture where it keeps one
     and the picture loads, and the kind's icon on a muted ground in its place otherwise, so a strip with few pictures
     still reads as a strip. Its size and shape are the card's. -->
<template>
  <div class="relative grid shrink-0 place-items-center overflow-hidden bg-elevated text-muted" data-cover>
    <img
      v-if="pictured"
      :src="pictureAddress(props.document.id)"
      alt=""
      loading="lazy"
      decoding="async"
      class="absolute inset-0 size-full object-cover"
      data-cover-picture
      @error="broken = true"
    >
    <UIcon v-else :name="kindIcon(props.kind)" class="size-8" data-cover-icon />
  </div>
</template>

<script setup lang="ts">
import { pictureAddress } from "../domain/artifact";
import { kindIcon } from "../domain/document-words";

import type { ArtifactDocument, ArtifactKind } from "../../shared/types";

const props = defineProps<{
  document: Pick<ArtifactDocument, "id" | "hasPicture">;
  kind: ArtifactKind | null;
}>();

// Whether its picture failed to load, which leaves the kind's icon in its place.
const broken = ref(false);
const pictured = computed(() => props.document.hasPicture && !broken.value);

watch(
  () => props.document.id,
  () => {
    broken.value = false;
  }
);
</script>
