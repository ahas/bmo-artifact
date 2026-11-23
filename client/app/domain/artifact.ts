import { relayAddress } from "bmo-ui/app/domain/relay";

import { SHARE_SPANS } from "../../shared/config";

import type { Ref } from "vue";
import type { ShareLink, ShareRoad } from "bmo-ui/app/domain/share-links";
import type {
  ArtifactAttachment,
  ArtifactAttachmentPage,
  ArtifactDocument,
  ArtifactDocumentPage,
  ArtifactDocumentReads,
  ArtifactDocumentReadsPage,
  ArtifactRecentRead,
  ArtifactRecentReadsPage,
  ArtifactKind,
  ArtifactKindPage,
  ArtifactLabelValue,
  ArtifactLabelValuePage,
  ArtifactReference,
  ArtifactReferencePage,
  ArtifactShare,
  ArtifactSharePage
} from "../../shared/types";

// The document screens' calls to the artifact service, each through the console's relay at the path
// the service's own API writes it (bmo-ui's `docs/console-bundles.md`, "How a bundle mounts";
// `docs/api.md`).

const SERVICE = "artifact";

/**
 * The address a path of the service is called at through the console's relay. It stands under the base
 * path the console app is served at, so a picture, a file or a frame the browser loads by its address
 * reaches the relay as a call does.
 */
export function artifactAddress(path: string): string {
  return relayAddress(SERVICE, path);
}

/** What the list is narrowed and paged by; `picture` holds it to the documents keeping a picture. */
export interface DocumentsAsked {
  kind?: string;
  label?: string;
  q?: string;
  picture?: boolean;
  before?: number;
  limit: number;
}

/** A page of documents, newest first. */
export function readDocuments(asked: DocumentsAsked): Promise<ArtifactDocumentPage> {
  return $fetch<ArtifactDocumentPage>(artifactAddress("/documents"), { query: asked });
}

/** A read of one document kept for the signed-in reader, once a day (`docs/artifact.md`, "Reads"). */
export async function keepRead(id: number): Promise<void> {
  await $fetch(artifactAddress(`/documents/${id}/reads`), { method: "POST" });
}

/** The documents most read in the last seven days, the most first. */
export async function readMostRead(limit: number): Promise<ArtifactDocumentReads[]> {
  return (await $fetch<ArtifactDocumentReadsPage>(artifactAddress("/document-reads"), { query: { limit } })).items;
}

/** The documents the signed-in reader read last in the last seven days, the one opened last first. */
export async function readRecentRead(limit: number): Promise<ArtifactRecentRead[]> {
  return (await $fetch<ArtifactRecentReadsPage>(artifactAddress("/document-reads/recent"), { query: { limit } })).items;
}

/** One document's head. */
export function readDocument(id: number): Promise<ArtifactDocument> {
  return $fetch<ArtifactDocument>(artifactAddress(`/documents/${id}`));
}

/**
 * How a page is asked for: the scheme it is drawn in and the language its player speaks, as a hosted page,
 * or the file itself to keep.
 */
export interface PageAsked {
  scheme?: string;
  language?: string;
  download?: boolean;
}

/** A page's asking as the query its address carries; empty where nothing is asked. */
export function pageQuery(asked: PageAsked): string {
  const query = new URLSearchParams();

  if (asked.scheme !== undefined) query.set("scheme", asked.scheme);
  if (asked.language !== undefined) query.set("language", asked.language);
  if (asked.download === true) query.set("download", "true");

  const rest = query.toString();

  return rest === "" ? "" : `?${rest}`;
}

/** Where a document's file is served: a page as asked, as a hosted page. */
export function contentAddress(id: number, asked: PageAsked = {}): string {
  return artifactAddress(`/documents/${id}/content${pageQuery(asked)}`);
}

/** A markdown document's file as its text. */
export function readContent(id: number): Promise<string> {
  return $fetch<string>(contentAddress(id), { responseType: "text" });
}

/** Where a document's cover picture is served. */
export function pictureAddress(id: number): string {
  return artifactAddress(`/documents/${id}/picture`);
}

/** Every member of a collection paged by `after`, read a page at a time until none is left. */
export async function everyPage<T extends { id: number }>(path: string): Promise<T[]> {
  const items: T[] = [];
  let after: number | undefined;

  for (;;) {
    const page = await $fetch<{ items: T[]; more: boolean }>(artifactAddress(path), { query: { after, limit: 200 } });

    items.push(...page.items);

    if (!page.more || page.items.length === 0) return items;

    after = page.items[page.items.length - 1]!.id;
  }
}

/** Every file a document keeps, in the order it was kept. */
export function readAttachments(id: number): Promise<ArtifactAttachment[]> {
  return everyPage<ArtifactAttachmentPage["items"][number]>(`/documents/${id}/attachments`);
}

/** Where one file a document keeps is served, and where it is taken to be saved. */
export function attachmentAddress(documentId: number, attachmentId: number, download = false): string {
  return artifactAddress(`/documents/${documentId}/attachments/${attachmentId}${download ? "?download=true" : ""}`);
}

/** Every page a document refers to, with what is kept of its preview. */
export function readReferences(id: number): Promise<ArtifactReference[]> {
  return everyPage<ArtifactReferencePage["items"][number]>(`/documents/${id}/references`);
}

/** One page a document refers to, its preview read where none is kept. */
export function readReference(documentId: number, referenceId: number): Promise<ArtifactReference> {
  return $fetch<ArtifactReference>(artifactAddress(`/documents/${documentId}/references/${referenceId}`));
}

/** Where the preview picture of a page a document refers to is served. */
export function referenceImageAddress(documentId: number, referenceId: number): string {
  return artifactAddress(`/documents/${documentId}/references/${referenceId}/image`);
}

/** The kinds a document may be filed as, in the catalogue's order. */
export async function readKinds(): Promise<ArtifactKind[]> {
  return (await $fetch<ArtifactKindPage>(artifactAddress("/document-kinds"), { query: { limit: 200 } })).items;
}

/** The kinds a document may be filed as, read once and shared by every screen that asks. */
export function useKinds(): Ref<ArtifactKind[]> {
  const { data } = useFetch<ArtifactKindPage>(artifactAddress("/document-kinds"), { key: "artifact-kinds", lazy: true, server: false });

  return computed(() => data.value?.items ?? []);
}

/** The words a label holds across the documents, the most carried first: what the list's category filter offers. */
export async function readLabelValues(key: string): Promise<ArtifactLabelValue[]> {
  return (await $fetch<ArtifactLabelValuePage>(artifactAddress("/label-values"), { query: { key, limit: 200 } })).items;
}

/** The live link of one document, null where it has none. */
export async function shareOf(documentId: number): Promise<ArtifactShare | null> {
  const page = await $fetch<ArtifactSharePage>(artifactAddress("/shares"), { query: { documentId } });

  return page.items[0] ?? null;
}

/** Every live link, by the document it opens: what the list's share badges read (docs/artifact.md, 「Where a person reads it」). */
export async function readLiveShares(): Promise<Map<number, ArtifactShare>> {
  const links = new Map<number, ArtifactShare>();
  let after: number | undefined;

  for (;;) {
    const page = await $fetch<ArtifactSharePage>(artifactAddress("/shares"), { query: { after, limit: 200 } });

    for (const link of page.items) links.set(link.documentId, link);

    // The page's cursor is the moment its last link was made.
    if (!page.more || page.items.length === 0) return links;

    after = page.items[page.items.length - 1]!.createdAt;
  }
}

/** One link and the document it opens, read by its token. */
export function readShare(token: string): Promise<ArtifactShare> {
  return $fetch<ArtifactShare>(artifactAddress(`/shares/${encodeURIComponent(token)}`));
}

// A link as the share dialog reads it: its end in seconds rather than milliseconds.
function dialogLink(share: ArtifactShare): ShareLink {
  return { token: share.token, expiresAt: share.expiresAt === null ? null : Math.floor(share.expiresAt / 1000) };
}

/**
 * The road one document's links are read, made and ended by, for bmo-ui's share dialog. The page a link
 * opens is the address the service answers it with.
 */
export function shareRoad(documentId: number): ShareRoad {
  const pages = new Map<string, string>();
  const known = (share: ArtifactShare): ShareLink => {
    pages.set(share.token, share.url);

    return dialogLink(share);
  };

  return {
    page: (link) => pages.get(link.token) ?? "",
    read: async () => {
      const share = await shareOf(documentId);

      return share === null ? null : known(share);
    },
    make: async (span) => {
      const ms = SHARE_SPANS.find((one) => one.value === span)?.ms ?? null;
      const share = await $fetch<ArtifactShare>(artifactAddress("/shares"), {
        method: "POST",
        body: { documentId, expiresAt: ms === null ? null : Date.now() + ms }
      });

      return known(share);
    },
    remove: async (link) => {
      await $fetch(artifactAddress(`/shares/${encodeURIComponent(link.token)}`), { method: "DELETE" });
    }
  };
}
