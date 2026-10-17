import type { Ref } from "vue";

import type { ArchiveKind, ArchiveKinds } from "../../shared/types";

// The catalogue of kinds as the read API answers it, read once and shared by every screen that asks.
// The catalogue decides what kinds there are, so a kind added there is a choice on the list and a
// label on its rows without this bundle being told.
export function useArchiveKinds(): Ref<ArchiveKind[]> {
  const { data } = useFetch<ArchiveKinds>("/api/archive/kinds", { key: "archive-kinds", lazy: true, server: false });

  return computed(() => data.value?.kinds ?? []);
}
