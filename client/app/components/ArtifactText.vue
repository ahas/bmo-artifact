<template>
  <!-- The finding's markdown as it was answered, until the reading is in the browser: this is what
       the server draws and what the browser draws on its first pass, so the two agree and the place
       is never empty. The HTML replaces it once the reading has arrived, and only ever after
       cleaning. -->
  <div v-if="html === null" v-bind="$attrs" class="break-words whitespace-pre-wrap">{{ props.markdown }}</div>
  <div v-else ref="shown" v-bind="$attrs" class="markdown markdown--finding" @click="copyFromPress(t, $event, copyText)" v-html="html" />

  <!-- The blocks markdown has no form for, each drawn with its component into the place the reading
       left for it in the HTML above. They are teleported rather than mounted apart so they stand
       inside this screen's own component tree and take what it provides. -->
  <Teleport v-for="block in blocks" :key="block.key" :to="block.place">
    <ArtifactFold v-if="block.block.kind === 'fold'" :label="block.block.label" :html="block.block.html" />
    <ArtifactGallery v-else-if="block.block.kind === 'gallery'" :slides="block.block.slides" />
    <ArtifactMap v-else :query="block.block.query" />
  </Teleport>
</template>

<script setup lang="ts">
import { colourCode, forgetCode } from "bmo-ui/app/domain/code-colour";
import { copyFromPress } from "bmo-ui/app/domain/code-copy";
import { drawDiagrams } from "bmo-ui/app/domain/diagram";
import { useDarkScheme } from "bmo-ui/app/domain/scheme";
import { useI18n } from "vue-i18n";

import { copyText } from "../domain/copy";
import { FINDING_BLOCK_PLACES, findingBlock } from "../domain/finding-blocks";
import { findingReading, loadFindingReading } from "../domain/finding-load";

import type { FindingBlock } from "../domain/finding-blocks";
import type { ImageSource } from "../domain/finding-markdown";

// One finding's text, standing as the formatting it was written for (`docs/layout.md`, "The console
// plugin bundle"). The read API has already turned its structure lines into markdown; how that
// markdown is read is the finding reading's (see domain/finding-markdown).

interface Props {
  markdown: string;
  // Where the finding's pictures are shown from.
  images?: ImageSource;
  // Where the files of the document the finding belongs to are served, for a picture or a link that
  // names one by its name.
  files?: ImageSource;
  // The mark each heading takes, in the order they stand, so the contents beside the text can steer
  // to one.
  slugs?: readonly string[];
}

// The text stands beside the blocks teleported into it, so what a caller puts on this component is
// put on the text itself rather than left to fall through a root there is not one of.
defineOptions({ inheritAttrs: false });

const props = defineProps<Props>();

// The formatted text has been put on the page, with the element that holds it.
const emit = defineEmits<{ rendered: [element: HTMLElement] }>();
const shown = useTemplateRef<HTMLElement>("shown");

// A diagram in the text is drawn, and a code block coloured, in the theme the text stands in.
const dark = useDarkScheme();

// The reading when it had already arrived before this text was drawn — except while the page is
// being hydrated, where the server drew the plain text and the browser has to draw the same.
const { t } = useI18n();
const reading = shallowRef(useNuxtApp().isHydrating ? null : findingReading());

const html = computed<string | null>(() =>
  reading.value === null ? null : reading.value.renderFinding(t, props.markdown, { images: props.images, files: props.files, slugs: props.slugs })
);

watch(
  html,
  async (value) => {
    if (value === null) {
      return;
    }

    await nextTick();

    if (shown.value !== null) {
      fillBlocks(shown.value);
      emit("rendered", shown.value);
      void drawDiagrams(shown.value, dark.value);
      colourCode(shown.value, dark.value);
    }
  },
  // A text mounted once the reading is already in the browser has its HTML from the start, so the
  // first one is settled too.
  { flush: "post", immediate: true }
);

// The blocks — a fold, a gallery, a map — stand in the text as places the reading left, each filled
// with its component once the text is on the page: what the reading wrote into a place is read off it
// and cleared, and the component is teleported in. The places live in the text's own HTML, which the
// next rendering replaces whole, so the blocks are let go before that happens.
const blocks = shallowRef<{ key: number; place: HTMLElement; block: FindingBlock }[]>([]);

function fillBlocks(root: HTMLElement): void {
  blocks.value = [...root.querySelectorAll<HTMLElement>(FINDING_BLOCK_PLACES)].flatMap((place, key) => {
    const block = findingBlock(place);

    if (block === null) {
      return [];
    }

    place.replaceChildren();

    return [{ key, place, block }];
  });
}

// The code blocks of the text being replaced, or leaving with this component, are let go of too.
watch(
  html,
  () => {
    blocks.value = [];

    if (shown.value !== null) {
      forgetCode(shown.value);
    }
  },
  { flush: "pre" }
);

onBeforeUnmount(() => {
  if (shown.value !== null) {
    forgetCode(shown.value);
  }
});

// A diagram in the text is drawn, and a code block coloured, again when its theme switches.
watch(dark, (inDark) => {
  if (shown.value !== null) {
    void drawDiagrams(shown.value, inDark);
    colourCode(shown.value, inDark);
  }
});

onMounted(async () => {
  if (reading.value === null) {
    reading.value = await loadFindingReading();
  }
});
</script>
