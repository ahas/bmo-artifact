<template>
  <!-- The start of an attached text, as it reads. It grows with its lines rather than scrolling
       inside the document, so a long file shows its head and is saved for the rest. -->
  <ContentSkeleton v-if="text === null && failed === ''" :label="$t('artifact.status.attachment_loading')">
    <SkeletonText :lines="4" size="sm" lastWidth="w-2/5" />
  </ContentSkeleton>
  <p v-else-if="failed !== ''" role="alert" class="text-sm text-error">{{ failed }}</p>
  <pre v-else class="rounded-lg border border-default bg-elevated/50 p-3 font-mono text-sm break-words whitespace-pre-wrap text-default">{{ text }}</pre>
</template>

<script setup lang="ts">
import { failureText } from "bmo-ui/app/domain/errors";
import { useI18n } from "vue-i18n";

import { textPreview } from "../domain/document-view";

const { t } = useI18n();
const props = defineProps<{ src: string }>();

// How much of a text is read for its preview. The rest is what saving it is for.
const PREVIEW_BYTES = 16 * 1024;

const text = ref<string | null>(null);
const failed = ref("");

async function load(): Promise<void> {
  text.value = null;
  failed.value = "";

  try {
    // The app's fetcher, as every call through the console's relay is made; the range header says how long
    // the whole file is, so the answer is read whole rather than its body alone.
    const response = await $fetch.raw<ArrayBuffer>(props.src, { headers: { range: `bytes=0-${PREVIEW_BYTES - 1}` }, responseType: "arrayBuffer" });

    text.value = textPreview(new Uint8Array(response._data ?? new ArrayBuffer(0)), response.headers.get("content-range"));
  } catch (error) {
    failed.value = failureText(error, t("artifact.error.attachment_read_failed"));
  }
}

watch(() => props.src, () => void load());
onMounted(() => void load());
</script>
