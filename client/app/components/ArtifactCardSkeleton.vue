<!-- The place one card of the front page keeps while its document is read: the card's frame, its cover and its
     title's and description's lines and its day, in the shape the card takes in its group (ArtifactCard). -->
<template>
  <div class="flex min-w-0 flex-col overflow-hidden rounded-lg border border-default" :class="props.lead ? 'h-full' : '@max-md/cards:flex-row'">
    <SkeletonBar
      class="rounded-none"
      :class="props.lead ? 'aspect-video w-full @xl/newest:aspect-auto @xl/newest:min-h-48 @xl/newest:flex-1' : 'aspect-video w-full shrink-0 @max-md/cards:aspect-[4/3] @max-md/cards:w-28'"
    />
    <div class="flex min-w-0 flex-col gap-2 p-3" :class="props.lead ? '' : '@max-md/cards:flex-1 @max-md/cards:justify-center @max-md/cards:gap-1'">
      <SkeletonLine :size="props.lead ? 'lg' : 'sm'" width="w-3/4" />
      <SkeletonLine :size="props.lead ? 'base' : 'sm'" width="w-full" />
      <SkeletonLine width="w-1/3" />
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ lead?: boolean }>();
</script>
