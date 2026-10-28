CREATE TABLE "web_search_share_notes" (
  "share_token" text NOT NULL,
  "search_id" bigint NOT NULL,
  "title" text NOT NULL,
  "ended_at" bigint NOT NULL,
  "reason" text NOT NULL,
  PRIMARY KEY ("share_token")
);
CREATE INDEX "web_search_share_notes_search" ON "web_search_share_notes" ("search_id");
CREATE INDEX "web_search_shares_expiry" ON "web_search_shares" ("expires_at");
