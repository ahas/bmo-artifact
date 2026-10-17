<template>
  <!-- What a document's text stands on: where each finding is, the pages each finding cites, the
       pages opened and cited nowhere, and the addresses the text mentions without opening them. -->
  <aside :aria-label="$t('archive.label.sources')" class="flex min-w-0 flex-col gap-4 py-4">
    <section v-if="props.contents.length > 1" class="flex flex-col gap-1">
      <h2 class="px-4 text-sm font-semibold text-muted">{{ $t("common.label.toc") }}</h2>
      <DocumentToc class="px-2" :headings="props.contents" :active="null" :label="$t('common.label.toc')" @select="emit('go', $event)" />
    </section>

    <p v-if="props.pageCount === 0" class="px-4 text-sm text-muted">{{ $t("archive.status.no_sources") }}</p>

    <section v-for="group in props.citedGroups" :key="group.key" class="flex flex-col gap-1">
      <h2 class="line-clamp-2 px-4 text-sm font-semibold text-muted">{{ group.title }}</h2>
      <ul class="flex flex-col px-2">
        <ArchiveSourceItem
          v-for="source in group.sources"
          :key="source.ref"
          :ref-base="props.refBase"
          :ref-name="source.ref"
          :url="source.url"
          :recorded="source.title"
          :note="source.supports"
          :mark="source.mark"
          :active="source.mark !== null && source.mark === props.activeMark"
        />
      </ul>
    </section>

    <section v-if="props.opened.length > 0" class="flex flex-col gap-1">
      <h2 class="px-4 text-sm font-semibold text-muted">{{ $t("archive.title.opened_pages", { n: countText(props.opened.length) }) }}</h2>
      <ul class="flex flex-col px-2">
        <ArchiveSourceItem
          v-for="source in props.opened"
          :key="source.ref"
          :ref-base="props.refBase"
          :ref-name="source.ref"
          :url="source.url"
          :recorded="source.title"
          :note="source.supports"
          :mark="source.mark"
          :active="source.mark !== null && source.mark === props.activeMark"
        />
      </ul>
    </section>

    <!-- Addresses the text mentions that the search never opened. They are not evidence, so they
         stand apart from the sources. -->
    <section v-if="props.bodyLinks.length > 0" class="flex flex-col gap-1">
      <h2 class="px-4 text-sm font-semibold text-muted">{{ $t("archive.title.body_links", { n: countText(props.bodyLinks.length) }) }}</h2>
      <ul class="flex flex-col px-2">
        <ArchiveSourceItem
          v-for="link in props.bodyLinks"
          :key="link.ref"
          :ref-base="props.refBase"
          :ref-name="link.ref"
          :url="link.url"
          :note="link.label"
        />
      </ul>
    </section>
  </aside>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";

import type { SourceGroup } from "../domain/archive";
import type { SearchLinkView, SearchSourceView } from "../../shared/types";

interface Props {
  // The road this document's refs are answered under. It names a document on the console's own
  // screen and a token on a share page, and nothing here knows which of the two it was given.
  refBase: string;
  contents: readonly { slug: string; text: string; depth: number }[];
  citedGroups: readonly SourceGroup[];
  opened: readonly SearchSourceView[];
  bodyLinks: readonly SearchLinkView[];
  pageCount: number;
  // The footnote mark last pressed in the text.
  activeMark?: string | null;
}

const props = withDefaults(defineProps<Props>(), { activeMark: null });
const emit = defineEmits<{ go: [place: string] }>();
</script>
