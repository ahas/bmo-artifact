import type { ShareSpanKey } from "../../shared/config";

// A share link as the one dialog that makes and ends links reads it, whatever road it is handed. The
// dialog keeps the mechanism — the form, the address, the time left, the copy — and the screen hands it
// the road its links are read, made and ended by.

export interface ShareLink {
  token: string;
  // When the link stops opening, and null where it never does.
  expiresAt: number | null;
}

export interface ShareRoad {
  // The link's page, under the console's own base.
  page: (token: string) => string;
  // The link that stands, or null where there is none.
  read: () => Promise<ShareLink | null>;
  // A link made living as long as the span says.
  make: (span: ShareSpanKey) => Promise<ShareLink>;
  remove: (link: ShareLink) => Promise<void>;
}

// What a screen that must not hold the link itself is told of it.
export interface ShareMark {
  expiresAt: number | null;
}
