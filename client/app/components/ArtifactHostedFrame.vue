<template>
  <!-- A document that is a page of its own, standing in a frame sandboxed as the page itself is
       (docs/archive.md, "An HTML page"), on the document paper rather than white while the page
       arrives. A picture pressed in the page opens in the console's picture viewer, as a markdown
       document's does ("Pictures open whole"). -->
  <iframe ref="frame" :src="props.src" :title="props.title" :sandbox="HOSTED_PAGE_SANDBOX" class="document-paper block min-h-0 w-full flex-1 border-0" />

  <ArtifactPictureViewer ref="viewer" :title="props.title" />
</template>

<script setup lang="ts">
import { HOSTED_PAGE_SANDBOX } from "../../shared/config";
import { receivedPictures } from "../../shared/document-pictures";

interface Props {
  // Where the page is served, the reader's scheme named in it.
  src: string;
  title: string;
}

const props = defineProps<Props>();

const frame = useTemplateRef("frame");
const viewer = useTemplateRef("viewer");

// Only the page in this frame is heard, and only a message of the one shape it asks by.
function onMessage(event: MessageEvent): void {
  const run = event.source !== null && event.source === frame.value?.contentWindow ? receivedPictures(event.data) : null;

  if (run !== null) viewer.value?.show(run);
}

onMounted(() => window.addEventListener("message", onMessage));
onBeforeUnmount(() => window.removeEventListener("message", onMessage));
</script>
