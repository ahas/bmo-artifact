CREATE TABLE "artifact_reads" (
  "search_id" bigint NOT NULL,
  "reader" text NOT NULL,
  "day" bigint NOT NULL,
  PRIMARY KEY ("search_id", "reader", "day")
);
CREATE INDEX "artifact_reads_day" ON "artifact_reads" ("day");
