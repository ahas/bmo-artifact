<!-- The place one document keeps in the list: its mark, its title, what was asked, and the chips
     under them. -->
<template>
  <div class="flex gap-3 rounded-lg border border-default px-4 py-3">
    <SkeletonBar class="mt-0.5 size-5 shrink-0 rounded-sm" />
    <div class="flex min-w-0 grow flex-col gap-2">
      <SkeletonLine width="w-3/4" />
      <SkeletonLine width="w-1/2" />
      <div class="flex h-5 items-center gap-2">
        <SkeletonBar class="h-5 w-12 rounded-sm" />
        <SkeletonBar class="h-5 w-16 rounded-sm" />
        <SkeletonBar class="h-5 w-20 rounded-sm" />
      </div>
    </div>
  </div>
</template>
