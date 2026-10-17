<template>
  <!-- A document's pictures opened whole (docs/archive.md, "Pictures open whole"). Opened on any
       picture the document lets open, it walks every such picture the document draws, in the order
       they stand, and its head carries the picture's properties and the way to open it in a tab of
       its own. -->
  <ImageViewer
    v-if="picture !== null"
    v-model:open="open"
    :src="picture.src"
    :title="title"
    :can-previous="index > 0"
    :can-next="index < pictures.length - 1"
    @previous="step(-1)"
    @next="step(1)"
    @loaded="drawn = $event"
  >
    <template #actions>
      <ActionGroup :actions="actions" :properties="documentPictureProperties(t, picture, drawn)" :properties-title="picture.caption || props.title" :narrow-fold="false" />
    </template>
  </ImageViewer>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";
import { useI18n } from "vue-i18n";

import { documentPictureProperties } from "../domain/document-pictures";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Size } from "bmo-ui/app/domain/zoom";
import type { DocumentPicture, PictureRun } from "../../shared/document-pictures";

interface Props {
  // The document's title, which the viewer's head carries with where the picture stands among its
  // pictures.
  title: string;
}

const props = defineProps<Props>();

const { t } = useI18n();

const open = ref(false);
const pictures = ref<DocumentPicture[]>([]);
const index = ref(0);
// The size the browser read off the picture standing, which its properties name.
const drawn = ref<Size | null>(null);

const picture = computed<DocumentPicture | null>(() => pictures.value[index.value] ?? null);
const title = computed(() => `${props.title} · ${countText(index.value + 1)}/${countText(pictures.value.length)}`);

watch(picture, () => {
  drawn.value = null;
});

// The pictures either side are fetched ahead, so going to one shows it at once.
watch(index, (at) => {
  for (const near of [pictures.value[at - 1], pictures.value[at + 1]]) {
    if (near !== undefined) new Image().src = near.src;
  }
});

// A picture written into the document as data has no address a tab could open.
const actions = computed<BarAction[]>(() =>
  picture.value === null || picture.value.address === null
    ? []
    : [{ id: "open", label: t("archive.action.open_new_tab"), icon: "i-material-symbols-open-in-new", onSelect: openPicture }]
);

function openPicture(): void {
  if (picture.value !== null) window.open(picture.value.src, "_blank", "noopener,noreferrer");
}

function step(by: -1 | 1): void {
  const next = index.value + by;

  if (next >= 0 && next < pictures.value.length) index.value = next;
}

function show(run: PictureRun): void {
  pictures.value = run.pictures;
  index.value = run.at;
  open.value = true;
}

defineExpose({ show });
</script>
