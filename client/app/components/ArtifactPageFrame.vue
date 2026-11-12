<template>
  <!-- A document that is a page of its own (docs/artifact.md, "An HTML page"): its title on a row
       of its own, and under it the page filling the rest of the screen; the actions on it stand in the
       screen's own row above. The page's own scroll is the only one here. The frame names the scheme
       the console stands in, so the page is drawn in it from its first frame and again when the
       screen mode changes (docs/artifact.md, "How a document is drawn"), and names the console's
       language, which the player the page's sounds stand in speaks. A share page has no header: the
       page fills the window. The reader draws no band over a page: its sounds stand where the page
       writes them (docs/artifact.md, "An HTML page"). -->
  <div class="flex min-h-0 flex-1 flex-col">
    <header v-if="props.chrome" class="shrink-0 border-b border-default px-4 py-3">
      <!-- The title wraps between words onto two lines at most; a longer one is clamped there and the console's
           tooltip for a cut line shows it whole (docs/artifact.md, 「Where a person reads it」). -->
      <h1 class="line-clamp-2 text-lg font-semibold break-keep text-highlighted [overflow-wrap:anywhere] sm:text-xl" data-page-title>{{ props.title }}</h1>
    </header>

    <ArtifactHostedFrame :src="framed" :title="props.title" />
  </div>
</template>

<script setup lang="ts">
import { DOCUMENT_PALETTE } from "../../shared/document-design";
import { useFramedPage } from "../domain/hosted-page";

import type { PageServed } from "../domain/hosted-page";

interface Props {
  title: string;
  page: PageServed;
  // Whether the page stands under its title, or fills the window alone.
  chrome?: boolean;
}

const props = withDefaults(defineProps<Props>(), { chrome: true });

useHead({ style: [{ key: "document-palette", innerHTML: DOCUMENT_PALETTE }] });

const framed = useFramedPage(() => props.page);
</script>
