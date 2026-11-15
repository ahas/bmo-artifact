ALTER TABLE "searches" ADD COLUMN "owner" text;
CREATE INDEX "searches_owner" ON "searches" ("owner");
ALTER TABLE "imports" ADD COLUMN "owner" text;
CREATE TABLE "artifact_unit_shares" (
  "id" bigserial PRIMARY KEY,
  "search_id" bigint NOT NULL,
  "unit_id" bigint NOT NULL,
  "shared_at" bigint NOT NULL,
  UNIQUE ("search_id", "unit_id")
);
CREATE INDEX "artifact_unit_shares_unit" ON "artifact_unit_shares" ("unit_id");
