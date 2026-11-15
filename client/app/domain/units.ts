import { failureText } from "bmo-ui/app/domain/errors";

import { artifactAddress } from "./artifact";

import type { Translate } from "bmo-ui/shared/wording";
import type { ArtifactCompanyUnit, ArtifactCompanyUnits, ArtifactUnitSharePage } from "../../shared/types";

// The organisation units a document is shared with (`docs/artifact.md`, "Unit shares"): picked by its owner from the
// tree of the owner's company, which the artifact service reads from the account service each time it is asked, and
// kept by the service under each unit's number alone.

/** One unit as the unit dialog offers and lists it: its name, and the names of the units above it from the company
 * down. */
export interface UnitChoice {
  id: number;
  name: string;
  above: string[];
}

/** Each unit of a company's tree with the units above it, in the tree's order. */
export function unitChoices(tree: ArtifactCompanyUnit[]): UnitChoice[] {
  const units = new Map(tree.map((unit) => [unit.id, unit]));

  return tree.map((unit) => {
    const above: string[] = [];
    let parent = unit.parent === null ? undefined : units.get(unit.parent);

    // A tree the service answers has no loop; the count holds the walk to the tree's height all the same.
    while (parent !== undefined && above.length < tree.length) {
      above.unshift(parent.name);
      parent = parent.parent === null ? undefined : units.get(parent.parent);
    }

    return { id: unit.id, name: unit.name, above };
  });
}

/** The road one document's unit shares are read, made and taken off by. */
export interface UnitRoad {
  // The units of the owner's company, each with the units above it.
  tree: () => Promise<UnitChoice[]>;
  // The numbers of the units the document is shared with, in the order they were shared.
  read: () => Promise<number[]>;
  share: (unit: number) => Promise<void>;
  remove: (unit: number) => Promise<void>;
}

function unitsAddress(documentId: number, unit?: number): string {
  const units = `/documents/${documentId}/units`;

  return artifactAddress(unit === undefined ? units : `${units}/${unit}`);
}

/** The road one document's units go by, at the artifact service. */
export function unitRoad(documentId: number): UnitRoad {
  return {
    tree: async () => unitChoices((await $fetch<ArtifactCompanyUnits>(artifactAddress(`/documents/${documentId}/company-units`))).units),
    read: async () => {
      const units: number[] = [];
      let after: number | undefined;

      for (;;) {
        const page = await $fetch<ArtifactUnitSharePage>(unitsAddress(documentId), { query: { after, limit: 200 } });

        units.push(...page.items.map((share) => share.unit));

        if (!page.more || page.items.length === 0) return units;

        after = page.items[page.items.length - 1]!.id;
      }
    },
    share: async (unit) => {
      await $fetch(unitsAddress(documentId), { method: "POST", body: { units: [unit] } });
    },
    remove: async (unit) => {
      await $fetch(unitsAddress(documentId, unit), { method: "DELETE" });
    }
  };
}

// The service's code for a unit outside the owner's company (`docs/api.md`, "Unit shares").
const OTHER_COMPANY = "other-company";

/** The unit dialog's reading of a failure: a unit of another company refused in the reader's own words, and any other
 * failure as bmo-ui reads it. */
export function unitFailure(t: Translate): (error: unknown, fallback: string) => string {
  return (error, fallback) =>
    (error as { data?: { code?: unknown } } | null)?.data?.code === OTHER_COMPANY ? t("artifact.error.unit_other_company") : failureText(error, fallback);
}
