<template>
  <!-- One page a document refers to: its picture, what it is called, and what it says of itself. It is
       a link out of this console, so it opens in a tab of its own that gets no hold on this one. It
       stands with what the service kept of it from the start, and fills in its preview once the
       service has read it. -->
  <li ref="item">
    <a
      :href="reference.url"
      target="_blank"
      rel="noopener noreferrer"
      class="group flex min-h-11 items-start gap-3 rounded-md px-2 py-2 transition-colors hover:bg-elevated active:bg-accented focus-inset"
    >
      <span class="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-default bg-elevated">
        <img v-if="pictured" :src="props.image" alt="" loading="lazy" class="size-full object-cover" @error="pictured = false">
        <span v-else aria-hidden="true" class="text-sm font-semibold text-muted">{{ hostInitial(reference.url) }}</span>
      </span>

      <span class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="line-clamp-2 text-sm font-medium break-words text-highlighted group-hover:underline">{{ reference.title ?? hostOf(reference.url) }}</span>
        <span class="line-clamp-2 text-sm break-words text-muted">{{ reference.preview ?? hostOf(reference.url) }}</span>
      </span>

      <span class="sr-only">{{ $t("artifact.aria.opens_new_tab") }}</span>
      <UIcon name="i-material-symbols-open-in-new" aria-hidden="true" class="mt-0.5 size-4 shrink-0 text-muted" />
    </a>
  </li>
</template>

<script setup lang="ts">
import { hostInitial, hostOf } from "../domain/document-view";

import type { ArtifactReference } from "../../shared/types";

interface Props {
  reference: ArtifactReference;
  // Where the page's preview picture is served.
  image: string;
  // Reads the reference with its preview, where the list kept none of it.
  read: (reference: ArtifactReference) => Promise<ArtifactReference>;
}

const props = defineProps<Props>();

const reference = ref<ArtifactReference>(props.reference);
// The picture is tried once the preview is read, and dropped where it does not load.
const pictured = ref(false);

async function read(): Promise<void> {
  try {
    reference.value = props.reference.preview === null ? await props.read(props.reference) : props.reference;
    pictured.value = true;
  } catch {
    // A page whose preview could not be read keeps what the document called it.
  }
}

// A document can refer to two hundred pages, so a page's preview is asked for only once the item
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
