<template>
  <!-- The organisation units one document is shared with (docs/artifact.md, 「Where a person reads it」): a unit picked
       from the owner's company's tree and shared with, and each unit shared with on a row of its own that takes it
       off. bmo-ui's share dialog takes no part a host brings, so this stands as a small dialog of its own beside it,
       in the middle at every width as the settings dialog puts it. -->
  <SettingsDialog v-model:open="open" :title="$t('artifact.title.unit_share')" :description="props.subject">
    <section class="flex min-w-0 flex-col gap-3" data-share-units>
      <UFormField :label="$t('artifact.label.units')">
        <div class="flex items-center gap-2">
          <ChoiceSelect
            v-model="chosen"
            :items="offered"
            value-key="value"
            :placeholder="$t('artifact.placeholder.unit')"
            :loading="loading"
            :disabled="loading || choices === null"
            icon="i-material-symbols-account-tree-outline"
            :aria-label="$t('artifact.label.units')"
            class="min-w-0 flex-1"
            data-unit-choice
          />

          <UButton :label="$t('common.action.share')" :loading="sharing" :disabled="chosen === undefined" data-unit-share @click="share" />
        </div>
      </UFormField>

      <ContentSkeleton v-if="loading" list-class="flex flex-col gap-2" :label="$t('artifact.status.units_loading')">
        <div class="flex items-center gap-2" data-unit-skeleton>
          <div class="flex min-w-0 flex-1 flex-col">
            <SkeletonLine size="base" width="w-24" />
            <SkeletonLine width="w-40" />
          </div>
          <SkeletonBar class="size-8 rounded-md" />
        </div>
      </ContentSkeleton>

      <p v-else-if="rows.length === 0" class="text-sm text-muted" data-no-units>{{ $t("artifact.status.no_units") }}</p>

      <ul v-else class="flex flex-col gap-2" data-unit-list>
        <li v-for="row in rows" :key="row.id" class="flex items-center gap-2" :data-unit="row.id">
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate">{{ row.name }}</span>
            <span v-if="row.line !== ''" class="truncate text-sm text-muted">{{ row.line }}</span>
          </div>

          <IconButton
            :label="$t('common.action.unshare')"
            icon="i-material-symbols-group-remove-outline"
            :loading="working.includes(row.id)"
            data-unit-remove
            @click="remove(row.id)"
          />
        </li>
      </ul>

      <UAlert v-if="failed !== ''" color="error" variant="subtle" :description="failed" />
    </section>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { unitFailure, unitRoad } from "../domain/units";

import type { UnitChoice } from "../domain/units";

// Every change stands as it is made: a share and a removal are each stored on their own, the control that made one
// spinning until it stands. What stands here is read afresh each time it opens and left standing as it closes.

interface Props {
  documentId: number;
  // The document's title, which the screen behind the dialog does not say where it is read.
  subject?: string;
}

const props = withDefaults(defineProps<Props>(), { subject: undefined });
const open = defineModel<boolean>("open", { default: false });

const { t } = useI18n();
const explain = unitFailure(t);

const choices = ref<UnitChoice[] | null>(null);
const shared = ref<number[]>([]);
const loading = ref(false);
const chosen = ref<number | undefined>(undefined);
const sharing = ref(false);
const working = ref<number[]>([]);
const failed = ref("");
let generation = 0;

const road = computed(() => unitRoad(props.documentId));

// What a unit is told apart by under its name: the units above it, or for the company that it is the company.
function lineOf(choice: UnitChoice): string {
  return choice.above.length === 0 ? t("artifact.label.company") : choice.above.join(" › ");
}

// The units not yet shared with, each offered by its name with the units above it under it.
const offered = computed(() =>
  (choices.value ?? []).filter((choice) => !shared.value.includes(choice.id)).map((choice) => ({ label: choice.name, description: lineOf(choice), value: choice.id }))
);

// Each unit shared with, named as the tree names it; one the tree no longer holds stands by its number.
const rows = computed(() =>
  shared.value.map((id) => {
    const choice = choices.value?.find((one) => one.id === id);

    return choice === undefined ? { id, name: t("artifact.label.unit_number", { n: id }), line: "" } : { id, name: choice.name, line: lineOf(choice) };
  })
);

async function read(): Promise<void> {
  const mine = ++generation;

  loading.value = true;
  failed.value = "";

  try {
    const [tree, units] = await Promise.all([road.value.tree(), road.value.read()]);

    if (mine !== generation) return;

    choices.value = tree;
    shared.value = units;
  } catch (error) {
    if (mine === generation) failed.value = explain(error, t("artifact.error.units_read_failed"));
  } finally {
    if (mine === generation) loading.value = false;
  }
}

async function share(): Promise<void> {
  const unit = chosen.value;

  if (unit === undefined) return;

  sharing.value = true;
  failed.value = "";

  try {
    await road.value.share(unit);

    if (!shared.value.includes(unit)) shared.value = [...shared.value, unit];
    chosen.value = undefined;
  } catch (error) {
    failed.value = explain(error, t("artifact.error.unit_share_failed"));
  } finally {
    sharing.value = false;
  }
}

async function remove(unit: number): Promise<void> {
  working.value = [...working.value, unit];
  failed.value = "";

  try {
    await road.value.remove(unit);

    shared.value = shared.value.filter((id) => id !== unit);
  } catch (error) {
    failed.value = explain(error, t("common.error.unshare_failed"));
  } finally {
    working.value = working.value.filter((id) => id !== unit);
  }
}

watch(
  open,
  (standing) => {
    if (!standing) return;

    chosen.value = undefined;
    void read();
  },
  { immediate: true }
);
</script>
