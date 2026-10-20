import type { components } from "./openapi";

// What the service answers, as its OpenAPI document states it (`docs/api.md`): the generated
// `openapi.ts`, named here by the members the screens read. Times are epoch milliseconds.

type Schemas = components["schemas"];

export type ArtifactDocument = Schemas["Document"];
export type ArtifactDocumentPage = Schemas["DocumentPage"];
export type ArtifactKind = Schemas["DocumentKind"];
export type ArtifactKindPage = Schemas["DocumentKindPage"];
export type ArtifactAttachment = Schemas["Attachment"];
export type ArtifactAttachmentPage = Schemas["AttachmentPage"];
export type ArtifactReference = Schemas["Reference"];
export type ArtifactReferencePage = Schemas["ReferencePage"];
export type ArtifactShare = Schemas["Share"];
export type ArtifactSharePage = Schemas["SharePage"];
