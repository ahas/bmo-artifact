<template>
  <!-- A fold of a finding's text under its label, drawn as the details fold is. What it holds is the
       reading's HTML, already cleaned with the rest of the text it came in. -->
  <UCollapsible class="flex flex-col gap-1">
    <UButton color="neutral" variant="ghost" size="sm" :label="props.label" trailing-icon="i-material-symbols-keyboard-arrow-down" class="group self-start" :ui="{ trailingIcon: 'group-data-[state=open]:rotate-180 transition-transform' }" />

    <template #content>
      <div v-html="props.html" />
    </template>
  </UCollapsible>
</template>

<script setup lang="ts">
const props = defineProps<{ label: string; html: string }>();
</script>
