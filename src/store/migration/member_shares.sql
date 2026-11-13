CREATE TABLE "artifact_member_shares" (
  "id" bigserial PRIMARY KEY,
  "search_id" bigint NOT NULL,
  "user_key" text NOT NULL,
  "permission" text NOT NULL,
  "shared_at" bigint NOT NULL,
  UNIQUE ("search_id", "user_key")
);
CREATE INDEX "artifact_member_shares_user" ON "artifact_member_shares" ("user_key");
