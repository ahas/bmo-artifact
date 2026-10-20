<template>
  <!-- The files a document keeps beside it, after its body, in the order they were attached. Each
       is shown the way it is read — a picture inline, a sound in the console's own player that
       carries its name, size and download, a film in a player that plays where it stands, a text as
       the start of it, a page as a page of its own opened in a tab — and every one can be saved. -->
  <section :aria-label="$t('artifact.label.attachments')" class="flex flex-col gap-4">
    <h2 class="text-xl font-semibold text-highlighted">{{ $t("artifact.label.attachments") }} <span class="text-base font-normal text-muted tabular-nums">{{ countText(props.attachments.length) }}</span></h2>

    <ul class="flex flex-col gap-6">
      <li v-for="attachment in props.attachments" :key="attachment.id" class="flex min-w-0 flex-col gap-2">
        <AudioPlayer
          v-if="attachment.kind === 'audio'"
          :src="props.address(attachment)"
          :download="props.address(attachment, true)"
          :name="attachment.name"
          :size="sizeText(attachment.byteCount)"
        />
        <div v-else class="flex min-w-0 items-center gap-2">
          <UIcon :name="KIND_ICONS[attachment.kind] ?? KIND_ICONS.file" class="size-5 shrink-0 text-muted" />
          <span class="min-w-0 grow truncate text-sm font-medium text-highlighted" :title="attachment.name">{{ attachment.name }}</span>
          <span class="shrink-0 text-sm text-muted tabular-nums">{{ sizeText(attachment.byteCount) }}</span>
          <UTooltip v-if="attachment.kind === 'page'" :text="$t('artifact.action.open_page')">
            <UButton
              :to="props.address(attachment)"
              external
              target="_blank"
              rel="noopener noreferrer"
              size="sm"
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-open-in-new"
              :aria-label="$t('artifact.action.open_page')"
            />
          </UTooltip>
          <UTooltip :text="$t('artifact.action.download')">
            <UButton
              :to="props.address(attachment, true)"
              external
              download
              size="sm"
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-download"
              :aria-label="$t('artifact.action.download')"
            />
          </UTooltip>
        </div>

        <img
          v-if="attachment.kind === 'image'"
          :src="props.address(attachment)"
          :alt="attachment.name"
          loading="lazy"
          class="mx-auto max-h-[36rem] max-w-full rounded-lg border border-default bg-white text-neutral-900 object-contain"
        >
        <LoadingVideo v-else-if="attachment.kind === 'video'" :src="props.address(attachment)" class="max-w-full" />
        <ArtifactTextPreview v-else-if="attachment.kind === 'text'" :src="props.address(attachment)" />
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import { sizeText } from "../domain/sizes";

import type { ArtifactAttachment } from "../../shared/types";

interface Props {
  attachments: readonly ArtifactAttachment[];
  // Where one of them is served, and where it is taken to be saved.
  address: (attachment: ArtifactAttachment, download?: boolean) => string;
}

const props = defineProps<Props>();

// The icon each kind of file is marked with.
const KIND_ICONS: Record<string, string> = {
  image: "i-material-symbols-image-outline",
  audio: "i-material-symbols-music-note",
  video: "i-material-symbols-movie-outline",
  text: "i-material-symbols-description-outline",
  page: "i-material-symbols-web",
  file: "i-material-symbols-draft-outline"
};
</script>
