# Boundary API: bmo-artifact

This is the whole surface this repository exposes to the rest of bmo, and the whole surface it
calls on others. Built from Trace artifact consumers (stage-0-4 of this item), grouped by
operation rather than by old file, since the old files (Nuxt BFF routes, `bmo-server` verbs) do not
survive as such — see `docs/inventory.md` §3b.

## Conventions

- **Transport:** HTTP, served by this repository's own binary. One CLI verb exists only where an
  agent calls this API directly instead of through HTTP — `bmo archive doc` and `bmo archive
  share` keep their shape as CLI verbs of this repository's own thin binary, which is itself an
  HTTP client of the routes below.
- **Caller identity:** every request carries the caller's account id, set by whichever unit
  authenticated the human or the agent (`bmo-web`'s login, or the agent session's own account) —
  this repository does not perform login itself.
- **Ownership rule:** every read and every list is filtered to documents the caller's account owns.
  A document's `owner_account_id` is the account of the room that asked for it, or the console
  owner's account for a document BMO wrote with no asking room. A request naming a document id the
  caller's account does not own answers `404 not_found`, the same answer a non-existent id gets, so
  a caller cannot learn that a document exists by probing ids it does not own.
- **Messages:** every response body on an error is `{ "error": "<english message>", "code":
  "<snake_case code>" }`. No Korean, no agent name, no person's name in any message or code —
  `docs/inventory.md` §7 item 2 is the one place this rule's old violation is tracked.
- **Status codes:** `200` read ok, `201` created, `204` deleted with no body, `400` malformed
  input, `404` not found or not owned, `409` a conflicting write (e.g. `--replace` on a document
  this repository did not write), `413` an attachment or narration over its ceiling, `422` a
  refusal this repository's own rules make (empty body, wrong kind for the verb), `502` a call to
  another unit failed.

## 1. Create a document

- **HTTP:** `POST /v1/documents`
- **CLI:** `bmo archive doc --title <title> [--kind record|learning|import] [--body <file>]
  [--attach <path>]... [--html <file>] [--narration <file>] [--share-days <n>|--share-forever]
  [--replace <id>] [--room <roomId>] [--by <accountId>]`
- **Payload:** `{ title, kind: "record"|"learning"|"import"|"page", body?, attachments?: [...],
  html_file?, narration_file?, share: { days?: number, forever?: boolean }, room_id?,
  by_account_id?, replace_document_id? }`. `room_id`/`by_account_id` default to the console owner's
  own account when neither is given, matching the old verb's default.
- **Response `201`:** `{ document_id, address: { console_url, share_url? }, share: { url?,
  expires_at? } }`.
- **Refusals (`422`):** no title; empty body; a kind this verb does not write; `page` named as
  `kind` instead of given through `html_file`; a page given a body, attachments or a narration; a
  `replace_document_id` for a document this repository did not write, or that does not exist; a
  file this document already keeps, handed back to it; a file unreadable or over the attachment
  ceiling (`413`); a narration unreadable or empty.
- **Called by:** any unit that wants a result "put in order and kept" shown to a person —
  `bmo-job` (a document a job shows the owner), `bmo-voice` (a finished briefing's document; the
  `--narration` path is also how this repository itself attaches an already-synthesized sound,
  see §9).
- **Consumer units named by the old trace:** `bot/bot/worker-english.ts`, `bot/bot/code-settle.ts`,
  `bot/bot/agents.ts`, `scripts/setup/archive-roles.ts`, `jobs/jobs/work-order.ts`.

## 2. Create or read a share link for an existing document

- **HTTP:** `POST /v1/documents/{id}/share` (create or renew), `GET /v1/documents/{id}/share`
  (read the current link), `DELETE /v1/documents/{id}/share` (remove it)
- **CLI:** `bmo archive share <id> --room <roomId> [--days <n>|--forever]`
- **Payload (`POST`):** `{ days?: number, forever?: boolean }`.
- **Response:** `{ url, expires_at }` or `{ url: null, reason: "<english reason>" }` for a document
  with no body to share (§3 below), or for the console owner's own room, which never gets a share
  link and is told the console address instead.
- **Refusal (`404`):** a document id that names nothing, or a search that kept none.
- **Table:** `web_search_shares` (`docs/inventory.md` §4) — not `web_file_shares`, which is
  bmo-drive's.

## 3. List documents

- **HTTP:** `GET /v1/documents?kind=&phrase=&shared_only=&cursor=`
- **Response `200`:** `{ items: [{ id, title, kind, topic, keywords, rule, state, started_at,
  ended_at, shared: bool }], next_cursor? }`.
- **Rule:** a search that left no document (`docs/archive.md` 「A search that leaves no document」)
  is never in this list.
- **Consumers:** the console plugin bundle's `pages/archive/index.vue` (`docs/inventory.md` §3a).

## 4. Read one document

- **HTTP:** `GET /v1/documents/{id}`
- **Response `200`:** the row's head (question, topic, keywords, rule, options, state, timing),
  plus the outline its kind module builds (`docs/inventory.md` §2b, the five kind modules) — the
  table of contents entries, whether a summary stands, which head properties show.
- **Kind `page`:** this same route also returns `{ page: true, html_url }`; the body fields above
  are empty, matching `docs/archive.md` 「An HTML page」 — "no summary and no contents, since the
  page is read as its author drew it."

## 5. Read a document's file, attachments and pictures

- **HTTP:** `GET /v1/documents/{id}/file` (the markdown or HTML file as kept), `GET
  /v1/documents/{id}/attachments/{attachment_id}` (one attachment's bytes), `GET
  /v1/documents/{id}/refs/{ref}/preview`, `GET /v1/documents/{id}/refs/{ref}/image` (inline
  reference previews the body's markdown points at by name, `docs/archive.md` 「Where it stands in
  the document」).
- **Bound:** every address stays inside that document's own folder; no route takes a path that
  reaches another document's files.

## 6. Read, create and remove a document via a share token (no account)

- **HTTP:** `GET /v1/share/{token}` (resolves to the same shape as §4), `GET
  /v1/share/{token}/attachments/{attachment_id}`, `GET /v1/share/{token}/head` (the OpenGraph/
  Twitter head, `docs/archive.md` 「Where a person reads it」), `GET /v1/share/{token}/picture`
  (the JPEG made from the kept AVIF for link-preview crawlers).
- **No caller identity required** — a token stands in for ownership, exactly as `docs/archive.md`
  describes a share link doing. A token past its `expires_at`, or one that was removed, answers
  `404`.

## 7. Translate a document

- **HTTP:** `GET /v1/documents/{id}/translation?target=&part=`, `POST
  /v1/documents/{id}/translation` with `{ target, part, text, failed: bool }`
- **Table:** `document_translations`, owned here per `docs/inventory.md` §4.
- **Consumer named by the old trace:** `bot/bot/worker-english.ts`'s `recalledWorkerEnglish()` —
  this becomes a caller of this HTTP route rather than a direct table read; which account/unit is
  allowed to write a translation for which document is this route's own check, not something the
  caller enforces on its side.

## 8. Talk about a document

- **HTTP:** `GET /v1/documents/{id}/talk`, `POST /v1/documents/{id}/talk` with `{ speaker, text,
  passage?, heading?, version }`
- **Table:** `document_talk`, owned here per `docs/inventory.md` §4.

## 9. Search within documents

- **HTTP:** `GET /v1/documents/search?q=`, `POST /v1/documents/seek` with `{ q, document_id? }`
  for a narrower, single-document seek.
- **Response:** matches with the snippet and the hit positions, the same shape
  `client/shared/search.ts`'s `matchFields`/`searchPhrase`/`hitSnippet`/`searchHits` produce today
  (`docs/inventory.md` §3c) — this route is the server-side computation those client functions
  were duplicating against rows fetched directly; the client functions now only render what this
  route returns.

## What this repository itself calls

Per `#301`'s principles section, only `bmo-chat`, `bmo-chat-client` and `bmo-ui` may be a direct
dependency; every other unit is reached by a standard API call, never an import, a shared path, or
a shared table.

| Call | To | Why |
|---|---|---|
| Start an agent session, send it the search task, read its output, abort/fold/end it | **bmo-agency**, agent-session API | the search run is an agent session; `#301`'s agency section names exactly these verbs as the abstraction over herdr this repository must use instead of calling herdr itself |
| Run a web search, open and read a page, run a browse step | **bmo-explorer**, the search/browse tool, through the standard tool-host call | `#301`'s section on units that are not their own service names bmo-explorer as the search-and-browse tool; this repository's runner supervises a run and writes the document the run's output describes, but does not fetch pages itself — see `docs/inventory.md` §1's note on `verbs/search/web.rs`, which is rewritten to make this call instead of importing `web-layer` directly |
| Who sent the message that asked, and when | **bmo-receiver**, message-read API | `docs/archive.md`'s restaurant rule reads "that message's sender and its time from the archive"; once messages and rooms are bmo-receiver's own tables (`#301`'s artifact section and its addendum), this repository asks bmo-receiver's API for the same facts instead of reading the row directly |
| Synthesize a narration's audio | **bmo-voice**, briefing-synthesis API | `docs/archive.md`'s section on a narration — this repository hands the script to bmo-voice and receives the finished sound file back, which it then stores as an ordinary `archive_attachments` row through its own write path (§1), not by writing into bmo-voice's own tables |
| Read a worker's co-worker pool, if a document-writing step ever needs one | **bmo-agency**, co-worker API | `#301`'s agency section names the co-worker pool as bmo-agency's to supply; this repository does not manage one itself |

No call in this table is a crate import, a shared database connection, or a shared filesystem path.
Everything above is HTTP or the tool-host's standard call shape.
