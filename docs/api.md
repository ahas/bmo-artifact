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
  - `404`: no such document, attachment or token, or a token that has been removed
  - `409`: what is asked conflicts with what stands, such as a second share link
  - `410`: a share link whose time has passed
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

- `bmo search --chat <chatId> --query <what to find out> [--message <messageId>] [--depth <depth>] [--demand explicit|optional] [--period <span>] [--region <region>] [--exclude <sources>] [--shape <shape>] [--into <searchId>] [--by <who>] [--via <road>]`
  starts a search; the question comes on standard input where `--query` names none.
- `bmo search --chat <chatId> --import <url> [--query <what was asked>]` imports a page.
  - `--message` names the message the search answers; bmo-receiver says who sent it, when, and the
    road it came in on, which the result is answered on where `--via` names none.
  - `--by` names who set it off, and the variable `BMO_TURN_AGENT` the agent whose turn asked it.
  - Refused, with the usage line and no row opened anywhere: no room, no question, a depth that is not
    one of the three, a demand that is neither word, an option outside its values, and `--into`
    beside `--import`. `--into` naming no finished search of the room is refused before any row is
    opened.
  - Its last line on success: `search launched as <unit>; it answers chat <chatId> by itself when it is
    done. …`, after any note on an asker bmo-receiver does not know or a depth the pool lowered.
  - A run bmo-agency does not open or launch ends with failure: the search is put back as it stood, or
    closed `failed` where it was new; a run's row opened and never launched is closed `error`, and
    the question's file is taken off the disk. The refusal is `<unit> did not come up`.
- The search's row is opened here and the run's in bmo-agency's ledger, and bmo-agency launches its
  runner.
- `bmo runner search <chatId> <queryFile> [depth] [runId] [unit] [startedBy] [searchId] [demand] [profile] [--via <road>] [--kept-on <registration>] [--asked-by <agent>]`,
  also run as `search-runner`, is the run bmo-agency launches. It reads the question from the file and
  deletes it, carries the run, writes the document from the run's structure lines, and hands the
  ending back to the room's session, for the agent `--asked-by` names. It speaks to no room itself.
  - It answers with a line on how the run ended. A run that failed ends with failure and the reason.
  - A stop signal (`SIGTERM` or `SIGINT`) stops the run: what it had written in the result's
    structure is handed back as a stopped result, or, where it wrote nothing, the stop itself.
- `bmo search branch --owner <owner> --source <search|browse> --role <role> --chat <chatId> --index <n> [--closes-at <ms>] [--strategy <strategy>] [--host-interval-ms <ms>] [--engine-interval-ms <ms>]`
  - Runs one branch of a research run, reading the angle on stdin (`docs/research-runs.md`,
    "Branches are taken from the pool").
  - Holds a place in the co-worker pool, and answers with the branch's notes.
  - It keeps accepting both `--source` values, so bmo-explorer's browse runs call it across the
    boundary exactly as the search line does.
  - Also run as `research-branch`.
  - Every answer is one block on standard output, ending with success, so the parent's tool hands it
    to the run as it stands:
    - the notes, opening `BRANCH <n> REPORT` and closing `END OF BRANCH <n>`, with a last line saying
      the branch was stopped at research close where it was;
    - `RESEARCH CLOSED: …` where research had closed before the branch was asked;
    - `BRANCH REFUSED: …` where the pool could not take it, stayed full until research closed, or the
      run ended before it had a place or before it started;
    - `BRANCH <n> FAILED: …` where it failed.
  - An angle that cannot be read, and a line the verb does not take, are refused with failure.
  - A stop signal (`SIGTERM` or `SIGINT`) stops the branch, which gives its place back and answers
    with what it had written.

### The morning news page (CLI)

What the page is, when it runs and how it is made is `docs/news-brief.md`; these are its commands.

- `bmo news-brief` lists the rooms that get a page, one line to a room (its number, on or off, the
  room, the time it is sent, the lead, the hours its link lives and the day it last started), and how
  many holidays this year and the next hold. With no room it says so.
  - `--add --chat <chatId> [--at HH:MM] [--lead <minutes>] [--share-hours <n>] [--on]` adds a room,
    off unless `--on` says otherwise; the defaults are 08:30, 40 minutes and 24 hours. It answers
    with the row.
  - `--set <id> [--chat <chatId>] [--at HH:MM] [--lead <minutes>] [--share-hours <n>]` changes the
    fields named; `--on <id>` and `--off <id>` switch a room; `--remove <id>` takes it away. Each
    answers with the row as it stands, or as it stood for a removal.
  - `--run <id> [--send-to <chatId>]` opens a run's row in bmo-agency's ledger, has bmo-agency
    launch `runner news-brief` for that room's page of today, sent to `--send-to` where one is named,
    and answers with the run's number and its unit. A launch that did not come up closes the row
    `error` and ends with failure, with bmo-agency's sentence.
  - `--holidays [<year>]` lists the days off of a year, this year's where none is named;
    `--holiday-add YYYY-MM-DD '<name>'` adds a day or renames it, its name trimmed;
    `--holiday-remove YYYY-MM-DD` takes it off.
  - Refused, with nothing written: a room not named by its chat id, a time not written `HH:MM`, a
    lead outside 10 to 180 minutes, share hours outside 1 to 720, a row that does not exist, a day
    that is not a calendar date, and a holiday with no name.
- `bmo runner news-brief --chat <chatId> [--send-to <chatId>] [--day YYYY-MM-DD] [--share-hours <n>] [--send-at HH:MM] [--run <runId>] [--unit <unit>] [--answer <file>] [--no-send]`,
  also run as `news-brief-runner`, is the run bmo-agency launches.
  - `--unit` names the run's unit, which its writer's lines are logged under.
  - `--day` is today's in the settings' time zone where none is named, and `--share-hours` 24.
  - It writes the page through a research run apart, files it as a page document of `--chat` with a
    share link living `--share-hours`, holds the message until `--send-at`, and sends it to
    `--send-to`, or to `--chat` where none is named.
  - `--answer` draws the page from a writer's answer already written; `--no-send` stops once the
    page file is drawn.
  - `--run` names the row in bmo-agency's ledger it closes, `done` where the page went out and
    `failed` otherwise.
  - It answers with the page's title, the room and the link. A run that failed tells the owner,
    sends nothing, and ends with failure: `news-brief: FAILED — <reason>`.
  - A call that does not read is refused with its usage line.
- Callers: the service's own minute loop for `--run`'s launch, and an agent session for the rest.

### Schema (CLI)

- `bmo-artifact-cli migrate` brings the database the settings name up to this repository's schema,
  and answers with how many migrations it ran. A database that is already up to date is left as it
  is.
- Callers: the deployment, and the testbed when it makes the test database's template.

### Research settings (HTTP)

The agents screen's view of the research profiles, the common co-worker pool and the search engines,
and every change it asks for (`docs/research-runs.md`, "Research profiles"). Its bodies keep the
field names the console reads today.

- `GET /research-settings` answers `200` with:
  - `readAt`, when it was read;
  - `pool`:
    - `roles`, each pool role as bmo-agency lists it (`name`, `description`, `enabled`, `pooled`,
      `control`), with its `tier` and `usedBy`, the keys whose branches take it;
    - `tiers`, one for each co-worker tier of each provider that lends co-workers and has a place in
      the pool's status: `provider`, `label`, `tier`, the `model` and `effort` the tier resolves to
      or the `error` why it does not, `capacity`, `defaultCapacity`, `snapshotCapacities`,
      `snapshotModels`, `active`, `reserved`, `waiting`, `yielding` (`active` and `waiting`),
      `roles` (the enabled roles serving it, the control role left out), `reservations` (each with
      `owner`, `source`, `role`, `wanted`, `held`, `waiting`, `createdAt` and `lastPing`) and the
      tier's `lastPing`;
    - `capacityLimit`, the most places a pool limit may hold;
    - `defaults`, each co-worker tier's default capacity, by tier;
    - `coworkerTiers`, the co-worker tiers in order;
  - `profiles`, one per research key in the settings' order: `key`, the eight values (`capMs`,
    `synthesisReserveMs`, `childRole`, `maxChildren`, `waitLimitMs`, `strategy`, `hostIntervalMs`,
    `engineIntervalMs`), `setAt`, `seed` (the eight values it starts from), `strategies`, the key's
    `tier`, the `provider`, `model` and `effort` it launches with or the `error` why it cannot, and
    `room`, where a run of it opens branches, saying where a new optional request would stand:
    `state` (`free`, `full`, `no-capacity`, `role-off` or `unresolved`), `provider`, `tier`, `model`,
    `capacity`, `active`, `waiting`, `free` and `error`;
  - `limits`, the bounds the form checks against: `capMs` (`min`, `max`), `synthesisReserveMs`
    (`min`), `researchMs` (`min`), `maxChildren` (`max`), `waitLimitMs` (`max`), `intervalMs`
    (`max`);
  - `engines`, each search engine bmo-explorer reports: `engine`, `state`, `reason`, `observedAt`,
    `suspendedUntil`, `elapsedMs`, `resultCount`, `category`, `korean` (whether it is native to Korean
    pages) and `resting` (suspended until a time still to come);
  - `cooldowns`, each a `reason` and its `ms`;
  - `enginesError`, where bmo-explorer could not be read: the sentence why, with `engines` and
    `cooldowns` left empty.
- `POST /research-settings` takes one change as a JSON object naming its `action`, and answers `200`
  with `ok: true`:
  - `set-pool-limit`, with `provider`, `tier` and `capacity`, sets a co-worker tier's limit on a
    provider;
  - `reset-pool-limits` puts every limit back to its default;
  - `set-profile`, with `key` and `values`, the eight values, saves a profile;
  - `reset-profile`, with `key`, puts a profile back to its seed.
- Refusals, each with its English sentence:

| Status | Code | When |
|---|---|---|
| `400` | `not_json` | The body is not JSON |
| `400` | `bad_action` | The body is not an object, or names no action this route takes |
| `400` | `bad_pool_limit` | The provider lends no co-workers or is not enabled, or the tier is not a co-worker tier |
| `400` | `bad_capacity` | The capacity is not a whole number from 0 to the most a limit may hold |
| `400` | `bad_key` | The key is not a research key in the settings |
| `400` | `no_values` | `set-profile` carries no values |
| `422` | `invalid_profile` | A value fails its check; `faults` names each (below) |
| as bmo-agency answered | as bmo-agency answered | bmo-agency refused a limit |
| `502` | `agency_failed` | bmo-agency could not be reached or answered out of its shape |

- **A field's faults** come in the form's order, each with `field` (the value's name above), `code`
  and `error`, the sentence the form shows. The refusal's own `error` is those sentences joined.

| Field | Code | When |
|---|---|---|
| `capMs` | `no_clock` | The key holds no clock and a total time is given |
| `capMs` | `cap_out_of_range` | The total time is not a whole number within its limits |
| `synthesisReserveMs` | `no_clock` | The key holds no clock and a wrap-up time is given |
| `synthesisReserveMs` | `reserve_too_short` | The wrap-up time is not a whole number of at least its least |
| `synthesisReserveMs` | `research_too_short` | The wrap-up time leaves less research than its least |
| `maxChildren` | `children_out_of_range` | The branch count is not a whole number from 0 to its most |
| `maxChildren` | `branchless` | The key opens no branches and the count is not 0 |
| `childRole` | `bad_role` | The role is not a role name |
| `childRole` | `role_not_pooled` | Branches are allowed and the role is not enabled in the pool |
| `waitLimitMs` | `wait_out_of_range` | The wait limit is not a whole number from 0 to its most |
| `strategy` | `bad_strategy` | The key does not accept the strategy |
| `hostIntervalMs`, `engineIntervalMs` | `interval_out_of_range` | The interval is not a whole number from 0 to its most |

- Callers: the console's agents screen in bmo-web, and bmo-explorer, which reads the browse profiles.

### Room holding (HTTP)

- `GET /rooms/{chatId}/holding` answers what this repository keeps for a room: its documents, which
  are kept when the room is removed, never deleted with it.
  - The answer is `200` with `kept`, one entry for each table here that holds a room's rows:
    - `table`, the table's name;
    - `rows`, how many of the room's rows it holds, none included;
    - `why`, the English sentence saying why those rows outlive the room.
  - Each table declares its own entry beside the code that writes it. `searches` is the one table
    naming a room, and its rows are the room's documents. Nothing this repository holds goes
    with a room, so the answer names nothing to remove.
  - A `chatId` that is not a whole number is refused with `400` and `bad_room`.
- Caller: bmo-receiver, before it removes a room.

### Documents (HTTP)

Times here are epoch seconds, as the archive's rows keep them. Field names are the ones the console
reads.

- `GET /api/archive?kind=&q=&shared=1&before=&limit=` answers `200` with a page of documents, newest
  first: `items` and `hasMore`.
  - `limit` is read as a whole number off the front of its text; one from 1 is held to 200, and
    anything else reads 30.
  - `before` is the number of the last row the caller has; the page ends just before it. One that is
    not a whole number is refused `400 bad_cursor`.
  - `shared=1`, and nothing else, holds the page to documents with a live share link: one with no end,
    or one whose end is still to come.
  - `kind` holds it to one kind of the catalogue; a word the catalogue does not know is no filter.
  - `q`, cut to 200 characters, holds it to documents saying the phrase, matched whole and without
    regard to case in the title, the question, the topic, the keywords, the rule, the room's name and
    the body. An empty or blank phrase is no search.
  - Each filter is done in the reading itself, so a filtered page is a full page.
  - A search that left no document is never listed.
  - An item: `id`, `chatId`, `room` (the room's name as bmo-receiver gives it, or null), `query`,
    `startedDepth`, `endedDepth`, `status`, `startedAt`, `finishedAt`, `findings` and `sources` (their
    counts), `title`, `topic`, `keywords` (a list), `rule`, `options` (an object of strings), `kind`,
    `author`, `documented` and `shared` (`expiresAt`, null for a link with no end; null for a document
    with no live link). A searched list adds `excerpt`: the body's words around the first hit, or null
    where only the head said the phrase.
- `GET /api/archive/kinds` answers `200` with `kinds`, the catalogue in its order, each `id`,
  `labelKey` (the key the console's language packs name its label by), `summary`, `hosted`,
  `searched` and `head` (the properties its head shows).
- `GET /api/archive/{id}` answers `200` with one document whole, or `404 document_not_found`; a number
  that is not a whole number from 1 is refused `400 bad_id`.
  - `search`: the item's fields, `shared` as the list reads it, `runs` (every run that carried it, as
    bmo-agency lists them, and none where it cannot) and `document` (whether its file is there to read).
  - `kind`: the document's kind as the catalogue answers it. A word the catalogue does not know reads
    as `search`.
  - `outline`: `entries`, the table of contents its kind makes, each `slug`, `text`, `depth` and
    `level` (`part`, `chapter`, `section` or null); and `marks`, the marks each finding's headings are
    stamped with, by the finding's place. A page has no entries and no marks.
  - `findings`: each `place`, `runId`, `branch`, `text`, `name`, `title`, `markdown`, `foldedMarkdown`,
    `images` (each `ref`, `url`, `caption`), `links` (each `ref`, `url`, `label`) and `sources` (each
    `ref`, `url`, `title`, `supports`, `mark`). A link that is one of the document's sources, or a
    picture's address, is not listed as a link. The label of a link from a map, street-view or review
    line is the wording file's.
    - `name` is the name the finding gave itself on its title or name line, null where it gave none.
    - `title` is what it is called as a line of its own: its name, else the part or the section it
      opens with in the wording file's words, else the first thing it says, cut to 60 characters,
      else the wording file's untitled line.
    - `markdown` is what stands open of its text, and `foldedMarkdown` what is folded under the line
      where its long explanation begins, null where nothing is; each is its structure lines turned
      into markdown in the wording file's words, the same reading the outline's marks are made from.
      Besides plain markdown, the reading writes three blocks of its own, each opening on a line of
      its own after a blank line: `::: fold <label>` and `::: gallery`, each closed by a line `:::`,
      holding a fold's body or a gallery's pictures, and `::: map <words>` alone, naming the place a
      map is to be drawn of. A footnote mark is a link to `#mark-<mark>`, the mark percent-encoded.
      A line the search itself wrote in a block's form stands escaped, so it opens no block.
  - `opened`: the pages it opened and cited under nothing, as sources.
  - `attachments`: each `id`, `media`, `type`, `bytes`, `name` and `drawn`, every file but a picture a
    result named by its address. `drawn` says whether the body already draws the file where it stands:
    true for a picture that a picture of the findings' markdown, open or folded, names by the name it is
    kept under, read the way CommonMark reads a picture and with its percent-encoding undone. A link
    naming a file leaves it undrawn, since a link is not the file shown, and so does a name inside a
    code block. The console stands an undrawn file in the attachment section.
- A ref names a place in a document's record: `s<id>` a source row, `i<place>.<n>` the `n`th picture
  and `l<place>.<n>` the `n`th link of the finding at `place`.
- `GET /api/archive/{id}/document` serves the document's file as it was written, to keep:
  `text/markdown`, named `search-<id>.md`, never cached. `404 document_not_found` where the document has
  no file inside its folder.
- `GET /api/archive/{id}/attachments/{attachment}` serves one file the document keeps, looked up under
  that document. `404 attachment_not_found` for a number that is not its; `400 bad_attachment_id` for
  one that is not a whole number from 1.
  - `download=1` sends it to keep, under its name.
  - A `Range` header is answered `206` with the stretch asked; one no file holds is answered with the
    whole file.
  - An HTML page is sent as a hosted page (see "An HTML page" in `docs/archive.md`) unless it is asked
    for to keep: its prelude ahead of its first element, in the scheme `scheme` names, else the one the
    console's cookie keeps, else dark. It goes out under the sandboxed page policy.
  - Every other file goes out under the policy that lets nothing on it run.
- `GET /api/archive/{id}/refs/{ref}/preview` answers `200` with what a source or a link shows of
  itself: `title`, `thumbnail` and `favicon` (whether it has each picture). `404 no_such_source` for a
  picture's ref and for a ref the record does not hold.
- `GET /api/archive/{id}/refs/{ref}/image?as=` sends a picture: the picture itself for a picture's ref,
  its kept copy first; `as=thumb` and `as=favicon` the page's picture and icon for a source or a link.
  It goes out as the type its bytes are, not sniffed and sandboxed. `404 image_unavailable` where there
  is none.
  - A preview and a picture are fetched through bmo-explorer (see "Fetch a page's head") and kept under
    the file root's `web-cache`: a preview for a week, a picture for a month, a failure for a day, and
    a fetch that ran out of time not at all.
- Moved screens, each answered `301` under the console address's own path, the query kept:
  - `/searches` and `/searches/` to `/archive`, and `/searches/<rest>` to `/archive/<rest>`;
  - `/collected` and `/collected/` to `/archive`.
- Every file is served only from inside its own document's folder, every link resolved.
- Caller: the console plugin bundle in this repository's `client/`, through bmo-web, which signs the
  caller in.

### Share links (HTTP)

A link is a row of `web_search_shares` (not bmo-drive's `web_file_shares`): `token`, `searchId`,
`createdBy`, `createdAt` and `expiresAt`, null for a link with no end. A document has at most one. Times
are epoch seconds. The share API and the `archive` verbs write the rows through one module.

Behind the console's sign-in, a document number that is not a whole number from 1 is refused
`400 bad_id` on each of these:

- `GET /api/archive/{id}/share` answers `200` with `share`: the live link, or null where there is none,
  which is also what a link whose time has passed reads as.
- `POST /api/archive/{id}/share` makes a link, taking `span` and `by`, and answers `200` with `share`.
  - `span` is one of `1h`, `6h`, `24h`, `7d`, `30d` and `forever`; the link lives that long from now,
    and `forever` makes one with no end. Any other span is refused `400 bad_expiry` before anything is
    read.
  - `by` is who makes it, which bmo-web names as the signed-in user; none is refused `400 bad_maker`.
  - `404 document_not_found` for no such document; `409 nothing_to_share` for a search that left no
    document; `409 already_shared` where a link already stands, which a new one never replaces.
- `DELETE /api/archive/{id}/share` takes the link away at once, row and all, and answers `200` with
  `removed`, whether there was one.

Under a link, with no sign-in. The token is the whole permission, and no document number is taken from
the request:

- A token not shaped as one issued here (22 to 128 letters, digits, `-` and `_`), or one naming no link,
  is refused `404 share_not_found`; one whose time has passed `410 share_expired`. Its expiry is read
  each time.
- `GET /api/share/{token}` answers `200` with the document the link opens, and nothing of the console:
  `title` (its title, or its question where it wrote none), `runCount`, `kind` (its kind's word),
  `kindEntry` (its kind as the catalogue answers it), `outline`, `findings`, `opened` and `attachments`,
  as the document read gives them. It is not to be stored.
- `GET /api/share/{token}/refs/{ref}/preview` and `GET /api/share/{token}/refs/{ref}/image?as=` answer
  as the console's own roads do, on the document the link opens.
- `GET /api/share/{token}/attachments/{attachment}` serves one file of that document as the console's
  road does, not to be stored.
- `GET /api/share/{token}/head` answers `200` with what the link reads as before it is opened, not to be
  stored:
  - `title`;
  - `description`: the body's opening prose as plain words, cut at 160 characters, or its topic where
    the body has none;
  - `image`: `api/share/<token>/picture` where the document has a picture, the first its body draws or
    else the first it keeps; null otherwise;
  - `declared`: for a page document, the head its page declares (`title`, `meta`, each a `name` or a
    `property` with its `content`, and `canonical`); null for every other.
- `GET /api/share/{token}/picture` sends that picture as a JPEG for a link's preview card: upright,
  fitted inside 1200 pixels, never enlarged, laid on white. `404 image_unavailable` where the document
  has none.
- Every read and every making sweeps away the links whose time has passed.

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
| `search.reading.address` | none | The label of an entry's address, as the console's reading draws a finding |
| `search.reading.map` | none | The label of a map link, and of a link a finding points at from its map line |
| `search.reading.street_view` | none | The label of a street view link, the same way |
| `search.reading.menu` | none | What a menu is folded under |
| `search.reading.dish` | none | A menu's first column |
| `search.reading.price` | none | A menu's price column |
| `search.reading.notes` | none | A menu's column for notes, where a dish has one |
| `search.reading.released` | none | The label of when a product was released |
| `search.reading.newer` | none | The label of a newer model |
| `search.reading.stock` | none | The label of a product's state |
| `search.reading.product_page` | none | The link to a product's page |
| `search.reading.deal` | none | The label of a deal |
| `search.reading.usual` | `{usual}` | What a deal stands against |
| `search.reading.review` | none | The link to the page a review stands on, and the label of a link a finding points at from its review |
| `search.reading.untitled` | `{place}` | What a finding with no words of its own is called in the contents |
| `search.import.answered` | none | That a page was brought in whole |
| `search.import.topic` | `{host}` | Its topic, by the page's host |
| `search.import.missing` | `{n}`, `{caption}` | A figure that could not be captured |
| `search.import.kept` | none | The source line of the page kept |
| `search.import.moved` | none | The source line of the address asked, which moved to the page kept |
| `search.import.cited` | none | The source line of a work the page cites |
| `search.import.cited_text` | `{text}` | The same, with the work's own words |
| `news_page.language` | none | The language tag the page declares, such as `en` |
| `news_page.weekdays.sunday` | none | Sunday, as the day a morning news page is for names its weekday |
| `news_page.weekdays.monday` | none | Monday, the same way |
| `news_page.weekdays.tuesday` | none | Tuesday, the same way |
| `news_page.weekdays.wednesday` | none | Wednesday, the same way |
| `news_page.weekdays.thursday` | none | Thursday, the same way |
| `news_page.weekdays.friday` | none | Friday, the same way |
| `news_page.weekdays.saturday` | none | Saturday, the same way |
| `news_page.day` | `{year}`, `{month}`, `{date}`, `{weekday}` | The day a page is for, the numbers without leading noughts |
| `news_page.title` | `{day}` | The page's title and its message's title line |
| `news_page.news` | none | The heading of the news cards |
| `news_page.korea_market` | none | The heading of the home stock market section |
| `news_page.us_market` | none | The heading of the US stock market section |
| `news_page.real_estate` | none | The heading of the home property market section |
| `news_page.no_close` | none | What an index the page could read no close of shows |
| `news_page.close_of` | `{date}` | The day an index's close is of, `{date}` as MM/DD |
| `news_page.chart_label` | `{name}`, `{first}`, `{last}`, `{count}`, `{low}`, `{high}` | What a chart is read out as |
| `news_page.original` | `{host}` | The link under a card to its article |
| `news_page.sources` | none | What stands before a section's links to its sources |
| `news_page.footer` | none | The page's closing line |
| `news_page.indices.KOSPI` | none | The KOSPI's name, by the key it is read under |
| `news_page.indices.KOSDAQ` | none | The KOSDAQ's name |
| `news_page.indices.DJI` | none | The Dow Jones Industrial Average's name |
| `news_page.indices.SPX` | none | The S&P 500's name |
| `news_page.indices.NASDAQ` | none | The Nasdaq Composite's name |

## What this repository calls

Each call is HTTP or the tool host's standard call shape. bmo-receiver also reads this repository's
room-holding answer (see "Room holding") before it removes a room.

| Purpose | Unit |
|---|---|
| Open a search run's row in the ledger naming the search it carries, launch its runner, read and close a run, list the runs that have carried a search | bmo-agency |
| Hand a run's ending back to its room's session as a quiet instruction | bmo-receiver |
| Import a page, fetch a raster picture, check an address, pace requests per host and engine | bmo-explorer, through the tool host |
| Read the search-engine values a research profile chooses among | bmo-explorer, through the tool host |
| Start a briefing of a finished document | bmo-voice |
| Who sent the message that asked, when, and on what road; what each room is called | bmo-receiver |
| Fetch the head of a page a document cites, for its preview | bmo-explorer, through the tool host |
| Pick the knowledge a piece of work is handed | bmo-gate |
| Start, read and stop a research run apart, which bmo-agency carries on the accounts its road is offered | bmo-agency |
| Read providers in their order and enablement, tiers, tier assignments and routes, and check an effort | bmo-agency |
| Read the pool's roles and status; take, wait for, bind and give back places; reserve and release seats; set limits | bmo-agency |
| Count a run, write a note in a run's log | bmo-agency |
| Which registration owns a home, and whether it keeps its research | bmo-agency |
| Open a morning news page run's row, launch its runner, close it | bmo-agency |
| Write a morning news page through a research run apart | bmo-agency |
| Read the index closes a morning news page draws | the `stock_index` site tool, through the tool host |
| Fetch an article's page and picture for a morning news page | bmo-explorer, through the tool host |
| Send a morning news page's message into its room | bmo-receiver |
| Tell the owner a morning news page failed, or the holiday calendar is empty for a year | bmo-receiver |

Each unit is reached at the address its name holds under `[units]` in the settings: `voice`,
`tool_host`, `agency`, `receiver` and `gate`.

The calls to bmo-receiver and bmo-gate share one shape:

- **A body is JSON, both ways,** with snake_case keys; a time is epoch milliseconds, and a field
  written with a question mark may be null.
- **A `2xx` answer is the call's answer.** A read of one thing answers `404` where it does not exist,
  and the client reads that as nothing found.
- **Any other answer is a refusal** and carries its sentence under `error`; with none, the refusal is
  "<unit> answered <status>". A call that gets no answer is reported with the failure's own words.
- **Where the unit's address is unset,** every call is refused with "the <unit> address is not set: set
  <key> under [units] in the settings".

- **Start a briefing of a finished document** (bmo-voice): `POST /briefings` with a JSON body.
  - The body carries:
    - `chat_id`, the room it is spoken to;
    - `text`, the script;
    - `started_by`, which names what started it and the document: `archive-doc:<id>` for a filed
      document's narration, `search:<id>` for a deep dive's;
    - `document`, the document whose sound it is kept as.
  - A `2xx` answer carries `unit`, the name of the run that speaks it.
  - Any other answer is a refusal and carries its sentence under `error`.
  - The filing verb passes that sentence on, after `voice: `, and so does the search runner in its
    line on the narration; a narration not spoken leaves the answer and the document as they are.
  - A call that gets no answer is reported with the failure's own words.
- **Read a page and fetch a picture** (bmo-explorer, through the tool host): `POST /calls` at the
  tool host's address, with a JSON body naming the entry under `tool` and what it is handed under
  `arguments`.
  - `explorer/import_page`, handed `address`, draws the page in a browser and reads it whole. Its
    structured content carries:
    - `page`: `url`, the address after every redirect; `title`; `keywords`, a list; `body`, the
      page's text; `references`, each with its `text` and `url`; and `figures`, each figure the page
      draws with its number `n`, its `caption` and, where the page drew it as one, its `svg`;
    - `captured`: each figure captured as a picture, by its number `n`, with the PNG's bytes in
      base64 under `png`. A figure that would not capture is not among them.
  - `explorer/fetch_raster`, handed `address` and `pacing`, fetches the raster picture behind an
    address. Its content is one image block: the bytes in base64 under `data`, and their type under
    `mimeType`.
  - bmo-explorer checks each address before it connects, refusing one that resolves inside this
    network, and paces its requests per host. `pacing` names whose requests are paced together: one
    name for each search run's reading.
  - A `2xx` answer is the entry's tool result. A result marked `isError` is a refusal, and its text
    is the sentence. Any other answer is a refusal and carries its sentence under `error`. A call
    that gets no answer is reported with the failure's own words.
  - A page is waited for 150 seconds and a picture 60. A call that takes longer is given up as
    failed.
- **Fetch a page's head** (bmo-explorer, through the tool host): `explorer/fetch_page`, handed
  `address` and `pacing`, on the same `POST /calls` and refused the same ways. bmo-explorer fetches
  the page directly, checking the address and each redirect, and reads at most 512 KiB of it. Its
  structured content carries `url`, the address after every redirect; `type`, the content type as the
  server sent it; and `html`, the page decoded as its charset says, where it is markup, null otherwise.
  The read API reads a preview out of it, and fetches the preview's pictures by `explorer/fetch_raster`.
  A fetch is waited for 10 seconds.
- **Read the search engines** (bmo-explorer, through the tool host): `explorer/search_engines`,
  handed nothing, on the same `POST /calls`, refused the same ways and waited for 30 seconds. Its
  structured content carries:
  - `engines`, each engine a research run may query, in bmo-explorer's order: `name`, `category`,
    `korean_native`, `state` (`unknown` where it has not been read), and `reason?`, `observed_at?`,
    `suspended_until?`, `elapsed_ms?` and `result_count?`;
  - `cooldowns`, each a `reason` and the `ms` an engine rests for it.
- **A morning news page's reading** (bmo-explorer, through the tool host), on the same `POST /calls`,
  refused the same ways and waited for the same times, under one `pacing` name for the run,
  `news-brief:<day>:<chatId>`:
  - an article's own picture is read by `explorer/fetch_page` on the article's address: the first of
    `og:image:secure_url`, `og:image` and `twitter:image` its `html` declares, resolved against its
    `url` and kept only where it is a web address;
  - each picture, the one the writer named or else the article's own, is fetched by
    `explorer/fetch_raster`, and one that is not a raster picture is left out.
- **Read the index closes** (the `stock_index` site tool, through the tool host): `web/stock_index`,
  handed `indices`, the keys `KOSPI`, `KOSDAQ`, `DJI`, `SPX` and `NASDAQ` the page draws, and `days`,
  20, on the same `POST /calls`, refused the same ways and waited for 60 seconds.
  - Its structured content carries `series`, one per index it could read: `key`; `days`, the settled
    closes oldest first, each `date` (`YYYY-MM-DD` in the index's own market), `close`, `change` and
    `ratio`; and `session?`, the day still trading, with its `date`, `close`, `change`, `ratio` and
    `at`, the time of its figure.
  - An index missing from `series`, a result marked `isError`, or an answer with no structured
    content leaves that index's place on the page plain.

### bmo-receiver

| Call | Method and path | Request | Answer |
|---|---|---|---|
| Read a message | `GET /rooms/{room}/messages/{message}` | | `sender_id?`, `sender_name?`, `sent_at`, `via`; 404 where the room holds no such message |
| List the rooms' names | `GET /rooms` | | `rooms`, each `chat_id` and `title?` |
| Hand an ending back | `POST /instructions` | `agent?`, `chat`, `said` (`order`), `sort`, `via?`, `run?` | 201, `id` |
| Tell the owner | `POST /instructions` | `agent` null, `chat` null, `said` (`order`), `sort` | 201, `id` |
| Send a message into a room | `POST /sends` | `asker` (`chat`, `job`), `tool` `say`, `input` (`chat`, `text`, `reply`) | `lines`, `refusal?` |

- **The owner is told** by a quiet instruction to the deployment's agent in the owner's private
  chat: `agent` and `chat` null, `sort` `news_brief`, and `said` holding an `order`, one English line
  for the agent. A morning news page that failed names the day, the room and what failed, and that
  the room was sent nothing; a year the holiday calendar holds nothing of names the year, that the
  pages go out on every weekday until it is filled, and the command that fills it. A refusal is said
  on the runner's or the service's error stream and changes nothing else.
- **A morning news page's message** is sent with `say`: `asker` names the room as `chat` and `job`
  false, and no `agent`, since no agent's turn and no job asks it; `input` names the room again, the
  message as `text` and `reply` false. An answer carrying `refusal` is a message not sent, and the run
  fails with it.

- **The token** bmo-receiver's server takes goes with every call to it as `Authorization: Bearer
  <token>`, from `receiver_token` under `[units]`. Where it is unset a call goes with none, and
  bmo-receiver refuses it `401` as it refuses any call without its token.
- **An ending handed back** is the quiet instruction the room's session reads: `said` holds one key,
  `order`, the words the session is handed; `sort` is `search`; `chat` is the run's room; `via` is the
  road the question came in on, which the answer goes out on; `agent` is the agent whose turn set the
  search off, none where no turn did; and `run` is the run in bmo-agency's ledger whose result it is.
  bmo-receiver queues it for that agent in the run's room, and for the owner's room, naming the room,
  where the run's room is gone. A refusal costs the room its answer and nothing else: the runner says
  so on its error stream.
- **A message's `via`** is the road it came in on, which bmo-receiver always names.
- **The rooms' names** are read once for each list or document read: they stand on each row and a
  phrase is matched in them. Where bmo-receiver cannot be asked, rows carry no room's name and no row
  is found by one.
- **A message whose sender is unknown** leaves the search's asker unknown, and the launcher says so in
  a note.

### bmo-gate

| Call | Method and path | Request | Answer |
|---|---|---|---|
| Pick a piece of work's knowledge | `POST /knowledge/work` | `chat_id`, `lines` | `pieces`, each `path` and `text` |

- **`lines`** are the work's own lines; a search names one, `quiet instruction from the bot:
  <question>`.
- **The pieces** are attached under the run's prompt, in the reference block a turn's reference
  stands in. A pick that fails costs the run its reference and nothing else.

### bmo-agency

The research runs' model sessions, the providers, tiers and routes they resolve on, the co-worker pool,
the agents' run counts, the agent log, the registrations and the run ledger are bmo-agency's. This
repository reaches all of them through the calls below and nothing else. The trait `Agency` declares
every call, and `AgencyClient` makes them over HTTP; a test hands the service a stand-in of its own.

- **The address** is the base of every path below. Where `agency` under `[units]` is unset, every call
  is refused with "the agency's address is not set: set agency under [units] in the settings".
- **The token** bmo-agency's server takes goes with every call as `Authorization: Bearer <token>`,
  from `agency_token` under `[units]`. Where it is unset a call goes with none, and bmo-agency refuses
  it `401` as it refuses any call without its token.
- **A body is JSON, both ways,** with snake_case keys. A time is epoch milliseconds, and a field written
  with a question mark may be null.
- **A path segment is an id as it stands,** percent-encoded where it needs to be.
- **An answer with no body is 204.** An answer with a body is 200, or 201 where the call creates
  something.
- **A read of one thing by its id answers 404 where it does not exist,** and the client reads that as
  nothing found.
- **Any other refusal** carries `error` and `code`. The client hands both on with the status; a route
  answers them as they stand.
- **A call that does not reach bmo-agency,** or an answer that does not read as its shape, is a failure
  naming the call; a route answers it `502` with `agency_failed`.

#### Providers, tiers and routes

| Call | Method and path | Request | Answer |
|---|---|---|---|
| List the providers | `GET /providers` | | a list of `name`, `label`, `enabled`, `registered`, `lines` (the work lines its program carries), `coworkers` (true where it lends co-workers) |
| List the tiers | `GET /tiers` | | a list of `name`, `coworker` (true where a co-worker may run at it), `default_capacity?` (a co-worker tier's default pool capacity) |
| List the tier assignments | `GET /tiers/assignments` | | a list of `key` (a work key) and `tier` |
| Resolve a route | `POST /routes/resolve` | `work_key?`, `provider?`, `tier` | `provider`, `tier`, `model`, `effort?`, `answer_tokens?`; 404 where no route serves it. No work key resolves the tier on its own |
| Check an effort | `POST /routes/check-effort` | `provider`, `model`, `effort` | 204; 422 with the sentence where that provider or that model does not accept it |

- **The providers are listed in the provider order.** A research run of a line starts on the first that
  is enabled and carries the line.

#### The co-worker pool

| Call | Method and path | Request | Answer |
|---|---|---|---|
| List the roles | `GET /pool/roles` | | a list of `name`, `description`, `enabled`, `pooled`, `control` (true for the worker's own control role, never lent) |
| Read the status | `GET /pool/status` | | a list of places, one per provider and tier (below) |
| Ask for a place | `POST /pool/requests` | an asking | 201, a pool request |
| Read a request | `GET /pool/requests/{token}` | | a pool request |
| List the open requests | `GET /pool/requests?owner={owner}` | | a list of pool requests waiting or active, every owner's where none is named |
| Try for a place | `POST /pool/requests/{token}/lease` | | the pool request, `active` where a place was leased |
| Bind it | `POST /pool/requests/{token}/bind` | `child` | the pool request |
| Finish it | `POST /pool/requests/{token}/finish` | `outcome`, `cancel` | `finished` |
| Reserve seats | `POST /pool/reservations` | a reservation asking | 201, a reservation |
| List the held reservations | `GET /pool/reservations?owner={owner}` | | a list of reservations held, every owner's where none is named |
| Read a reservation's seats | `GET /pool/reservations/{token}/seats` | | a list of `state` (`reserved` or `waiting`) |
| Read a reservation's pings | `GET /pool/reservations/{token}/pings` | | a list of `place`, `at`, `outcome?`, `answered_at?` |
| Release a reservation | `POST /pool/reservations/{token}/release` | `reason` | `released` |
| Promote the reservations | `POST /pool/reservations/promote` | | 204 |
| Record a place's write | `PUT /pool/places/{place}/write` | `provider`, `model`, `effort?`, `upstream?` | 204 |
| Set a limit | `PUT /pool/limits/{provider}/{tier}` | `capacity` | 204; 422 with the sentence where it is refused |
| Reset the limits | `DELETE /pool/limits` | | 204 |

- **An asking:** `request_key`, `owner`, `source`, `role`, `work_key`, `provider`, `model`, `effort?`,
  `tier`, `yielding`, `place_key?`, `turn?`, `reservation?`. A branch asks under the work key
  `role:<role>` with the role's resolved model, effort and tier, not yielding, for no turn and naming
  no reservation; bmo-agency hands it a seat its owner's reservation holds where there is one.
- **A pool request** is the asking with `token`, `state` (`waiting`, `active` or `ended`),
  `enqueued_at`, `leased_at?`, `child?`, `ended_at?` and `outcome?`.
- **A reservation asking:** `reservation_key`, `owner`, `source`, `role`, `provider`, `model`,
  `effort?`, `tier`, `wanted`. A reservation is the asking with `token`, `state` (`held` or
  `released`) and `created_at`.
- **A place in the status:** `provider`, `tier`, `capacity`, `default_capacity`,
  `snapshot_capacities` (a list), `snapshot_models` (a list of `model` and `effort?`), `active`,
  `reserved`, `waiting`, `yielding_active`, `yielding_waiting`.
- **A place a branch answered on** is named `branch-line:<provider>:<model>:<effort>`, the effort empty
  where there is none.

#### The run ledger

A search run's row lives in bmo-agency's run ledger, one row a run of an agent's unit. Its fields are
the ledger's column names, in snake_case like every other body here, but its times are epoch seconds.

| Call | Method and path | Request | Answer |
|---|---|---|---|
| Open a run's row | `POST /runs` | `agent`, `chat_id?`, `unit`, `depth`, `cap_ms?`, `started_at`, `started_by?`, `search_id?` | 201, `id`; 400 where a field it needs is missing |
| Read a run | `GET /runs/{id}` | | `id`, `agent`, `chat_id?`, `unit`, `depth`, `started_at`, `finished_at?`, `outcome?`, `context_tokens?`; 404 where the ledger holds no such run |
| Close a run | `POST /runs/{id}/close` | `finished_at`, `outcome`, `context_tokens?` | 204; 409 where it is already closed, and it keeps its first closing |
| Launch its runner | `POST /runs/{id}/launch` | `title`, `agent`, `command` | 204; a refusal with the sentence where it did not come up |
| List the runs that carried a search | `GET /runs?search={id}` | | `runs`, their ids in the order they were opened |

- **bmo-agency serves the first three.** It serves neither the launch nor the list: each is refused as
  any other refusal, so a search's runner never comes up and its launch ends as a run that did not
  (see "Searching (CLI)"), and the list reads as the run that asks it alone, or as none for the read
  API.
- **A search run's row** is opened under the agent `search`, with no `cap_ms`, the search it carries
  as a number, and as `depth` the rung it runs on by its place on the ladder: `0` for `basic`, `1` for
  `thorough`, `2` for `exhaustive`. A follow-up names the earlier search. `started_by` is who set it
  off, such as the console.
- **`command`** is the runner's process: `program`, the path of this repository's CLI binary; `args`,
  the runner's words; `cwd`, the folder the launcher started in; and `environment`, the variables
  it is handed. It is started where a run of the agent stands, under the run's unit and title.
- **The outcomes a search run is closed with:** `done`, `unanswered`, `stopped`, `failed`,
  `live-tasks`, and `error` for a run that never came up.
- **A morning news page run's row** is opened under the agent `search`, with no `cap_ms`, no search,
  `depth` `0`, the unit `bmo-news-brief-<epoch ms>`, and `started_by` `news-brief:<row id>`. Its
  runner is launched with the title `morning news page`, and the row is closed `done` where the page
  went out, `failed` where it did not, and `error` where the runner never came up.

#### Run counts, the agent log and registrations

| Call | Method and path | Request | Answer |
|---|---|---|---|
| Count a run | `POST /agents/{agent}/runs` | | 204 |
| Write a note | `POST /log/notes` | `agent`, `run`, `chat_id?`, `text` | 204 |
| Find a home's registration | `GET /registrations/by-home?home={home}` | | `name`, `keeps_research`; 404 where no registration owns the home |

- **A home** is the folder a run's program was handed as its own; a path inside it names the same
  registration. The search launcher reads its own from the variable `CLAUDE_CONFIG_DIR`, where its
  program names it.

#### Runs apart

A run apart is one model session started for a research run or one of its branches, outside any room's
conversation. bmo-agency composes the line's program and its web tools, walks the accounts its road is
offered until one carries it, and records what it spent, its answer, the windows it read and the
context it ended carrying.

| Call | Method and path | Request | Answer |
|---|---|---|---|
| Start one | `POST /apart` | an apart asking | 201, `id` |
| Read it | `GET /apart/{id}` | | a run apart |
| Stop it | `POST /apart/{id}/stop` | | 204 |

- **An apart asking:** `agent` (the agent it is counted under), `line`, `title`, `work_key`, `road`,
  `instructions`, `environment`, `prompt` and `log?`.
  - `road` holds `kind`: `key`, with `provider`, `model`, `effort?` and `tier`; or `registration`, with
    `name`, the registration it goes out on with that registration's own model.
  - `instructions` holds `name`, the instructions bmo-agency composes for the line (`search` for a
    research run's own, `search-branch` for a branch's), and `text?`, the run's own where its caller
    wrote them.
  - `environment` is an object of the variables its web tools are told, each a string.
  - `log` holds `run` and `chat_id?`: the run log every line the run writes is fed to. None keeps no
    log.
- **A run apart:** `id`, `state` (`running`, `answered`, `failed` or `stopped`), `streamed` (every line
  its record has gained so far, in order), `reply?` and `failure?`.
  - `reply` holds `text`, `model?`, `effort?`, `context_tokens?` and `live_tasks` (the names of its own
    work still running as it ended).
  - `failure` is the sentence why it gave no answer.
- **This repository asks every half second** until the run is no longer `running`, and stops it the
  moment its own run is stopped or its research closes.
- **A morning news page's writer** is a run apart on the `search` line under the `search:basic`
  profile's road: `instructions` names `news-brief`, the news task bmo-agency composes for the line out
  of its own prompt tree with the line's tools and rulebooks, and carries no `text`; `prompt` is the
  prompt tree's `agents/search/news-brief-request.md`, filled with `{{day}}` and `{{askedAt}}`; `log`
  names the run's unit and the room. It opens no branch. Its `reply` text is the brief.

## Gaps in siblings

What another repository's boundary must gain for this one's statements to hold, each for that
repository's worker:

- **The `stock_index` site tool** stands in no repository of the new layout, and the reading it
  answers is text for an agent, with no structured content. Until its own repository places it with
  the tool host and answers `series` as "Read the index closes" states, every index place on a
  morning news page stays plain.
- **bmo-web's language pack** holds no `news_page` keys. A wording file must hold every key (see
  "Wording"), so a deployment whose wording setting names that pack is refused until the pack gains
  them, and one naming no file reads the English default this repository ships.
- **bmo-agency** serves no runner launch (see "The run ledger"), so neither a search's runner nor a
  morning news page's comes up until it does; and its document states no run apart, nor the
  `news-brief` instructions it is to compose for the search line out of its own
  `prompts/agents/search/news-brief.md`, which hold Korean where every model prompt is to be English.
