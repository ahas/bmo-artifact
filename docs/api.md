# Boundary API: bmo-artifact

This document is the whole surface bmo-artifact exposes and the whole surface it calls. Other
repositories match against it, and so does the switch-over that points the console and the other
units at this service. Phase 1 keeps today's behaviour: every path, flag, field and answer below
mirrors what the old bmo serves today, so the two can be compared side by side.

## Conventions

- **Transport.** The service serves HTTP. The CLI verbs are this repository's `bmo-artifact-cli`
  binary. After the binary's name, an agent runs each verb exactly as it runs the old one.
- **CLI answers.** A verb prints its answer on standard output and ends with success. A refusal is
  printed on standard error and ends with failure. A verb that did part of what it was asked prints
  that part on standard output and the refusal of the rest on standard error, and ends with failure.
  A verb that ends with a code of its own ends with that code.
- **Visibility.** A console caller, signed in through bmo-web, sees every document. A share token
  opens exactly the one document it was made for, without signing in. Per-account ownership is not
  part of this phase.
- **Errors.** An error body is a JSON object carrying the English message under `error` and the
  snake_case code under `code`. No message or code holds Korean, a person's name or an agent's name.
  Wording a person reads in a room comes from the document wording file (see "Wording").
- **Status codes.**
  - `200`: read
  - `201`: created
  - `204`: removed
  - `400`: malformed input
  - `404`: no such document, attachment or token, or a token that has expired or been removed
  - `413`: a file over its ceiling
  - `422`: a refusal by this repository's own rules
  - `502`: a call to another unit failed
- **Internal dependencies.** Only `bmo-chat`, `bmo-chat-client` and `bmo-ui` may be depended on
  directly. Every other unit is reached by the calls under "What this repository calls", never by an
  import, a shared table or a shared path.

## What this repository exposes

### Filing a document (CLI)

- `bmo archive doc [--chat <chatId>] [--by <userId>] --title '<title>' [--kind record|learning|import] [--body <file> | stdin] [--attach <path>]… [--html <file>] [--narration <file>] [--share-days <n> | --share-forever] [--replace <id>]`
  - Files a record, learning material or an import, or files an HTML page given as `--html`.
  - Attaches files and a narration.
  - Answers with the document's number and the address the room may open: the console address in a
    room that can open the console, a share link otherwise.
  - With no `--chat` and no `--by`, the room and the author come from settings.
  - Refusals:
    - no title, or an empty body;
    - a kind this verb does not write;
    - a page given a body, attachments or a narration;
    - `--replace` naming a document this verb did not write;
    - a file the document already keeps;
    - a file that cannot be read, or one over the ceiling;
    - an empty narration.
- `bmo archive share <id> --chat <chatId> [--days <n> | --forever]`
  - Makes or renews a document's share link for a room, and answers with the link and when it ends.
  - The owner's own private room, and a room that cannot open the console, are each given a plain
    reason that names no person.
- `bmo archive html-read <file> [--pictures <dir>] [--stem <name>] [--from <line>]`
  - Reads a filed HTML page for its words in order, cut at the answer ceiling; `--from` reads on
    from a numbered line.
  - Names each picture embedded as a data address after `picture`, or after `--stem`, and with
    `--pictures` writes each out as a file of its own.
  - Refuses a `--from` that is not a line number from 1, and a `--stem` that is not a short name.
- Callers:
  - bmo-job, for a document a job shows the owner;
  - bmo-voice, for a briefing's document and its narration;
  - any agent session that puts a result in order and keeps it.

### Searching (CLI)

- `bmo search --chat <chatId> --query <what to find out> [--message <messageId>] [--depth <depth>] [--demand explicit|optional] [--period <span>] [--region <region>] [--exclude <sources>] [--shape <shape>] [--into <searchId>]`
  starts a search.
- `bmo search --chat <chatId> --import <url> [--query <what was asked>]` imports a page.
- The run is queued through bmo-job. Its runner, `bmo runner search`, writes the document from the
  run's structure lines.
- `bmo search branch --owner <owner> --source <search|browse> --role <role> --chat <chatId> --index <n> [--closes-at <ms>] [--strategy <strategy>] [--host-interval-ms <ms>] [--engine-interval-ms <ms>]`
  - Runs one branch of a research run, reading the angle on stdin.
  - Holds a place in the co-worker pool, and answers with the branch's notes.
  - It keeps accepting both `--source` values, so bmo-explorer's browse runs call it across the
    boundary exactly as the search line does.

### Research settings (HTTP)

- `GET /research-settings` reads every research profile: its values, its resolved provider and
  mapping, and its seed.
- `POST /research-settings` saves a profile's values or puts a profile back to its seed.
- A field that fails its check answers `422` with an English code per field.
- Callers: the console's agents screen in bmo-web, and bmo-explorer, which reads the browse profiles.

### Room holding (HTTP)

- `GET /rooms/{chatId}/holding` answers what this repository keeps for a room: its documents, which
  are kept when the room is removed, never deleted with it.
- Caller: bmo-receiver, before it removes a room.

### Documents (HTTP)

- `GET /api/archive?kind=&q=&shared=1&before=&limit=`
  - Lists documents, newest first.
  - Filters by kind, by a phrase (its length capped) and by shared-only.
  - Answers with a cursor for the next page.
  - A search that left no document is never listed.
- `GET /api/archive/{id}` reads one document: its head (question, topic, keywords, rule, options,
  state, timing), the outline its kind builds, its findings, sources, attachments and pictures, and
  its share state. A page answers with no summary and no contents.
- `GET /api/archive/{id}/document` serves the document's file.
  - A markdown document is served as kept.
  - An HTML page is served as a hosted page: sandboxed, with its link and picture bridges and the
    reader's colour scheme.
- `GET /api/archive/{id}/attachments/{attachment}` serves one attachment's bytes.
- `GET /api/archive/{id}/refs/{ref}/preview` and `GET /api/archive/{id}/refs/{ref}/image` serve
  the preview and the picture of a source the body points at.
- Moved screens:
  - `/searches` and `/searches/<rest>` answer with the matching `/archive` address, query kept.
  - `/collected` answers with `/archive`.
- Every address stays inside its own document's folder.
- Caller: the console plugin bundle in this repository's `client/`.

### Share links (HTTP)

- `GET /api/archive/{id}/share` reads a document's link.
- `POST /api/archive/{id}/share` makes or renews it for a span.
- `DELETE /api/archive/{id}/share` removes it.
- `GET /api/share/{token}` reads the shared document in the same shape as the document read.
- `GET /api/share/{token}/refs/{ref}/preview` and `GET /api/share/{token}/refs/{ref}/image` serve
  a shared document's source previews and pictures.
- `GET /api/share/{token}/attachments/{attachment}` serves a shared document's attachment.
- `GET /api/share/{token}/head` serves the link preview head: title, description, picture and the
  robots rule.
- `GET /api/share/{token}/picture` serves the JPEG card picture made from the kept picture, for
  link-preview crawlers.
- Expired links are swept by the service itself.
- Table: `web_search_shares`, not bmo-drive's `web_file_shares`.

### Wording

- Sentences a room reads (a filed document's reply, a search's notices) come from the document
  wording file.
- Its path is a setting, and an English default ships with this repository.
- bmo-web's language pack supplies the live wording under the same keys.
- The time zone used in those sentences is a setting.

## What this repository calls

Each call is HTTP or the tool host's standard call shape. bmo-receiver also reads this repository's
room-holding answer (see "Room holding") before it removes a room.

| Purpose | Unit |
|---|---|
| Queue a search run, mark it stopped, read and set its live task names, read the instruction line | bmo-job |
| Import a page, fetch a raster picture, check an address, pace requests per host and engine, stop a remote browse | bmo-explorer, through the tool host |
| Read the search-engine values a research profile chooses among | bmo-explorer, through the tool host |
| Start a briefing of a finished document | bmo-voice |
| Who sent the message that asked, and when | bmo-receiver |
| Start, drive, read, stop and end the research session | bmo-agency |
| Read and apply tiers, providers and their order and enablement | bmo-agency |
| Take and give back places in the co-worker pool | bmo-agency |
| Read the accounts a run is offered | bmo-agency |
