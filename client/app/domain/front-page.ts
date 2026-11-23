import type { ArtifactDocument, ArtifactDocumentReads, ArtifactKind, ArtifactRecentRead, ArtifactShare } from "../../shared/types";

// What the front page last read, kept for as long as the browser tab is open (docs/artifact.md, 「The front page」), so a
// reader coming back to it from a document finds it drawn at once, at the place they left, while it is read again.

/** The front page's parts as last read; each null until it has been read once. */
export interface FrontReading {
  kinds: ArtifactKind[];
  newest: ArtifactDocument[] | null;
  strips: Map<string, ArtifactDocument[]> | null;
  recentRead: ArtifactRecentRead[] | null;
  mostRead: ArtifactDocumentReads[] | null;
  shares: Map<number, ArtifactShare> | null;
}

/** The front page's last reading, which every front page of the tab starts from and writes back. */
export const lastFront: FrontReading = { kinds: [], newest: null, strips: null, recentRead: null, mostRead: null, shares: null };
