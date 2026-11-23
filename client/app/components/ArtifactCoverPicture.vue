<!-- A document's cover picture filling the box its classes give it, drawn from the copy near that box's size
     (docs/artifact.md, "A picture drawn small"). Nothing is fetched until the box is measured. -->
<template>
  <img
    ref="image"
    :src="src"
    alt=""
    loading="lazy"
    decoding="async"
    class="object-cover"
    data-cover-picture
    @load="loaded"
    @error="emit('error')"
  >
</template>

<script setup lang="ts">
import { useCoverPicture } from "../domain/cover-picture";

const props = defineProps<{ address: string }>();
const emit = defineEmits<{ error: [] }>();

const image = ref<HTMLImageElement | null>(null);
const { src, loaded } = useCoverPicture(() => props.address, image);
</script>
