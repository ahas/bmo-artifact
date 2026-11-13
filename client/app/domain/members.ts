import { inject } from "vue";

import { artifactAddress } from "./artifact";

import type { ShareMember, ShareMemberRoad, SharePermission, SharePerson } from "bmo-ui/app/domain/share-members";
import type { ArtifactMemberSharePage } from "../../shared/types";

// The members a document is shared with (`docs/artifact.md`, "Member shares"): found through the console's
// people directory (bmo-ui's `docs/console-bundles.md`, "How a bundle mounts"), kept by the artifact service
// under each member's account key alone, and handed to bmo-ui's share dialog as the road its member part goes by.

/** What the console provides under `bmo-console-people`: the members of the reader's company. */
export interface PeopleDirectory {
  search: (words: string) => Promise<SharePerson[]>;
  person: (user: string) => Promise<SharePerson | null>;
  me: () => Promise<SharePerson | null>;
}

const PEOPLE_KEY = "bmo-console-people";

/** The console's people directory, or null where the console provides none and a document is shared by link alone. */
export function usePeopleDirectory(): PeopleDirectory | null {
  return inject<PeopleDirectory | null>(PEOPLE_KEY, null);
}

// A member the directory does not answer stands as the account key it is kept by.
async function personOf(people: PeopleDirectory, user: string): Promise<SharePerson> {
  const person = await people.person(user).catch(() => null);

  return person ?? { user, name: user, organization: "", jobTitle: "" };
}

function memberAddress(documentId: number, user?: string): string {
  const members = `/documents/${documentId}/members`;

  return artifactAddress(user === undefined ? members : `${members}/${encodeURIComponent(user)}`);
}

/** The road one document's members are found, read, shared with, changed and taken off by. */
export function memberRoad(documentId: number, people: PeopleDirectory): ShareMemberRoad {
  return {
    search: (words) => people.search(words),
    read: async () => {
      const members: ShareMember[] = [];
      let after: number | undefined;

      for (;;) {
        const page = await $fetch<ArtifactMemberSharePage>(memberAddress(documentId), { query: { after, limit: 200 } });
        const read = await Promise.all(
          page.items.map(async (member) => ({ person: await personOf(people, member.user), permission: member.permission }))
        );

        members.push(...read);

        if (!page.more || page.items.length === 0) return members;

        after = page.items[page.items.length - 1]!.id;
      }
    },
    share: async (users: string[], permission: SharePermission) => {
      await $fetch(memberAddress(documentId), { method: "POST", body: { users, permission } });
    },
    change: async (user, permission) => {
      await $fetch(memberAddress(documentId, user), { method: "PATCH", body: { permission } });
    },
    remove: async (user) => {
      await $fetch(memberAddress(documentId, user), { method: "DELETE" });
    }
  };
}
