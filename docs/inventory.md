# Inventory: what bmo-artifact takes from the old repository

Source for every item below is the old repository `~/repos/bmo` (read only, never edited).
Counts and classifications are carried whole from this item's own investigation stage; where a
decision resolves an open question that stage left standing, the resolution and its reasoning are
given here and nowhere else needs to re-derive it.

## 1. Rust crates and binaries taken whole

| Old path | Lines | New path | What it is |
|---|---|---|---|
| `archive/bmo-archive/` (29 files) | ~10,000 | `src/` (library + `bin/`) | Schema, migrations, archive-move tool, breaking-change analysis, row copy/verify/freeze, connection |
| `archive/archive-read/` (6 files) | ~500 | folded into `src/` as the read path | Read-only access over either database engine, kind registry |
| `archive/archive-pool/` (2 files) | ~200 | folded into `src/` as the pool module | Connection pooling |
| `testbed/scratch-archive/` (2 files) | ~60 | `tests/support/` | Per-test scratch copy of the schema |
| `bot/bmo-server/src/verbs/archive_docs.rs` | 1,476 | `src/controller/document.rs` + CLI verb | `bmo archive doc`, `bmo archive share` — document create, share-link create |
| `bot/bmo-server/src/verbs/archive_readers.rs` | 411 | `src/service/read.rs` | Re-export of archive-read's read operations to the old monolith; folds into the service layer directly, no longer a separate re-export shim |
| `bot/bmo-server/src/verbs/search/mod.rs` | 27 | `src/service/search/mod.rs` | module entry |
| `bot/bmo-server/src/verbs/search/launch.rs` | 387 | `src/service/search/launch.rs` | search launch orchestration |
| `bot/bmo-server/src/verbs/search/runner.rs` | 542 | `src/service/search/runner.rs` | search runner — supervises the run, writes the document |
| `bot/bmo-server/src/verbs/search/request.rs` | 540 | `src/service/search/request.rs` | search request parsing |
| `bot/bmo-server/src/verbs/search/result.rs` | 591 | `src/service/search/result.rs` | structure-line result parsing |
| `bot/bmo-server/src/verbs/search/document.rs` + `document/tests.rs` | 778 + 617 | `src/service/search/document.rs` + `tests/document.rs` | TITLE/TOPIC/KEYWORDS/RULE parsing, finding/source parsing, document writing |
| `bot/bmo-server/src/verbs/search/record.rs` | 445 | `src/service/search/record.rs` | search record management |
| `bot/bmo-server/src/verbs/search/images.rs` | 330 | `src/service/search/images.rs` | kept-picture management (AVIF conversion, bounds) |
| `bot/bmo-server/src/verbs/search/import.rs` | 210 | `src/service/search/import.rs` | page-import handling (the document side, not the fetch) |
| `bot/bmo-server/src/verbs/search/gallery.rs` | 371 | `src/service/search/gallery.rs` | gallery/album assembly for the room |
| `bot/bmo-server/src/verbs/search/share.rs` | 43 | `src/service/share.rs` | share-link creation, folded with archive_docs's own share path |
| `bot/bmo-server/src/control/research.rs` | 517 | `src/controller/document.rs` (routes) | search operation HTTP routes — becomes this repository's own `controller/`, not a "control port" of a shared process |
| `bot/bmo-server/src/control/research_settings.rs` | 457 | `src/controller/settings.rs` | search-settings HTTP routes |

Commands behind the counts: `git ls-files archive/bmo-archive archive/archive-read archive/archive-pool testbed/scratch-archive bot/bmo-server/src/verbs/archive_docs.rs bot/bmo-server/src/verbs/archive_readers.rs bot/bmo-server/src/verbs/search bot/bmo-server/src/control/research.rs bot/bmo-server/src/control/research_settings.rs` (stage-0-0); file line counts from the same stage's reading of each file.

### Internal crates these files import, carried only where they are a base library

`bmo_lang::archive_docs`, `bmo_policy`, `bmo_tool`, `sea_orm`, `regex`, `scraper`, `chrono` (stage-0-0).
Per `#301`'s principles section, only `bmo-chat`, `bmo-chat-client` and `bmo-ui` may be depended on
directly once this repository stands on its own. `bmo_lang` (the language-pack crate) and
`bmo_policy` (the shared policy-value crate) are both retired as shared crates by `#301`'s
data-folders and policy sections:
- `bmo_lang::archive_docs` — the human-language text the old verb sent into rooms moves out: this
  repository returns English messages and status codes only (`#301`'s language-pack section); the
  chat agent that receives them writes what the room reads. No language-pack crate is depended on.
- `bmo_policy` — every policy value this repository's code reads is checked against the table in
  §4 below and copied into this repository's own settings, not pulled from a shared crate.
- `bmo_tool`, `sea_orm`, `regex`, `scraper`, `chrono` are ordinary crates.io dependencies, not
  internal bmo crates, and are kept.

## 2. TypeScript logic: what has a Rust twin, what is ported, what stays behind

Stage-0-1 read all 175 files under `archive/archive` (124), `markup/markup` (32), `bot/bot/archive-*`
and `bot/bot/research-*` (17), plus `scripts/html-read.ts` and `repo/repo/snapshot-backup.ts`.

**Rule applied:** a TS file whose behaviour already has a Rust twin in `archive/bmo-archive` or
`archive/archive-read` is dropped — the twin is what ships, not a re-port of the TS. A TS file with
artifact-specific logic and no twin is ported to Rust inside this repository. A TS file that is a
facade over a table another unit owns (messages, rooms, runs-as-a-bmo-job-concept, file shares) is
excluded and left to that unit, named in §5.

### 2a. Dropped — Rust twin already exists, nothing to port

| TS file | Rust twin |
|---|---|
| `archive-schema.ts` | `archive/bmo-archive/src/shape.rs` |
| `archive-lock.ts` | `archive/bmo-archive/src/freeze.rs` |
| `archive-migrator.ts` (4,738 lines) | `archive/bmo-archive/src/migration/` |
| `archive-place.ts` | `archive/archive-read/src/lib.rs` |
| `archive-snapshot.ts` | `archive/bmo-archive/src/copy.rs` |
| `postgres-archive-sql.ts` | `archive/bmo-archive/src/connect.rs` |

`archive-sql.ts` and `sqlite-archive-sql.ts` fold into the same twin set: `bmo-archive`'s `connect.rs`
and `cells.rs` already cover both engines through one Rust type, so neither SQL-dialect TS file is
ported.

### 2b. Ported — document-table logic with no existing twin

These own document-table rows (`searches`, `search_findings`, `search_sources`, `archive_attachments`,
`search_images`, `web_search_shares`, `document_translations`, `document_talk`) and have no Rust
counterpart yet, so their behaviour is rewritten in Rust inside this repository's `src/service/`:
- `search-store.ts` (527 lines) — createSearch, finishSearch, addFinding, addSource, addAttachment,
  updateSearchTitle. Becomes `src/service/document.rs`'s write path (the Rust twin of this file is
  what stage-0-0's `archive_docs.rs`/`document.rs` already does for the CLI/runner path; the
  **console-facing** read/write that `search-store.ts` served through Nuxt server routes is the
  piece with no existing twin, and is ported here).
- `briefing-store.ts` (172 lines) — `voice_briefings` table methods. **Excluded**, not ported: see
  §4, this table belongs to bmo-voice.
- `search-share-rows.ts`, `query-rows.ts` — share-link and query-log row access for `web_search_shares`.
  Ported into `src/service/share.rs`.
- `archive-document-row.ts`, `archive-kinds.ts` + its 4 kind modules (search/import/learning/record,
  page is the 5th) — the per-kind shape module described in `docs/archive.md` 「Kinds」. Ported into
  `src/service/kinds/` as one Rust module per kind, matching the "one module answering one common
  shape" rule already written into the document being carried forward; this is the one piece of
  product logic in the TS layer with no Rust equivalent anywhere in the old tree.
- `archive-seeding.ts` — initial-row seeding for a fresh archive. Folds into `bmo-archive`'s
  migration bin as a seed step.

### 2c. Excluded — not this unit's table

`run-store.ts`, `run-rows.ts`, `file-share-rows.ts` and the "38 consumer stores" stage-0-1 found in
`archive/archive` read or write tables this unit does not own (`runs` as bmo-job's run-queue concept,
`web_file_shares` as bmo-drive's file-share table — see §4). None are copied; the service that owns
each table ports its own logic from these files in its own build.

### 2d. markup/markup (32 files) and scripts/html-read.ts, repo/repo/snapshot-backup.ts

`markup/markup` is text/markdown formatting used by many units, not artifact-specific; it is a
candidate for its own shared library and is out of this unit's scope to carry (`bmo-markup` already
exists as a Rust crate and is the twin bmo-artifact depends on, same as bmo-server does today —
stage-0-6: `bmo-markup` depends only on `bmo-policy`, so once `bmo-policy` is retired `bmo-markup`
depends on nothing internal and may be a base-library-style dependency; left to that crate's own
job to confirm, not decided here).

`scripts/html-read.ts` (`bun scripts/html-read.ts <file> --pictures <dir>`, docs/archive.md 「An HTML
page」 「Reading it」) has no Rust twin and is ported into this repository as a binary
(`src/bin/html-read.rs`), since it is exactly this unit's own operation — reading a page BMO already
filed, for BMO to learn what it says.

`repo/repo/snapshot-backup.ts` is **excluded**: see §6, backup scope.

## 3. Console plugin bundle

Per `#301`'s section on which units stand as their own service and which do not, a service's
screen is its own plugin bundle, not code living inside `bmo-web`'s client. Stage-0-2 found 57 files on the graph node `console/archive-screen`
plus the 13 test files and 3 shared modules listed below; all of them move into this repository as
its plugin bundle, not into bmo-web.

### 3a. Taken into this repository (`client/` plugin bundle)

- Pages (3): `pages/archive/index.vue`, `pages/archive/[id].vue`, `pages/share/[token].vue`
- Components (11): `ArchiveBody.vue`, `ArchiveSources.vue`, `ArchiveShareDialog.vue`,
  `ArchiveAttachments.vue`, `ArchiveSourceItem.vue`, `ArchiveTextPreview.vue`, `ArchiveRowSkeleton.vue`,
  `ArchivePageFrame.vue`, `HostedPageFrame.vue`, `DocumentPictureViewer.vue`, `ShareLinkDialog.vue`
- Domain/logic (3): `domain/archive.ts`, `domain/share-links.ts`, `domain/document-pictures.ts`
- Shared, specific to documents (3): `shared/document-pictures.ts`, `shared/share-head.ts`,
  `shared/document-design.ts` — these are not generic UI, they are this unit's own document
  rendering rules (the palette, the picture viewer contract, the OpenGraph head), so they move here
  rather than staying in bmo-web's shared folder.
- Tests (13): the colocated tests for the above, moved into `tests/client/`.

### 3b. Server routes and utils: reimplemented as this repository's own Rust HTTP API, not copied as Nuxt routes

The 15 Nuxt server routes and 8 server utils stage-0-2 found (`client/server/api/archive/*`,
`client/server/api/share/*`, `searches.ts`, `search-shares.ts`, `hosted-page.ts`, `card-picture.ts`,
`page-bridges.ts`, `moved-screens.ts`) were a BFF layer proxying into the TS archive facade and the
database directly. Once `bmo-web`'s own server is Rust (sumida-shaped, per `#301` 「web: bmo-web」)
there is no Nuxt server layer to hold a BFF. These routes' *behaviour* is what `docs/api.md` §1–§8
specifies as this repository's own HTTP API, served directly by this repository's binary; the plugin
bundle's Vue pages call that API directly (or through bmo-web's thin reverse proxy, which is
bmo-web's concern, not this repository's). No Nuxt `.ts` route file is copied; `moved-screens.ts`
(the `/searches` → `/archive` redirect) becomes a route alias declared in this plugin bundle's own
router config.

### 3c. Depended on, not copied

`client/shared` imports that are genuinely generic UI (not document-specific) — `SEARCHES_PAGE_SIZE`,
`SEARCHES_PHRASE_LIMIT`, `searchTitle`, `SHARE_SPANS`, `HOSTED_PAGE_SANDBOX`, the `SearchPage` /
`SearchSummary` / `SearchDetail` / `ArchiveAttachmentView` / `SearchFindingView` /
`DocumentPicture` / `PictureRun` types, and `matchFields` / `searchPhrase` / `hitSnippet` /
`searchHits` — are specific to this unit's own documents even though they sat in bmo-web's shared
folder before. They move into this repository's plugin bundle alongside §3a, since no other unit
reads them (per `#301`, a value only one unit reads does not stay in a shared location). The
UI primitives (`UDashboardPanel`, `UDashboardNavbar`, `UInput`, `UButton`, `USelect`, `UIcon`,
`UBadge`, `DeployMark`, `TodoOpenButton`, `ContentSkeleton`, `SearchText`, `SearchHit`) are kept as
a dependency on `bmo-ui`, the base UI library, which this repository may depend on directly.

### 3d. Language-pack keys

The `archive.*` (25 keys) and the archive-relevant `common.*` keys stage-0-2 found stay out of this
repository entirely. Per `#301`'s language-pack section, `lang/` lives only in `bmo-web`; this
repository returns English messages and status codes, and `bmo-web`'s language pack is the one
place these human-facing strings are declared and rendered.

## 4. Tables this repository owns

Resolving the disputed set the work order named, against `#301`'s own artifact section and its
addendum: the document archive's own tables come here; every other table goes to the
service that owns the chat/job/account concept it hangs on.

| Table | Owner | Why |
|---|---|---|
| `searches` | **bmo-artifact** | the document row itself, every kind |
| `search_findings` | **bmo-artifact** | one row per finding, FK to `searches` |
| `search_sources` | **bmo-artifact** | one row per source, FK to `search_findings` |
| `search_images` | **bmo-artifact** | legacy picture table, folded forward into `archive_attachments` on open, per `docs/archive.md` |
| `archive_attachments` | **bmo-artifact** | one row per attached file, FK to `searches` |
| `web_search_shares` | **bmo-artifact** | the document's own share-link table, FK to `searches` |
| `document_translations` | **bmo-artifact** | `document` FK is `searches.id` (confirmed, stage-0-3); a document's translated text is the document's own data |
| `document_talk` | **bmo-artifact** | `document` FK is `searches.id` (confirmed, stage-0-3); per-document conversation grounded in that document, not a chat-room table |
| `collected_items` | bmo-receiver | keyed by `chat_id`/`message_id`, not by any document kind in `docs/archive.md` 「Kinds」; this is chat-scoped material collection, not an archive document, so it stays with the unit that owns messages and rooms |
| `news_briefs` | bmo-voice | a per-chat *schedule* for when a voice briefing runs (`send_at`, `lead_minutes`, `enabled`), not a document; it configures bmo-voice's briefing synthesis. When a briefing produces a document, bmo-voice calls this repository's document-create API exactly as `bmo archive doc` does today |
| `voice_briefings` | bmo-voice | run-associated narration audio, matching bmo-voice's voice-synthesis role per `#301`; when a narration is attached to a document, the finished sound file arrives here as an ordinary `archive_attachments` row through this repository's own API, same as any other attachment |
| `page_read_failures` | bmo-explorer | `via`/`run` describe a page-reading attempt, the same mechanism `docs/archive.md`'s section on the tools a search reads before it searches assigns to the site-tools catalogue, not to the document archive; a failed import surfaces to this repository only as an error from bmo-explorer's read API, which this repository reports to the room, and the `todo_id` row is raised through bmo-job's API by whichever unit owns the read, not written here |
| `web_file_shares` | bmo-drive | `#301`'s console feature-services section names file-share links as bmo-drive's (`docs/files.md`, file-share), distinct from this table's near-namesake `web_search_shares` |

Commands: schema line numbers from `archive/bmo-archive/src/migration/archive.sql` as read by
stage-0-3; FK confirmation for `document_translations`/`document_talk` from the same stage's grep of
`archive.sql`.

## 5. Data folders this repository owns

| Folder | Size | Owner | Why |
|---|---|---|---|
| `searches/` | 435 MB | **bmo-artifact** | document markdown, HTML pages, images and attachments, named by document id |
| `collected/` | 5.2 GB | bmo-receiver | `stored_path` of `collected_items`, excluded per §4 |
| `news-brief/` | 1.5 MB | bmo-voice | audio/script storage for `news_briefs`/`voice_briefings`, excluded per §4 |
| `archive-copies/` | 3.4 GB | none — not carried forward | pre-Postgres-migration source copies; `docs/archive-backup.md` already names these as transition artifacts excluded from backups, and the new repository starts with no migration history to copy from |
| `archive-return/` | 3.5 GB | none — not carried forward | same transition basis as `archive-copies/` |

## 6. Backup scope

`bmo-snapshot-backup` and `bmo-restore-drill` are **not copied into this repository**. Neither
`repo/repo/snapshot-backup.ts` nor any Rust equivalent moves here. Reasoning: `#301` lists no
top-level folder for a backup service, and the backup/restore mechanism runs over the whole data
root (every unit's tables and files), not this unit's alone — it is a general facility, and
`#301`'s policy section assigns a policy file to the service that owns what it governs only where
the policy is itself unit-specific. What *is* this repository's own job is to state, in `docs/layout.md`'s
deploy declaration, which local paths (`searches/`) and which tables (§4's bmo-artifact rows) must
be included in any backup manifest — the same "neutral shape the testbed can read without knowing
this repository" the deploy declaration already exists to provide. The backup service itself is a
bmo-testbed-era concern to place in its own repository's build.

## 7. Defects found on these paths (plain facts, not fixed by this document)

1. `policy/bmo-policy/src/deployment.rs:8,11` — `AGENT_NAME` and `OWNER_NAME` are hardcoded
   constants holding, respectively, the running agent's own name and the console owner's own name
   as literal text. Per `#301`'s principles section these names are agent-registration data,
   never code. **Resolution for this repository:** `bmo-policy` is not depended on at all (§1), so this defect
   does not travel here; it is `bmo-policy`'s own repository's defect to fix, not re-created in this
   one. Noted so the build-list does not drop it.
2. `bot/bmo-server/src/verbs/archive_docs.rs:352,366` — error messages interpolate `OWNER_NAME`
   directly into English text returned to a room (`"...this is {OWNER_NAME}'s private chat..."`).
   **Build-list item for this repository:** when `archive_docs.rs` is ported to
   `src/controller/document.rs` (§1), these two messages are rewritten to a generic phrase — "the
   console owner's private chat" or an equivalent that names no person — with no constant
   substitution, matching `#301`'s rule that comments and messages are read the same by anyone who
   installs this repository.
3. `archive/archive/nodes.ts:97` — `HUB_SEED` hardcodes `tokenFile: "~/.local/share/jake/hub.env"`,
   a personal path under a different account name than this deployment's own. This file is excluded
   from this repository entirely (§2c/§2a — `nodes.ts` is node-infrastructure, not a document-table
   facade), so the defect does not travel here; it belongs to whichever repository inherits node
   hub-seeding (`bmo-node`, per `#301`'s table).
4. `bot/bmo-server/src/verbs/snapshot.rs:501` — hardcodes `.local/share/bmo/` into a named root. Not
   copied here (§6 — the snapshot binary is not carried into this repository at all).
5. Stage-0-0 and stage-0-2 report no code defects inside the Rust and console files that *are*
   taken into this repository. The three defects above all sit in files this repository excludes;
   none remain to fix inside what is actually copied.
