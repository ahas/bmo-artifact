# Inventory: what bmo-artifact takes from the old bmo repository

bmo-artifact is the document archive: research documents, imported pages, learning material,
records, HTML pages, their attachments and pictures, and their share links. This inventory lists
everything it takes from the old single bmo repository and everything near it that it leaves. Each
member appears exactly once, and each excluded member names the unit that owns it. The split plan,
archive document #301, settles which unit owns what. Its addendum also settles that the old
`archive/` code is a facade over every table, and that each table goes to the service using it. Only
the document tables come here.

Phase 1 carries behaviour unchanged. A TypeScript file is ported to Rust only where a new server has
to exist, which is the HTTP API the console screen calls. Every other carried behaviour already has
a Rust original, and that original is what comes.

The "Step" column names the build step that carries a member: 1 inventory docs, 2 archive design,
3 root skeleton, 4 schema and database, 5 helpers and wording, 6 document verb and HTML read,
7 search document core, 8 research profiles, 9 search launch and runner, 10 document read API,
11 share API, 12 console plugin bundle.

## 1. Rust source taken (23 files, 7,979 lines)

| Old file | Lines | Step | What it is |
|---|---|---|---|
| `bot/bmo-server/src/verbs/archive_docs.rs` | 1,476 | 6 | The `archive doc` and `archive share` verbs: filing a record, learning or import document or an HTML page, with attachments, a narration and a share link |
| `bot/bmo-server/src/verbs/html_read.rs` | 98 | 6 | Reading a filed HTML page for its words, with its embedded pictures taken out as files |
| `bot/bmo-server/src/verbs/search/document.rs` | 778 | 7 | The structure lines of a search's result, and writing the document from them |
| `bot/bmo-server/src/verbs/search/document/tests.rs` | 617 | 7 | `document.rs`'s test module, moved into `tests/` |
| `bot/bmo-server/src/verbs/search/result.rs` | 591 | 7 | Reading a run's result |
| `bot/bmo-server/src/verbs/search/request.rs` | 540 | 7 | Parsing a search request |
| `bot/bmo-server/src/verbs/search/record.rs` | 445 | 7 | The search row's life, and the room-holding declaration for `searches` |
| `bot/bmo-server/src/verbs/search/gallery.rs` | 371 | 7 | The picture album sent to the room |
| `bot/bmo-server/src/verbs/search/images.rs` | 330 | 7 | Kept pictures |
| `bot/bmo-server/src/verbs/search/import.rs` | 210 | 7 | The document side of a page import |
| `bot/bmo-server/src/verbs/search/share.rs` | 43 | 7 | A search's share link |
| `bot/bmo-server/src/verbs/search/web.rs` | 182 | 7 | Page reading, which becomes the page-reader client of the tool host |
| `bot/bmo-server/src/verbs/search/mod.rs` | 27 | 9 | The search verb's entry |
| `bot/bmo-server/src/verbs/search/launch.rs` | 387 | 9 | Launching a search: admission, depth, queueing the run |
| `bot/bmo-server/src/verbs/search/runner.rs` | 542 | 9 | The search runner: the run's session, its branches, its document |
| `bot/bmo-server/src/verbs/research/mod.rs` | 895 | 8 | The research base under the search line: profiles, the clock, places in the co-worker pool, knowledge, spend |
| `bot/bmo-server/src/verbs/research/admission.rs` | 292 | 8 | Whether a deeper run goes as asked, and the wait for its first seat |
| `bot/bmo-server/src/verbs/research/branch.rs` | 329 | 8 | The `search branch` verb behind the run's branch tool |
| `bot/bmo-server/src/verbs/research/run.rs` | 629 | 8 | One research run's model, from its line to its record |
| `bot/bmo-server/src/control/research.rs` | 517 | 8 | Research profiles, their fields and their checks |
| `bot/bmo-server/src/control/research_settings.rs` | 457 | 8 | The research-settings routes |
| `archive/archive-pool/src/lib.rs` | 167 | 4 | Connection pools |
| `testbed/scratch-archive/src/lib.rs` | 201 | 4 | A scratch database per test |

The 4 files under `verbs/research/` are 2,145 lines.

## 2. Old integration tests moved into `tests/` (3 files)

| Old file | Lines | Step |
|---|---|---|
| `bot/bmo-server/tests/integration/verbs_archive.rs` | 679 | 6 |
| `bot/bmo-server/tests/integration/search_verb.rs` | 350 | 9 |
| `bot/bmo-server/tests/integration/search_runner.rs` | 878 | 9 |

In-file test modules move with the file that holds them (14 modules). `archive_docs` moves in step 6.
`request`, `share`, `import`, `images`, `result`, `gallery` and `document/tests` move in step 7.
`control/research` and the four of `verbs/research` (`mod`, `admission`, `branch`, `run`) move in step
8, and `search/launch` in step 9. Two cases of `verbs/research/mod` are not carried, since what they
test is bmo-agency's: reading the registration a home names, and counting what a session spent.

## 3. Schema statements taken (6 tables, 4 indexes)

The six tables are `searches`, `search_findings`, `search_sources`, `search_images`,
`archive_attachments` and `web_search_shares`. They are declared once, as the old archive declares
them, together with their four indexes. All of it lands in step 4. Two of the old `bmo-archive` files
supply them:

- `src/migration/archive.sql` supplies the six tables' statements and the four indexes.
- `src/migration/m20261006_000001_users.rs` supplies the one later change to these tables: a
  nullable `user_id` column on `web_search_shares`. Its default reads another unit's settings table,
  so the default is not carried.

## 4. Old bmo-server internals the carried files reach (36 members)

None of these is copied as a dependency. Each is rebuilt in this repository or replaced by a call
to the unit that owns it.

| Members | Count | Step | Becomes |
|---|---|---|---|
| `Shell`, `VerbError`, `Call`, `Refused`, `Route`, `answer` | 6 | 3 | This repository's own error, CLI and HTTP plumbing |
| `Refusal`, `all`, `one`, `run`, `now_ms` | 5 | 4 | Rebuilt over this repository's pool |
| `lines::{Trees, read_required, render}`, `run_log::{json::Json, lines::BRANCH_TOOLS, lines::stream_lines}` | 6 | 5 | Copied helpers |
| `room_record` | 1 | 7 | This repository's room-holding route |
| `control::{harness, pool, tiers, claude_accounts}` | 4 | 8 | The bmo-agency client |
| `runs`, `queue`, `job_run` | 3 | 9 | The bmo-job client: a run's row opened, launched, read and closed in its ledger, the runs that carried a search, and an ending handed back; the stop's outcome word and the live task names are this repository's own |
| `reference::road` | 1 | 9 | The quiet-instruction line is this repository's own; the knowledge a run is handed is picked by bmo-gate through its client |
| `places::Ground` | 1 | 9 | Settings: the database, the file root, the prompt tree and each unit's address; the launching process's home and turn are read from its environment |
| `lines::runner::Search` | 1 | 9 | The bmo-agency client: the run apart under the instructions bmo-agency composes for the search line |
| `chat::voice`, `voice::brief` | 2 | 9 | The bmo-voice client |
| `remote_browser::Stop` | 1 | 9 | This repository's own stop listener: it hears the process's own signals, and nothing of it is bmo-explorer's |
| `Shell::owner`, `providers::is_owner_room` | 2 | 6 | The owner's private room and the default asker, named in settings |
| `AGENT_KEY`, `SENDER_NAME`, `OWNER_NAME` | 3 | 6 | The writer and sender names in settings; the owner's name is not carried |

`archive_docs` starts a narration through `voice::brief`, so the bmo-voice client's briefing call
lands in step 6, and step 9 adds the rest of that client.

Beside these internals, the search launcher read bmo-receiver's `messages` table for who sent the
asking message, when and on what road; that read becomes the bmo-receiver client.

## 5. Rust twins copied from internal crates (7 items)

- `markup/bmo-markup/src/web_address.rs` (bmo-markup), step 5
- `fill_marks` (bmo-policy's library root), step 5
- `count_text`, `Said` and `cap_items` (bmo-tool's ceiling module), step 5
- `raster_type` and `archived` (bmo-picture), step 6

Only these items are copied, with what they stand on:

- `web_address.rs` stands on bmo-markup's JavaScript white space and character helpers, which
  `run_log::lines::stream_lines` also trims with;
- the ceiling module comes whole, `cap_value` and its constants with it;
- `run_log::lines` reads a toolbook proxy call as the tool it reached, so harness-providers' toolbook
  reading comes with it;
- `archived` stands on bmo-picture's picture decoding, its AVIF and SVG readers included, and on
  its fitting of a picture inside a side.

The crates themselves are not depended on. `lines::Trees` comes as the one tree the code reads, the
prompt tree the settings name, without the constructor that read `places::Ground`.

## 6. TypeScript ported to Rust (16 files)

| Old file | Step |
|---|---|
| `archive/archive/archive-kinds.ts` | 10 |
| `archive/archive/archive-kinds.test.ts` | 10 |
| `archive/archive/archive-kinds/kind.ts` | 10 |
| `archive/archive/archive-kinds/search.ts` | 10 |
| `archive/archive/archive-kinds/search.test.ts` | 10 |
| `archive/archive/archive-kinds/import.ts` | 10 |
| `archive/archive/archive-kinds/learning.ts` | 10 |
| `archive/archive/archive-kinds/record.ts` | 10 |
| `archive/archive/archive-kinds/page.ts` | 10 |
| `archive/archive/archive-document-row.ts` | 10 |
| `markup/markup/finding-markdown.ts` | 10 |
| `markup/markup/markdown-structure.ts` | 10 |
| `chat/chat/page-preview.ts` | 10 (its fetch goes through the page reader) |
| `chat/chat/picture-ground.ts` | 10 |
| `client/server/utils/media.ts` (only the parts the archive routes use) | 10 |
| `archive/archive/search-share-rows.ts` | 11 |

### TypeScript tests whose cases become Rust tests (4 files)

These test the behaviour carried above, and no earlier list named them.

| Old file | Step |
|---|---|
| `bot/bot/archive-documents.test.ts` | 6 |
| `bot/bot/archive-kinds/kinds.test.ts` | 10 |
| `markup/markup/finding-markdown.test.ts` | 10 |
| `markup/markup/markdown-structure.test.ts` | 10 |

## 7. Console graph node `console/archive-screen` (58 entries)

`docs/archive.md` lands in step 2. The 57 code files split as follows.

**Copied into `client/` in step 12 (28 files).**

- Pages (3): `client/app/pages/archive/index.vue`, `client/app/pages/archive/[id].vue`,
  `client/app/pages/share/[token].vue`.
- Components (11): `ArchiveSourceItem.vue`, `ArchiveBody.vue`, `ArchiveSources.vue`,
  `ArchiveShareDialog.vue`, `ArchiveAttachments.vue`, `ArchiveTextPreview.vue`,
  `ArchiveRowSkeleton.vue`, `ArchivePageFrame.vue`, `HostedPageFrame.vue`,
  `DocumentPictureViewer.vue`, `ShareLinkDialog.vue`.
- Domain (3): `client/app/domain/archive.ts`, `client/app/domain/document-pictures.ts`,
  `client/app/domain/share-links.ts`.
- Shared (3): `client/shared/document-pictures.ts`, `client/shared/share-head.ts`,
  `client/shared/document-design.ts`.
- Tests (8), moved to `client/tests`: `client/app/pages/archive/index.test.ts`,
  `client/app/components/ArchiveBody.test.ts`, `client/app/components/ShareLinkDialog.test.ts`,
  `client/app/domain/archive.test.ts`, `client/shared/document-pictures.test.ts`,
  `client/browser/document-pictures.test.ts`, `client/shared/share-head.test.ts`,
  `client/shared/document-design.test.ts`.

**Reimplemented in Rust as the document read API, step 10 (15 files).**

- `client/server/utils/searches.ts` and `searches.test.ts`.
- `client/server/utils/moved-screens.ts`, `moved-screens.test.ts` and
  `client/server/middleware/moved-screens.ts`.
- `client/server/utils/hosted-page.ts` and `hosted-page.test.ts`.
- `client/server/utils/page-bridges.ts`.
- Six routes under `client/server/api/archive/`: `index.get.ts`, `[id]/index.get.ts`,
  `[id]/document.get.ts`, `[id]/refs/[ref]/preview.get.ts`, `[id]/refs/[ref]/image.get.ts` and
  `[id]/attachments/[attachment].get.ts`.
- `client/server/api/archive/archive.test.ts`.

**Reimplemented in Rust as the share API, step 11 (14 files).**

- `client/server/utils/search-shares.ts` and `search-shares.test.ts`.
- `client/server/utils/card-picture.ts`.
- `client/server/api/archive/[id]/share.get.ts`, `share.post.ts` and `share.delete.ts`.
- `client/server/api/archive/share-routes.test.ts`.
- Six routes under `client/server/api/share/[token]/`: `index.get.ts`,
  `refs/[ref]/preview.get.ts`, `refs/[ref]/image.get.ts`, `attachments/[attachment].get.ts`,
  `head.get.ts` and `picture.get.ts`.
- `client/server/api/share/share.test.ts`.

**What the bundle depends on.** UI primitives come from `bmo-ui`. Language-pack keys stay in
bmo-web's language pack: the screen calls `t()` with the same keys it uses today. The console's own
components the screen stands on, which bmo-ui does not carry, are the host's: the markdown renderer
that draws a finding, the table of contents and the three pieces of the title bar (`docs/layout.md`,
"The console plugin bundle"). The kinds' outlines, a finding's name, title and fold, and the page
bridges come from this repository's server side rather than from the bundle.

## 8. Policy files (5)

| Old file | Where it goes |
|---|---|
| `policy/bmo-policy/policy/research.toml` | Here, as this repository's own settings (step 8) |
| `policy/bmo-policy/policy/research-seeds.toml` | Here, as this repository's own settings (step 8) |
| `policy/bmo-policy/policy/tiers.toml` | bmo-agency. Step 8 reads it through bmo-agency's API |
| `policy/bmo-policy/policy/search-engines.toml` | bmo-explorer. Step 8 reads its values through the tool host |
| `policy/bmo-policy/policy/archive-seeds.toml` | bmo-agency (agent seeds). Not read here |

## 9. Language-pack files with Korean text (3)

| Old file | Step | Becomes |
|---|---|---|
| `lang/bmo-lang/lang/ko/archive-docs.yaml` (3 groups) | 5 | The document wording file, with an English default |
| `lang/bmo-lang/lang/ko/search.yaml` (8 groups) | 5 | The document wording file, with an English default |
| `lang/bmo-lang/lang/ko/control-answers.yaml`, its `request` key only | 8 | English messages and codes |

## 10. Hangul in carried code (4 files)

Each file is cleared in the step that carries it:

- `control/research.rs`, 21 lines (step 8)
- `control/research_settings.rs`, 4 lines (step 8)
- `verbs/search/document/tests.rs`, 2 lines (step 7)
- `tests/integration/search_runner.rs`, 2 lines (step 9)

## 11. Documents (2)

- `docs/archive.md` (984 lines, 83 of them with Hangul) is restated in English in step 2.
- The search-run parts of `docs/research-runs.md` (686 lines, 15 of them with Hangul) are restated
  in English in step 8. Its browse-only parts go to bmo-explorer.

## 12. Service units (1 new)

- `bmo-artifact.service` is a new neutral unit template, added in step 3.
- `bmo-snapshot-backup` and `bmo-restore-drill` are not carried. The backup runs over the whole data
  root, not one unit's part of it. This repository only declares, in its deploy declaration, which
  tables and which data folder a backup must include.

## 13. Tables

| Table | Owner |
|---|---|
| `searches` | bmo-artifact |
| `search_findings` | bmo-artifact |
| `search_sources` | bmo-artifact |
| `search_images` | bmo-artifact |
| `archive_attachments` | bmo-artifact |
| `web_search_shares` | bmo-artifact |
| `research_profiles` | bmo-artifact. bmo-explorer reads the browse profile through `GET /research-settings` |
| `document_talk` | bmo-brain. Its `document` column is plain text with no foreign key to `searches`, and it serves the brain's document viewer |
| `document_translations` | bmo-brain for the document viewer, and bmo-job for work-order translation. Same reason: plain text, no foreign key |
| `collected_items` | bmo-receiver |
| `news_briefs` | bmo-voice |
| `voice_briefings` | bmo-voice |
| `page_read_failures` | bmo-explorer |
| `web_file_shares` | bmo-drive |

## 14. Data folders

| Folder | Owner |
|---|---|
| `searches/` (document files, `images/`, `attachments/`, per document id) | bmo-artifact |
| `collected/` | bmo-receiver |
| `news-brief/` | bmo-voice |
| `archive-copies/`, `archive-return/` | Not carried. Database-move transition copies |

## 15. Excluded: each line names the owning unit

### Old Rust crates and server files

- `archive/bmo-archive`, the other 49 of its 51 tracked files. This is the whole bot archive's
  PostgreSQL tooling: shape, cells, check, connect, copy, freeze, reverse, rights, verify, breaks,
  the `archive-move` binary, the migrations over every table, `todos_merge.sql`, its manifest, its
  library root and its three integration test files. It stays with the old repository for the
  switch-over's data move.
- `archive/archive-read` (6 files: its manifest, `lib.rs`, `registry.rs` and three integration test
  files) is a reader any tool uses across both database engines, so it is not carried. Its
  `registry.rs` is the model registry and goes to bmo-agency.
- `archive/archive-pool/Cargo.toml` and `testbed/scratch-archive/Cargo.toml` are replaced by this
  repository's root manifest.
- `bot/bmo-server/src/verbs/archive_readers.rs` is the output gate's casebook plus a co-work timing
  read. It stays with the units that own those: the gate and bmo-job.
- `bot/bmo-server/src/verbs/snapshot.rs` belongs to the backup facility and is not carried.
- `bot/bmo-server/src/verbs/browse.rs` and `browse_runner.rs` go to bmo-explorer. They call the
  research base carried here and reach it through this repository's `search branch` verb and
  settings routes.
- `bot/bmo-server/src/verbs/news_brief_runner/` (6 files) goes to bmo-voice.
- `bot/bmo-server/src/editors/` (26 files) are the brain's editors and go to bmo-brain.
- `web/web-layer` (42 files) goes to bmo-explorer. Page import, picture fetch, address checks,
  pacing and search engines are reached through the tool host.

### `archive/archive` (124 files)

- **Ported (11 files):** `archive-kinds.ts`, `archive-kinds.test.ts`, the seven files under
  `archive-kinds/`, `archive-document-row.ts` and `search-share-rows.ts` (all in §6).
- **Rust twin carried, so not ported (4 files):**
  - `search-store.ts` and `search-store.test.ts`: the twin is `record.rs` and `archive_docs.rs`.
  - `research-fields.ts` and `research-profile.ts`: the twin is `verbs/research` and
    `control/research.rs`. The console form's copy of the field checks stays with the console's
    agents screen in bmo-web, which calls this repository's research-settings routes.
- **bmo-voice (6 files):** `briefing-store.ts`, `briefing-store.test.ts`, `news-brief-store.ts`,
  `news-brief-store.test.ts`, `voice-store.ts`, `voice-store.test.ts`.
- **bmo-drive (1 file):** `file-share-rows.ts`.
- **bmo-node (3 files):** `nodes.ts`, `node-state.ts`, `node-state.test.ts`.
- **Whole-archive database tooling, which stays with the old repository for the switch-over
  (13 files):** `archive-schema.ts`, `archive-migrator.ts`, `archive-lock.ts`,
  `archive-lock.test.ts`, `archive-place.ts`, `archive-snapshot.ts`, `archive-sql.ts`,
  `archive-sql.test.ts`, `sqlite-archive-sql.ts`, `postgres-archive-sql.ts`, `postgres-client.ts`,
  `postgres-reconnect.test.ts`, `archive.ts`.
- **Package configuration, replaced by this repository's own (3 files):** `package.json`,
  `tsconfig.json`, `bunfig.toml`.
- **Facade over tables other units own (83 files).** Each one goes to the unit that owns its table,
  in that unit's own split item:
  - `account-seed.ts`, `account-store.ts`, `agent-rest.test.ts`, `agent-store.ts`
  - `archive-seeding.ts`, `base-prompts.ts`, `bmo-events.ts`
  - `chat-store.ts`, `checkout-lease-store.ts`, `claude-account-store.ts`, `clock-hold-store.ts`
  - `companion-rooms.ts`, `companion-settings.ts`, `conversational.ts`
  - `coworker-pool.ts`, `native-coworker-pool.ts`
  - `deploy-store.ts`, `deploy-store.test.ts`
  - `execution-store.ts`, `fallback-store.ts`
  - `fragment-row-store.ts`, `fragment-titles.ts`
  - `gate-rows.ts`, `gate-store.ts`
  - `hardware-tables.ts`, `holiday-store.ts`, `instruction-store.ts`
  - `job-store.ts`, `job-store.test.ts`
  - `laya-health-store.ts`, `laya-rows.ts`, `laya-verdicts.ts`
  - `legacy-routes-fixture.ts`, `media-store.ts`, `memory-row-store.ts`
  - `message-arrival.test.ts`, `message-store.ts`, `message-store.test.ts`
  - `model-catalogue.ts`, `model-registry.ts`, `model-registry.test.ts`, `model-tiers.ts`
  - `note-store.ts`, `outcomes.ts`, `outcomes.test.ts`
  - `provider-names.ts`, `provider-number-store.ts`, `provider-numbers.ts`
  - `query-rows.ts`, `question-store.ts`
  - `random-draw-store.ts`, `random-draw-store.test.ts`
  - `reference-verdicts.ts`, `reminder-store.ts`
  - `room-session-store.ts`, `route-modes.ts`
  - `row-copy.ts`, `row-steps.ts`
  - `run-rows.ts`, `run-store.ts`
  - `seed-refresh.test.ts`, `session-context-store.ts`, `session-resets.ts`, `setting-store.ts`
  - `talk-store.ts`
  - `test-databases.ts`, `test-engine.ts`, `test-server.ts`, `test-sql-thread.ts`
  - `todo-merge.test.ts`, `todo-note-store.ts`, `todo-store.ts`
  - `usage-store.ts`, `verdict-store.ts`, `watch-store.ts`
  - `web-policy.ts`, `web-policy.test.ts`
  - `work-node-store.ts`, `work-node-store.test.ts`
  - `work-order-lock-store.ts`, `work-order-mark.ts`, `work-order-shape.ts`
  - `worker-session-store.ts`

### `markup/markup` (32 files)

- **Ported (4 files):** `finding-markdown.ts`, `finding-markdown.test.ts`, `markdown-structure.ts`,
  `markdown-structure.test.ts` (§6).
- **Rust twin carried (4 files):**
  - `web-address.ts` and `web-address.test.ts`: the twin is `web_address.rs`, step 5.
  - `html-reading.ts` and `html-reading.test.ts`: the twin is `html_read.rs`, step 6.
- **The shared text library, left to its own unit (24 files):**
  - `answer-cap-cases.json`, `answer-cap.ts`, `answer-cap.test.ts`, `answer-language.ts`
  - `dashes.ts`, `dashes.test.ts`, `format.ts`, `format.test.ts`
  - `korean-in-code-ledger.json`, `korean-in-code.ts`, `korean-in-code.test.ts`
  - `markdown.ts`, `markdown.test.ts`, `policy-reads.ts`, `policy-reads.test.ts`
  - `rust-reading.ts`, `telegram-parts.ts`, `translation-parts.ts`, `wording.ts`
  - `written-dashes.ts`, `written-dashes.test.ts`
  - `package.json`, `tsconfig.json`, `bunfig.toml`

### `bot/bot` archive and research files (18)

- **Cases moved to Rust tests (2 files):** `archive-documents.test.ts` and
  `archive-kinds/kinds.test.ts` (§6).
- **bmo-agency (2 files):** `archive-seeds.ts`, `archive-seeds.test.ts`.
- **Whole-archive start-up and shape checks, which stay with the old repository for the switch-over
  (6 files):** `archive-start.ts`, `archive-start.test.ts`, `archive-numbering.test.ts`,
  `archive-open.test.ts`, `archive-schema.test.ts`, `archive-todo-shape.test.ts`.
- **Rust twin carried in step 8 (6 files):** `research-controls.ts`, `research-defaults.ts`,
  `research-effort.ts`, `research-effort.test.ts`, `research-profile.test.ts`,
  `research-mcp.test.ts`.
- **The tool host (2 files):** `research-environment.ts` and `research-environment.test.ts`. They
  state what a research run's web tools are told, and the tool host reads them.

### Scripts

- `scripts/html-read.ts`: the Rust twin `html_read.rs` is carried in step 6.
- `repo/repo/snapshot-backup.ts`: the backup facility, not carried (§12).

### Console files outside the archive-screen node

- `client/server/utils/archive.ts` goes to bmo-receiver (rooms, messages, judgements).
- `client/server/utils/document-records.ts`, `client/server/utils/document-fragments.ts` and the
  nine routes under `client/server/api/documents/` go to bmo-brain: the document viewer.
- `client/server/api/todos/[id]/work-order/translation.get.ts` and `translation.post.ts` go to
  bmo-job: work-order translation.
- `client/server/utils/code-reviews.ts` goes to bmo-review.
- The twelve routes under `client/server/api/files/` and the two under `client/server/api/share/files/`
  go to bmo-drive: file shares.
- The console's research-settings form (`ResearchSettings.vue`, its test, `domain/research.ts`, its
  test, `domain/research-settings.ts`, and the `research-settings` get and post routes with the post
  route's test) stays on the console's agents screen in bmo-web, which calls this repository's
  research-settings routes.

## 16. Facts on these paths

- bmo-policy's deployment module holds the agent's name and the owner's name as constants. This
  repository depends on no bmo-policy, so the constants do not travel here.
- `archive_docs.rs` puts the owner's name into two refusal messages. Step 6 rewrites both so that
  they name no person, and takes the default room and author from settings.
- `nodes.ts` holds a personal path for the node hub's token file. Its owner is named in section 15,
  so the path does not travel here.
- `snapshot.rs` fixes the deploy tree's directory layout into a named root. It is not carried (see
  section 15), so the layout does not travel here.
