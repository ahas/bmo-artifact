<template>
  <!-- One page a document stands on: its picture, what it is called, and what it was cited for. It
       is a link out of this console, so it opens in a tab of its own that gets no hold on this
       one. It stands with the site's name and the citation from the start, and fills in what the
       page says of itself once the server has read it. -->
  <li ref="item" :data-source-mark="props.mark ?? undefined">
    <a
      :href="props.url"
      target="_blank"
      rel="noopener noreferrer"
      class="group flex min-h-11 items-start gap-3 rounded-md px-2 py-2 transition-colors hover:bg-elevated active:bg-accented focus-inset"
      :class="props.active ? 'bg-primary/10 ring-1 ring-primary ring-inset' : ''"
    >
      <UBadge
        v-if="props.mark !== null"
        :color="props.mark.endsWith('†') ? 'warning' : 'neutral'"
        variant="subtle"
        size="sm"
        class="mt-0.5 shrink-0 tabular-nums"
        :label="props.mark"
        :title="props.mark.endsWith('†') ? $t('archive.label.snippet_only') : undefined"
      />
      <span class="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-default bg-elevated">
        <img
          v-if="picture !== null"
          :key="picture"
          :src="picture"
          alt=""
          loading="lazy"
          class="size-full object-cover"
          @error="fallBack"
        >
        <span v-else aria-hidden="true" class="text-sm font-semibold text-muted">{{ initial }}</span>
      </span>

      <span class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="line-clamp-2 text-sm font-medium break-words text-highlighted group-hover:underline">{{ title }}</span>
        <span class="line-clamp-2 text-sm break-words text-muted">{{ subtitle }}</span>
      </span>

      <span class="sr-only">{{ $t("archive.aria.opens_new_tab") }}</span>
      <UIcon name="i-material-symbols-open-in-new" aria-hidden="true" class="mt-0.5 size-4 shrink-0 text-muted" />
    </a>
  </li>
</template>

<script setup lang="ts">
import { hostInitial, pageSubtitle, pageTitle, refPath } from "../domain/archive";
import { appUrl } from "../domain/urls";

import type { SearchRef, SearchRefPreview } from "../../shared/types";

interface Props {
  // The road this document's refs are answered under, which the page reading it supplies.
  refBase: string;
  refName: SearchRef;
  url: string;
  // What the record calls the page, and what it says under it.
  recorded?: string | null;
  note?: string | null;
  // The footnote the text cites the page by, and whether it is the one last pressed.
  mark?: string | null;
  active?: boolean;
}

const props = withDefaults(defineProps<Props>(), { recorded: null, note: null, mark: null, active: false });

const preview = ref<SearchRefPreview | null>(null);
// Which of the page's pictures is being tried: its own, then its icon, then none.
const stage = ref<"thumb" | "favicon" | "none">("none");

const title = computed(() => pageTitle(props.url, props.recorded, preview.value));
const subtitle = computed(() => pageSubtitle(props.url, props.note));
const initial = computed(() => hostInitial(props.url));

const picture = computed<string | null>(() =>
  stage.value === "none" ? null : appUrl(refPath(props.refBase, props.refName, "image", stage.value))
);

function fallBack(): void {
  stage.value = stage.value === "thumb" && preview.value?.favicon === true ? "favicon" : "none";
}

async function read(): Promise<void> {
  try {
    const answer = await $fetch<SearchRefPreview>(refPath(props.refBase, props.refName, "preview"));

    preview.value = answer;
    stage.value = answer.thumbnail ? "thumb" : answer.favicon ? "favicon" : "none";
  } catch {
    // A page whose preview could not be read keeps its site's name and its citation.
  }
}

// A document can stand on two hundred pages, so a page's preview is asked for only once the item
// comes near the screen.
const item = useTemplateRef<HTMLElement>("item");
let watcher: IntersectionObserver | null = null;

onMounted(() => {
  if (item.value === null) {
    return;
  }

  watcher = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        watcher?.disconnect();
        watcher = null;
        void read();
      }
    },
    { rootMargin: "200px" }
  );
  watcher.observe(item.value);
});

onBeforeUnmount(() => watcher?.disconnect());
</script>
