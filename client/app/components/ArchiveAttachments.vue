<template>
  <!-- The files a document keeps beside it, after its body, in the order they were attached. Each
       is shown the way it is read — a picture inline, a sound in the console's own player that
       carries its name, size and download, a film in a player that plays where it stands, a text as
       the start of it, a page as a page of its own opened in a tab — and every one can be saved. -->
  <section :aria-label="$t('archive.label.attachments')" class="flex flex-col gap-4">
    <h2 class="text-xl font-semibold text-highlighted">{{ $t("archive.label.attachments") }} <span class="text-base font-normal text-muted tabular-nums">{{ countText(props.attachments.length) }}</span></h2>

    <ul class="flex flex-col gap-6">
      <li v-for="attachment in props.attachments" :key="attachment.id" class="flex min-w-0 flex-col gap-2">
        <AudioPlayer
          v-if="attachment.media === 'audio'"
          :src="fileUrl(attachment)"
          :download="fileUrl(attachment, true)"
          :name="attachment.name"
          :size="sizeText(attachment.bytes)"
        />
        <div v-else class="flex min-w-0 items-center gap-2">
          <UIcon :name="MEDIA_ICONS[attachment.media]" class="size-5 shrink-0 text-muted" />
          <span class="min-w-0 grow truncate text-sm font-medium text-highlighted" :title="attachment.name">{{ attachment.name }}</span>
          <span class="shrink-0 text-sm text-muted tabular-nums">{{ sizeText(attachment.bytes) }}</span>
          <UTooltip v-if="attachment.media === 'page'" :text="$t('archive.action.open_page')">
            <UButton
              :to="fileUrl(attachment)"
              external
              target="_blank"
              rel="noopener noreferrer"
              size="sm"
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-open-in-new"
              :aria-label="$t('archive.action.open_page')"
            />
          </UTooltip>
          <UTooltip :text="$t('archive.action.download')">
            <UButton
              :to="fileUrl(attachment, true)"
              external
              download
              size="sm"
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-download"
              :aria-label="$t('archive.action.download')"
            />
          </UTooltip>
        </div>

        <img
          v-if="attachment.media === 'image'"
          :src="fileUrl(attachment)"
          :alt="attachment.name"
          loading="lazy"
          class="mx-auto max-h-[36rem] max-w-full rounded-lg border border-default bg-white text-neutral-900 object-contain"
        >
        <LoadingVideo v-else-if="attachment.media === 'video'" :src="fileUrl(attachment)" class="max-w-full" />
        <ArchiveTextPreview v-else-if="attachment.media === 'text'" :src="fileUrl(attachment)" />
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import { attachmentUrl } from "../domain/archive";
import { sizeText } from "../domain/sizes";

import type { ArchiveAttachmentView } from "../../shared/types";

interface Props {
  attachments: readonly ArchiveAttachmentView[];
  // The road this document's attachments are answered under: the console names its document
  // there, a share page names its token, and nothing here knows which of the two it is.
  base: string;
}

const props = defineProps<Props>();

const MEDIA_ICONS: Record<ArchiveAttachmentView["media"], string> = {
  image: "i-material-symbols-image-outline",
  audio: "i-material-symbols-music-note",
  video: "i-material-symbols-movie-outline",
  text: "i-material-symbols-description-outline",
  page: "i-material-symbols-web",
  file: "i-material-symbols-draft-outline"
};

function fileUrl(attachment: ArchiveAttachmentView, download = false): string {
  return attachmentUrl(props.base, attachment, download);
}
</script>
