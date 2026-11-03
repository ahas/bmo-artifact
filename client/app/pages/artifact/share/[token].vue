<template>
  <!-- One document read by somebody holding its link. It is the body, the files it keeps and the pages
       it refers to, and nothing else: no sidebar, no title bar, and no control that leads anywhere in
       this console. On a phone the side rises from the bottom edge over the text. A document that is a
       page of its own is that page filling the window, its sounds standing where the page writes them. -->
  <ScreenFrame class="artifact-paper">
    <div v-if="failed !== ''" role="alert" class="flex min-h-0 flex-1 items-center justify-center p-4">
      <p class="text-sm text-error">{{ failed }}</p>
    </div>

    <ContentSkeleton v-else-if="held === null" class="p-4" :label="$t('artifact.status.document_loading')">
      <SkeletonText :lines="8" size="base" lastWidth="w-2/5" />
    </ContentSkeleton>

    <ArtifactPageFrame
      v-else-if="held.kind?.hosted === true"
      :title="held.document.title"
      :page="(asked) => sharedContentAddress(token, asked)"
      :chrome="false"
    />

    <div v-else-if="narrow" class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 overflow-y-auto">
        <ArtifactBody v-bind="bodyProps" :references="() => { sideOpen = true; }" />
      </div>

      <UDrawer v-model:open="sideOpen" :title="$t('artifact.title.references_count', { n: countText(held.references.length) })" :ui="FULL_SHEET">
        <template #body>
          <div class="min-h-0 flex-1 overflow-y-auto">
            <ArtifactReferences v-bind="sideProps" @go="goTo" />
          </div>
        </template>
      </UDrawer>
    </div>

    <USplitter v-else id="artifact-share-splitter" auto-save-id="bmo-artifact-share-splitter" :items="SIDES" :ui="{ handle: SPLIT_HANDLE }" class="min-h-0 flex-1">
      <template #body>
        <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
          <ArtifactBody v-bind="bodyProps" />
        </div>
      </template>

      <template #side>
        <div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
          <ArtifactReferences v-bind="sideProps" @go="goTo" />
        </div>
      </template>
    </USplitter>
  </ScreenFrame>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { useI18n } from "vue-i18n";

import { DOCUMENT_PALETTE } from "../../../../shared/document-design";
import { SHARE_ROBOTS, previewDescription, shareHeadTags } from "../../../../shared/share-head";
import { contentsOf } from "../../../domain/document-view";
import {
  linkEnded,
  readSharedAttachments,
  readSharedContent,
  readSharedDocument,
  readSharedKind,
  readSharedLink,
  readSharedReference,
  readSharedReferences,
  sharedAttachmentAddress,
  sharedContentAddress,
  sharedPictureAddress,
  sharedReferenceImageAddress
} from "../../../domain/shared-document";

import type { SplitterItem } from "@nuxt/ui";
import type { ArtifactAttachment, ArtifactDocument, ArtifactKind, ArtifactReference } from "../../../../shared/types";

// What a share link opens. Every read the page makes carries its token and reads the one document the link
// opens: the document, its kind, its file, its files and the pages it refers to. They are the share reads the
// manifest names, which whoever serves the screen relays to a reader with no sign-in (`docs/api.md`, "Share
// links"; bmo-ui's `docs/console-bundles.md`, "Share pages").

definePageMeta({ layout: false });

const { t } = useI18n();
const SIDES: SplitterItem[] = [
  { slot: "body", minSize: 40, defaultSize: 72 },
  { slot: "side", minSize: 16, defaultSize: 28 }
];

const route = useRoute();
const narrow = useNarrow("(max-width: 1023px)");
const sideOpen = ref(false);
const token = computed(() => String(route.params.token ?? ""));

interface Held {
  document: ArtifactDocument;
  kind: ArtifactKind | null;
  markdown: string;
  attachments: ArtifactAttachment[];
  references: ArtifactReference[];
}

const held = ref<Held | null>(null);
const failed = ref("");
let generation = 0;

onScopeDispose(() => {
  generation++;
});

// A kind's name in the pack's word for it, and as written where the pack has none.
function kindWordOf(name: string): string {
  const key = `artifact.kind.${name}`;
  const said = t(key);

  return said === key ? name : said;
}

const kindText = computed(() => kindWordOf(held.value?.document.kind ?? ""));
const contents = computed(() => contentsOf(held.value?.markdown ?? "", held.value?.kind?.contentsDepth ?? 2));

const bodyProps = computed(() => ({
  title: held.value?.document.title ?? "",
  kind: kindText.value,
  markdown: held.value?.markdown ?? "",
  slugs: contents.value.slugs,
  attachments: held.value?.attachments ?? [],
  address: (attachment: ArtifactAttachment, download = false) => sharedAttachmentAddress(token.value, attachment.id, download)
}));

const sideProps = computed(() => ({
  contents: contents.value.entries,
  references: held.value?.references ?? [],
  image: (reference: ArtifactReference) => sharedReferenceImageAddress(token.value, reference.id),
  read: (reference: ArtifactReference) => readSharedReference(token.value, reference.id)
}));

async function goTo(slug: string): Promise<void> {
  sideOpen.value = false;
  await nextTick();

  const target = document.querySelector(`[data-heading="${CSS.escape(slug)}"]`);

  if (target !== null) scrollWithin(target, "start");
}

// What the link reads as before it is opened is read while the page is rendered on the server, so the
// page goes out with the document's own head and a chat app drawing a preview of the link reads it
// without running anything. A link that opens nothing has no head of its own.
const link = useRequestURL({ xForwardedHost: true });
const { data: head } = await useAsyncData(`artifact-share-head:${token.value}`, async () => {
  try {
    const [shared, document, kind] = await Promise.all([readSharedLink(token.value), readSharedDocument(token.value), readSharedKind(token.value)]);

    // A document that keeps no picture has none to show.
    const image = document.hasPicture ? new URL(sharedPictureAddress(token.value), link.origin).toString() : null;
    // The kind's word leads the description where the page names the kind over the title, as it does for
    // every document but a page.
    const kindWord = kind.hosted ? null : kindWordOf(document.kind);
    const description = previewDescription(document.description, kindWord, (word, said) =>
      t("artifact.preview.described", { kind: word, description: said })
    );

    return {
      title: document.title,
      description,
      image,
      link: shared.url,
      siteName: t("artifact.preview.site_name"),
      locale: t("artifact.preview.locale")
    };
  } catch {
    return null;
  }
});
const tags = computed(() => (head.value === null || head.value === undefined ? null : shareHeadTags(head.value)));

// The page stands on the document paper from its first frame, the skeleton included.
useHead({
  title: computed(() => (tags.value === null ? t("artifact.title.screen") : tags.value.title.slice(0, 60))),
  meta: computed(() => tags.value?.meta ?? [SHARE_ROBOTS]),
  link: computed(() => (tags.value === null ? [] : [{ rel: "canonical", href: tags.value.canonical }])),
  style: [{ key: "document-palette", innerHTML: DOCUMENT_PALETTE }]
});

async function load(): Promise<void> {
  const mine = ++generation;

  failed.value = "";

  try {
    const [document, kind] = await Promise.all([readSharedDocument(token.value), readSharedKind(token.value)]);
    const [markdown, attachments, references] = await Promise.all([
      kind.hosted ? Promise.resolve("") : readSharedContent(token.value),
      readSharedAttachments(token.value),
      readSharedReferences(token.value)
    ]);

    if (mine === generation) held.value = { document, kind, markdown, attachments, references };
  } catch (error) {
    if (mine !== generation) return;

    failed.value = linkEnded(error)
      ? t("artifact.error.link_ended")
      : failureText(error, t("artifact.error.open_failed"));
  }
}

onMounted(() => void load());
</script>
