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
  - With no `--chat` and no `--by`, the room and the author come from settings. Where settings name
    no default room, a call with no `--chat` is refused.
  - Also run as `archive-doc`.
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
  - Also run as `archive-share`.
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

### Schema (CLI)

- `bmo-artifact-cli migrate` brings the database the settings name up to this repository's schema,
  and answers with how many migrations it ran. A database that is already up to date is left as it
  is.
- Callers: the deployment, and the testbed when it makes the test database's template.

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

- Sentences a room reads (a filed document's reply, a search's notices) and the lines a document
  says of itself come from the document wording file.
- Its path is a setting. Where none is set, the English default that ships with this repository is
  read.
- bmo-web's language pack supplies the live wording under the same keys. A file holds every key
  below and no other. The server does not start, and no verb runs, on a file that cannot be read,
  lacks a key or holds one more; the refusal names the file and the key.
- Each line is filled in by its `{name}` marks. A mark nothing is named for stays as written.
- The time zone those lines state a time in is a setting, by its IANA name. A document's moment is
  told in it on a twelve-hour clock.

A key is its groups joined by dots. The marks are those its line is filled in with.

| Key | Marks | What the line says |
|---|---|---|
| `archive_docs.header.kind` | `{kind}` | A filed document's kind, by its label under `archive_docs.kinds` |
| `archive_docs.header.request` | `{request}` | What the document was written for |
| `archive_docs.header.written` | `{time}` | When it was written, as `archive_docs.time.line` tells it |
| `archive_docs.header.attached` | `{names}` | The files it keeps, where it keeps any |
| `archive_docs.kinds.record` | none | What a record is called |
| `archive_docs.kinds.learning` | none | What learning material is called |
| `archive_docs.kinds.import` | none | What a page brought in whole is called |
| `archive_docs.kinds.page` | none | What an HTML page is called |
| `archive_docs.time.line` | `{year}`, `{month}`, `{day}`, `{half}`, `{hour}`, `{minute}` | A moment, with the hour on a twelve-hour clock |
| `archive_docs.time.morning` | none | The `{half}` before noon |
| `archive_docs.time.afternoon` | none | The `{half}` from noon |
| `search.archive_name` | none | What the console's collection of documents is called |
| `search.import_query` | `{source}` | The question a page brought in whole stands under |
| `search.head.untitled` | `{query}` | A search document's title where the result gave none |
| `search.head.query` | `{value}` | The head's question line |
| `search.head.topic` | `{value}` | The head's topic line |
| `search.head.keywords` | `{value}` | The head's keywords line |
| `search.head.rule` | `{value}` | The rule the search applied |
| `search.head.option` | `{name}`, `{value}` | One option the search was given |
| `search.head.depth` | `{value}` | The search's depth |
| `search.head.status` | `{value}` | The search's status |
| `search.head.started` | `{value}` | When it started |
| `search.head.finished` | `{value}` | When it finished |
| `search.head.runs` | `{value}` | The runs it took |
| `search.result.heading` | none | The result's heading |
| `search.result.empty` | none | What stands under it while nothing is written |
| `search.result.branch` | `{branch}` | The branch a finding came out of |
| `search.result.sources` | none | The heading of the pages the result cites |
| `search.trail.heading` | none | The heading of the trail folded under the result |
| `search.trail.summary` | none | The line that opens the trail |
| `search.trail.own` | none | The run's own section |
| `search.trail.run` | `{run}` | The run a branch was opened in, where the search took several |
| `search.trail.asked` | none | What one branch asked |
| `search.trail.opened` | none | The pages one branch opened |
| `search.trail.found` | none | What one branch found |
| `search.parts.VERDICT` | none | The short answer part of a result's body |
| `search.parts.SUMMARY` | none | The summary part |
| `search.parts.INTRO` | none | The introduction part |
| `search.parts.BODY` | none | The body part |
| `search.parts.CONCLUSION` | none | The conclusion part |
| `search.parts.UNCONFIRMED` | none | What could not be confirmed |
| `search.parts.MORE` | none | The part in detail |
| `search.fields.ADDRESS` | `{value}` | An entry's address line |
| `search.fields.MAP` | `{value}` | An entry's map line |
| `search.fields.ROADVIEW` | `{value}` | An entry's street view line |
| `search.fields.MENU` | `{value}` | An entry's menu line |
| `search.fields.menu_alone` | none | A menu line with the menu under it |
| `search.fields.RELEASED` | `{value}` | When a product was released |
| `search.fields.NEWER` | `{value}` | A newer model of it |
| `search.fields.STOCK` | `{value}` | Its state, when it was read and where, joined |
| `search.fields.deal` | `{now}`, `{usual}`, `{real}` | A deal: its price now, then the two pieces below |
| `search.fields.deal_usual` | `{usual}` | A deal's usual price, where given |
| `search.fields.deal_real` | `{real}` | The real word on a deal, where given |
| `search.fields.review_link` | `{page}` | The link to the page a review stands on |
| `search.fields.no_rating` | none | The rating written where there is none |
| `search.fields.original` | `{address}` | Where a picture kept as the archive's own copy came from |
| `search.import.answered` | none | That a page was brought in whole |
| `search.import.topic` | `{host}` | Its topic, by the page's host |
| `search.import.missing` | `{n}`, `{caption}` | A figure that could not be captured |
| `search.import.kept` | none | The source line of the page kept |
| `search.import.moved` | none | The source line of the address asked, which moved to the page kept |
| `search.import.cited` | none | The source line of a work the page cites |
| `search.import.cited_text` | `{text}` | The same, with the work's own words |

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

Each unit is reached at the address its name holds under `[units]` in the settings.

- **Start a briefing of a finished document** (bmo-voice): `POST /briefings` with a JSON body.
  - The body carries:
    - `chat_id`, the room it is spoken to;
    - `text`, the script;
    - `started_by`, which names the verb and the document, as `archive-doc:<id>`;
    - `document`, the document whose sound it is kept as.
  - A `2xx` answer carries `unit`, the name of the run that speaks it.
  - Any other answer is a refusal and carries its sentence under `error`.
  - The filing verb passes that sentence on, after `voice: `.
  - A call that gets no answer is reported with the failure's own words.
