<template>
  <!-- The pictures a finding wrote together, one at a time: the picture at a fixed height so the text
       under it never moves as the slides change, its caption under it, and a row stepping through
       them with where in the run the reader stands. A swipe moves it on as well. -->
  <div class="flex flex-col gap-2">
    <UCarousel ref="carousel" v-slot="{ item, index }" :items="props.slides" @select="current = $event">
      <figure class="flex flex-col items-center gap-2">
        <div class="relative h-64 w-full overflow-hidden rounded-md border border-default bg-elevated sm:h-80" :aria-busy="shown(item, index) && !loaded.includes(index)">
          <SkeletonStatus v-if="shown(item, index) && !loaded.includes(index)" class="absolute inset-0 flex" :label="$t('common.status.picture_loading')">
            <SkeletonBar class="size-full" />
          </SkeletonStatus>

          <img
            v-if="shown(item, index)"
            :src="item.src ?? ''"
            :alt="item.caption"
            :data-picture-address="webPicture(item.href)"
            loading="lazy"
            class="size-full bg-white object-contain"
            @load="loaded.push(index)"
            @error="failed.push(index)"
          >

          <div v-else class="flex size-full items-center justify-center">
            <ULink :to="item.href" target="_blank" rel="noopener noreferrer" class="text-sm">{{ $t("artifact.action.original") }}</ULink>
          </div>
        </div>

        <figcaption v-if="item.caption !== ''" class="text-center text-sm text-muted">{{ item.caption }}</figcaption>
      </figure>
    </UCarousel>

    <div class="flex items-center justify-center gap-2">
      <UTooltip :text="$t('common.action.previous_picture')">
        <UButton icon="i-material-symbols-chevron-left" color="neutral" variant="ghost" :aria-label="$t('common.action.previous_picture')" :disabled="current === 0" @click="step(-1)" />
      </UTooltip>

      <span class="text-sm text-muted tabular-nums">{{ countText(current + 1) }} / {{ countText(props.slides.length) }}</span>

      <UTooltip :text="$t('common.action.next_picture')">
        <UButton icon="i-material-symbols-chevron-right" color="neutral" variant="ghost" :aria-label="$t('common.action.next_picture')" :disabled="current === props.slides.length - 1" @click="step(1)" />
      </UTooltip>
    </div>
  </div>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import { webPicture } from "../domain/document-pictures";

import type { GallerySlide } from "../domain/finding-markdown";

// A gallery of a finding's text (see domain/finding-markdown): the reading names the slides, and this
// puts them on the screen where it left their place.
const props = defineProps<{ slides: GallerySlide[] }>();

const carousel = useTemplateRef("carousel");
const current = ref(0);
const loaded = ref<number[]>([]);
const failed = ref<number[]>([]);

// A slide shows its picture where the console has a copy of it that loaded; otherwise it stands as a
// link to the address the search named.
function shown(slide: GallerySlide, index: number): boolean {
  return slide.src !== null && !failed.value.includes(index);
}

function step(by: -1 | 1): void {
  const api = carousel.value?.emblaApi;

  if (by < 0) api?.scrollPrev();
  else api?.scrollNext();
}
</script>
