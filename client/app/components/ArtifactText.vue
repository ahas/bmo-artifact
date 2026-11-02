<template>
  <!-- The finding's markdown as it was answered, until the reading is in the browser: this is what
       the server draws and what the browser draws on its first pass, so the two agree and the place
       is never empty. The HTML replaces it once the reading has arrived, and only ever after
       cleaning. -->
  <div v-if="html === null" v-bind="$attrs" class="break-words whitespace-pre-wrap">{{ props.markdown }}</div>
  <div
    v-else
    ref="shown"
    v-bind="$attrs"
    class="markdown markdown--finding"
    @click="onPress"
    @pointerover="onPointerOver"
    @pointerout="onPointerOut"
    v-html="html"
  />

  <!-- The blocks markdown has no form for, each drawn with its component into the place the reading
       left for it in the HTML above. They are teleported rather than mounted apart so they stand
       inside this screen's own component tree and take what it provides. -->
  <Teleport v-for="block in blocks" :key="block.key" :to="block.place">
    <ArtifactFold v-if="block.block.kind === 'fold'" :label="block.block.label" :html="block.block.html" />
    <ArtifactGallery v-else-if="block.block.kind === 'gallery'" :slides="block.block.slides" />
    <ArtifactMap v-else :query="block.block.query" />
  </Teleport>

  <!-- The source a footnote mark cites, brought forward (docs/archive.md, "How a document is drawn"): over the mark
       in a popover where the pointer rests on it, and in a dialog centred on the screen where a finger taps it. -->
  <UPopover v-model:open="sourceOpen" :reference="sourceMark ?? undefined" :content="SOURCE_PLACE" arrow>
    <template #content>
      <div v-if="source !== null" class="max-w-[min(40rem,calc(100vw-1rem))] p-3" data-mark-source @pointerenter="holdSource" @pointerleave="leaveSource">
        <ArtifactMarkSource :source="source" />
      </div>
    </template>
  </UPopover>

  <UModal v-model:open="dialogOpen" :title="$t('artifact.title.source', { mark: source?.mark ?? '' })" :ui="{ content: 'max-w-sm' }">
    <template #content>
      <div v-if="source !== null" data-mark-dialog>
        <div class="p-4">
          <ArtifactMarkSource :source="source" />
        </div>
        <div class="border-t border-default p-2">
          <UButton block color="neutral" variant="ghost" :label="$t('common.action.close')" @click="dialogOpen = false" />
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { colourCode, forgetCode } from "bmo-ui/app/domain/code-colour";
import { copyFromPress } from "bmo-ui/app/domain/code-copy";
import { drawDiagrams } from "bmo-ui/app/domain/diagram";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";
import { useDarkScheme } from "bmo-ui/app/domain/scheme";
import { useI18n } from "vue-i18n";

import { copyText } from "../domain/copy";
import { backMarkAt, citeId, entryId, markAt, sourceOf } from "../domain/footnotes";
import { FINDING_BLOCK_PLACES, findingBlock } from "../domain/finding-blocks";
import { findingReading, loadFindingReading } from "../domain/finding-load";

import type { FindingBlock } from "../domain/finding-blocks";
import type { ImageSource } from "../domain/finding-markdown";
import type { MarkSource } from "../domain/footnotes";

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

// A footnote mark brings its source forward in a popover over it, above where there is room. A pointer
// resting on the mark opens it and it stays while the pointer is on the mark or in the popover; a press
// opens it without following the mark's link. A finger has no pointer to rest, so a tap opens the source in a
// dialog centred on the screen instead, and follows no link either: the text never moves under the reader.
const SOURCE_PLACE = {
  side: "top",
  sideOffset: 4,
  collisionPadding: 8,
  // Brought up by the pointer, it takes no focus, and gives none back to a trigger it does not have; brought
  // up by a key, the focus goes into it and comes back to the mark.
  onOpenAutoFocus: (event: Event) => {
    if (!openedByKey) event.preventDefault();
  },
  onCloseAutoFocus: (event: Event) => {
    event.preventDefault();

    if (openedByKey) sourceMark.value?.focus({ preventScroll: true });
  }
} as const;

// How long a pointer rests on a mark before its source comes up, and how long the source stays once the
// pointer has left both, so a pointer crossing the gap between the two never loses it.
const HOVER_OPEN_MS = 100;
const HOVER_CLOSE_MS = 200;

const sourceOpen = ref(false);
const dialogOpen = ref(false);
const sourceMark = shallowRef<HTMLAnchorElement | null>(null);
const source = shallowRef<MarkSource | null>(null);
let openedByKey = false;
let hoverTimer: ReturnType<typeof setTimeout> | undefined;

function stopHoverTimer(): void {
  clearTimeout(hoverTimer);
  hoverTimer = undefined;
}

// The source of a mark brought forward, over the mark or in the dialog; a mark whose text carries no entry for it
// opens nothing.
function showSource(mark: HTMLAnchorElement, byKey: boolean, inDialog = false): void {
  const name = mark.dataset["mark"] ?? "";
  const found = shown.value === null ? null : sourceOf(shown.value, name, mark.textContent?.trim() ?? name);

  stopHoverTimer();

  if (found === null) return;

  openedByKey = byKey;
  sourceMark.value = mark;
  source.value = found;

  if (inDialog) {
    sourceOpen.value = false;
    dialogOpen.value = true;
  } else {
    sourceOpen.value = true;
  }
}

function closeSource(): void {
  stopHoverTimer();
  sourceOpen.value = false;
}

function hovering(event: PointerEvent): boolean {
  return event.pointerType === "mouse" || event.pointerType === "pen";
}

function onPointerOver(event: PointerEvent): void {
  const mark = shown.value === null || !hovering(event) ? null : markAt(event.target, shown.value);

  if (mark === null) return;

  stopHoverTimer();

  if (sourceOpen.value && sourceMark.value === mark) return;

  hoverTimer = setTimeout(() => showSource(mark, false), HOVER_OPEN_MS);
}

function onPointerOut(event: PointerEvent): void {
  const mark = shown.value === null || !hovering(event) ? null : markAt(event.target, shown.value);

  if (mark === null || (event.relatedTarget instanceof Node && mark.contains(event.relatedTarget))) return;

  leaveSource(event);
}

function holdSource(): void {
  stopHoverTimer();
}

function leaveSource(event: PointerEvent): void {
  if (!hovering(event)) return;

  stopHoverTimer();

  if (sourceOpen.value && !openedByKey) hoverTimer = setTimeout(closeSource, HOVER_CLOSE_MS);
}

// A tap on a mark opens its source in the dialog, and a key opens it over the mark. A click scrolls the text to the
// mark's entry and marks it there for a moment, as an entry's mark leads back to the text. None of them follows the
// mark's link, so the address never changes. A press on an entry's mark scrolls the text back to the mark's first place and marks it there
// for a moment. Anything else is the copy buttons'.
const FOUND_MS = 2000;

// The place a mark leads to, scrolled to the middle of the text and marked there for a moment.
function bringForward(place: HTMLElement | null): void {
  if (place === null) return;

  scrollWithin(place, "center");
  place.dataset["markFound"] = "";
  setTimeout(() => delete place.dataset["markFound"], FOUND_MS);
}

function onPress(event: MouseEvent): void {
  const root = shown.value;
  const mark = root === null ? null : markAt(event.target, root);

  if (mark !== null) {
    event.preventDefault();

    const touched = (event as PointerEvent).pointerType === "touch";

    if (touched || event.detail === 0) {
      showSource(mark, event.detail === 0, touched);

      return;
    }

    closeSource();
    bringForward(root!.querySelector<HTMLElement>(`#${CSS.escape(entryId(mark.dataset["mark"] ?? ""))}`));

    return;
  }

  const back = root === null ? null : backMarkAt(event.target, root);

  if (back !== null) {
    event.preventDefault();

    bringForward(root!.querySelector<HTMLElement>(`#${CSS.escape(citeId(back.dataset["markBack"] ?? ""))}`));

    return;
  }

  copyFromPress(t, event, copyText);
}

// A press outside the popover puts it away, but a press on the mark it stands over is the mark's own, and
// so is the focus that press gives the mark. The library asks on the pressed element, on the way down.
const OUTSIDE = ["dismissableLayer.pointerDownOutside", "dismissableLayer.focusOutside"];

function keepForMark(event: Event): void {
  const detail = (event as CustomEvent<{ originalEvent: Event }>).detail;
  const target = detail?.originalEvent?.target ?? event.target;

  if (sourceMark.value !== null && target instanceof Node && sourceMark.value.contains(target)) event.preventDefault();
}

watch(shown, (element, before) => {
  for (const name of OUTSIDE) {
    before?.removeEventListener(name, keepForMark, true);
    element?.addEventListener(name, keepForMark, true);
  }
});

// The code blocks of the text being replaced, or leaving with this component, are let go of too, and so
// is the source of a mark the new text no longer holds.
watch(
  html,
  () => {
    blocks.value = [];
    closeSource();

    if (shown.value !== null) {
      forgetCode(shown.value);
    }
  },
  { flush: "pre" }
);

onBeforeUnmount(() => {
  stopHoverTimer();

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
