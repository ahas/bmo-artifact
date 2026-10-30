<template>
  <!-- The listen band (docs/archive.md, "How a document is drawn"): the sounds heard at a markdown
       document's head, each in the design's own player and labelled with the language it is in where its
       name says one, under the masthead, where its players reach the document's files on the route the
       screen reads it on. -->
  <section :aria-label="$t('artifact.label.listen')" class="flex flex-col gap-2" data-listen>
    <p class="document-eyebrow">{{ $t("artifact.label.listen") }}</p>
    <div class="grid gap-x-6 gap-y-3" :class="props.sounds.length > 1 ? 'sm:grid-cols-2' : ''">
      <div v-for="sound in props.sounds" :key="sound.id" class="flex min-w-0 flex-col gap-1">
        <p v-if="languageOf(sound) !== null" class="text-sm font-medium text-muted" data-listen-language>{{ languageOf(sound) }}</p>
        <AudioPlayer :src="props.address(sound)" :download="props.address(sound, true)" :name="sound.name" :size="sizeText(sound.byteCount)" />
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { languageOf } from "../domain/document-view";
import { sizeText } from "../domain/sizes";

import type { ArtifactAttachment } from "../../shared/types";

interface Props {
  // The sounds heard at the head, in the order they stand.
  sounds: readonly ArtifactAttachment[];
  // Where one of the document's files is served, and where it is taken to be saved.
  address: (attachment: ArtifactAttachment, download?: boolean) => string;
}

const props = defineProps<Props>();
</script>
