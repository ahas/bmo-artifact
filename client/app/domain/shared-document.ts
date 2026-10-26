import { artifactAddress, everyPage } from "./artifact";

import type { ArtifactAttachment, ArtifactAttachmentPage, ArtifactDocument, ArtifactKind, ArtifactReference, ArtifactReferencePage } from "../../shared/types";

// The reads a share page makes of the one document its link opens, each a share read carrying the token
// (`docs/api.md`, "Share links"), which is the whole of what a reader with no sign-in holds.

function sharedAddress(token: string, path = ""): string {
  return artifactAddress(`/shares/${encodeURIComponent(token)}${path}`);
}

/** The document the link opens. */
export function readSharedDocument(token: string): Promise<ArtifactDocument> {
  return $fetch<ArtifactDocument>(sharedAddress(token, "/document"));
}

/** The kind of the document the link opens. */
export function readSharedKind(token: string): Promise<ArtifactKind> {
  return $fetch<ArtifactKind>(sharedAddress(token, "/kind"));
}

/** Where the file of the document the link opens is served: a page in the scheme named, as a hosted page. */
export function sharedContentAddress(token: string, scheme?: string, download = false): string {
  const query = new URLSearchParams();

  if (scheme !== undefined) query.set("scheme", scheme);
  if (download) query.set("download", "true");

  const rest = query.toString();

  return sharedAddress(token, `/content${rest === "" ? "" : `?${rest}`}`);
}

/** The markdown file of the document the link opens, as its text. */
export function readSharedContent(token: string): Promise<string> {
  return $fetch<string>(sharedContentAddress(token), { responseType: "text" });
}

/** Where the cover picture of the document the link opens is served. */
export function sharedPictureAddress(token: string): string {
  return sharedAddress(token, "/picture");
}

/** Every file the document the link opens keeps, in the order it was kept. */
export function readSharedAttachments(token: string): Promise<ArtifactAttachment[]> {
  return everyPage<ArtifactAttachmentPage["items"][number]>(`/shares/${encodeURIComponent(token)}/attachments`);
}

/** Where one file the document keeps is served, and where it is taken to be saved. */
export function sharedAttachmentAddress(token: string, attachmentId: number, download = false): string {
  return sharedAddress(token, `/attachments/${attachmentId}${download ? "?download=true" : ""}`);
}

/** Every page the document the link opens refers to, with what is kept of its preview. */
export function readSharedReferences(token: string): Promise<ArtifactReference[]> {
  return everyPage<ArtifactReferencePage["items"][number]>(`/shares/${encodeURIComponent(token)}/references`);
}

/** One page the document refers to, its preview read where none is kept. */
export function readSharedReference(token: string, referenceId: number): Promise<ArtifactReference> {
  return $fetch<ArtifactReference>(sharedAddress(token, `/references/${referenceId}`));
}

/** Where the preview picture of a page the document refers to is served. */
export function sharedReferenceImageAddress(token: string, referenceId: number): string {
  return sharedAddress(token, `/references/${referenceId}/image`);
}
