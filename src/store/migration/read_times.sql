ALTER TABLE "artifact_reads" ADD COLUMN "read_at" bigint;
UPDATE "artifact_reads" SET "read_at" = "day" * 86400000;
ALTER TABLE "artifact_reads" ALTER COLUMN "read_at" SET NOT NULL;
CREATE INDEX "artifact_reads_reader" ON "artifact_reads" ("reader", "read_at");
