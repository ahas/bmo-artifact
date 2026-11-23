import type { components } from "./openapi";

// What the service answers, as its OpenAPI document states it (`docs/api.md`): the generated
// `openapi.ts`, named here by the members the screens read. Times are epoch milliseconds.

type Schemas = components["schemas"];

export type ArtifactDocument = Schemas["Document"];
export type ArtifactDocumentPage = Schemas["DocumentPage"];
export type ArtifactDocumentReads = Schemas["DocumentReads"];
export type ArtifactDocumentReadsPage = Schemas["DocumentReadsPage"];
export type ArtifactRecentRead = Schemas["RecentRead"];
export type ArtifactRecentReadsPage = Schemas["RecentReadsPage"];
export type ArtifactKind = Schemas["DocumentKind"];
export type ArtifactKindPage = Schemas["DocumentKindPage"];
export type ArtifactAttachment = Schemas["Attachment"];
export type ArtifactAttachmentPage = Schemas["AttachmentPage"];
export type ArtifactReference = Schemas["Reference"];
export type ArtifactReferencePage = Schemas["ReferencePage"];
export type ArtifactShare = Schemas["Share"];
export type ArtifactSharePage = Schemas["SharePage"];
export type ArtifactMemberShare = Schemas["MemberShare"];
export type ArtifactMemberSharePage = Schemas["MemberSharePage"];
export type ArtifactUnitShare = Schemas["UnitShare"];
export type ArtifactUnitSharePage = Schemas["UnitSharePage"];
export type ArtifactCompanyUnit = Schemas["CompanyUnit"];
export type ArtifactCompanyUnits = Schemas["CompanyUnits"];
export type ArtifactLabelValue = Schemas["LabelValue"];
export type ArtifactLabelValuePage = Schemas["LabelValuePage"];
