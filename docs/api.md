# The boundary: bmo-artifact

bmo-artifact keeps documents, finds them again and shares them by link, with members and with organisation units: records, learning books, imported
pages and HTML pages, each with its body, its attachments, the pages it refers to and its cover picture. It
is an upstream (bmo-chat's `docs/conventions.md`, 「Upstream and downstream」): a harness takes it as its
first entry point through its MCP server, with no console and no other unit running beside it but the page
reader it imports through. It runs no research and speaks no narration: a research run is a client's work
that files its result here, and a narration is a sound a client attaches. This document is the whole of what
it serves and of what it calls; another repository reaches it through nothing else.

```mermaid
flowchart TD
  H[A harness] -->|stdio| M[bmo-artifact mcp]
  C[Any other caller] -->|HTTP| S[The service]
  M -->|the client| S
  S --> D[(The document tables)]
  S --> F[The file root]
  S -->|bmo-explorer's client| E[Page reading]
  S -->|bmo-web's client| W[The account service]
```

## Versioning

- **The repository has one version,** in `VERSION`, the Cargo manifest and the console bundle's package,
  which `GET /health`, the OpenAPI document and the MCP server's `serverInfo` all answer.
- **It is numbered as bmo-chat's `docs/conventions.md` says (「Versions」):** before the first release
  `0.1.0`, then `0.1.0-rev.<n>`, one revision a version bump, whatever the change.

## The wire

- **The wire format is bmo-chat's** (`docs/conventions.md`, 「The common wire format」 and 「API design」):
  camelCase keys, times in epoch milliseconds, ids as signed 64-bit integers, a page as `items` and
  `more`, a failure as a status with `code` and `error`.
- **Every route takes a bearer token,** as `Authorization: Bearer <token>`: the service's token or the
  console's, both named by the settings. A missing or wrong token is refused 401 `unauthorized`; an empty
  token in the settings lets no caller in by it.
- **The service's token reaches every route but the console face,** which takes the console's token alone.
  The console's token reaches only the console face and what its screens use, under the guard 「The console」
  states.
- **The paths carry no prefix.** Whatever serves the service to a browser mounts it under its own path.
- **What a caller reads depends on who it reads for** (「Readers」).
- **A caller's strings are opaque:** `scope` is what a document or an import is held under, `actorName`
  who filed it, and `labels` the caller's own metadata, an object of strings. The service stores and
  matches them and never reads meaning into them.
- **Every answer is English,** a sentence or a status code.
- **`docs/openapi.json` is the contract,** an OpenAPI 3.1 document generated from the service's own types:
  every route with its parameters, bodies, answers and codes and how far the console's token reaches it under
  `x-console`, the events under `x-events`, the tools under `x-tools`, the plugin under `x-harness` and the
  units it calls under `x-calls`. A test fails where the
  committed document differs from what the code generates; `bmo-artifact openapi` prints it.

### Codes

| Status | `code` | When |
|---|---|---|
| 400 | `bad-request` | the body, the query or a path key is not the shape the route takes; the sentence names what |
| 401 | `unauthorized` | the token is missing or wrong, or is the service's on the console face |
| 403 | `forbidden` | the console's token on a route its screens do not use, or without the role or owner key the screen needs; an account reader on a route that changes a document or how it is shared, where it reads the document alone, or on a route only a reader of every document reaches (「Readers」) |
| 404 | `no-route` | no route answers this method and path |
| 404 | `not-found` | nothing stands at the key the path names |
| 409 | `conflict` | the member does not take the change: a body for a page, HTML for any other kind; a member or unit share of a document whose owner stands in no company, or a unit share of one with no owner |
| 410 | `gone` | a share link that has ended, and what it opened; an event stream resumed from before the window the service keeps |
| 413 | `too-large` | the body or an attachment is over its ceiling |
| 422 | `read-only` | a member share asked with a permission other than `read` (「Member shares」) |
| 422 | `other-company` | a member or a unit outside the company the document's owner stands in (「Member shares」, 「Unit shares」) |
| 500 | `failed` | the service failed: its store or its file root; the sentence is the failure |
| 502 | `upstream-failed` | the account service failed, or the settings name none, where a call needs it (「What this service calls」) |


### Lists

- **A page is newest first** unless its route says otherwise, at most `limit` items, 30 where none is
  given and 200 at most; a `limit` that is not a whole number from 1 is refused `bad-request`.
- **`before` and `after` are ids:** `before` asks for the members older than the one named, `after` for the
  ones newer. A page answers `more` where something stands past its last item.

### Readers

A call reads for one of two readers, and the reader decides which documents it reaches (docs/artifact.md,
「Owners」).

- **A reader of every document** is a caller holding the service's token that names no account. It reaches every
  document as each route below states.
- **An account reader** is the account a call names in `Owner-Key`: the console relaying for any signed-in reader,
  and a caller holding the service's token that names an account, as a supervisor's MCP server and archive verb do
  for the user they work for. `Owner-Role` `administrator` marks the platform administrator. It reaches:
  - **wholly,** the documents it owns, and, for the administrator, the documents with no owner: it reads them, and
    rewrites, attaches to, lets go of and shares them on the routes it reaches;
  - **to read alone,** the documents shared with its account (「Member shares」) or with a unit it stands in or under
    (「Unit shares」), and, for the administrator, every document another account owns, shared or not, each
    answered with `permission` `read`;
  - and no other document: the administrator therefore reads every document of every company, in lists, searches
    and label counts as in each document's own reads.
- **What an account reader does not reach is not there for it:**
  - `GET /documents` lists what it reaches alone, `GET /label-values` counts those alone, and `GET /shares` lists
    their live links alone;
  - `GET /documents/{documentId}` and every route under it answer a document it does not reach 404 `not-found`, in
    the sentence of a document that does not stand, and so does every route that changes one;
  - a route that changes a document or how it is shared — `PATCH` and `DELETE /documents/{documentId}`, `POST
    /documents/{documentId}/attachments`, `POST /shares`, `DELETE /shares/{token}`, and every route of 「Member
    shares」 and 「Unit shares」 — is refused 403 `forbidden` on a document it reads alone;
  - `GET /imports/{importId}` answers the imports it asked for alone, any other 404 `not-found`;
  - `GET /events` and the routes of 「Scopes」, which reach every document, are refused 403 `forbidden`;
  - a call on the console's token with no `Owner-Key` owns nothing and has nothing shared with it.
- **The unit a reader stands in is the account service's answer at the time of the call** (「What this service
  calls」): a unit share opens a document to a reader whose unit is that unit or one under it, read through `GET
  /users/{user}/unit` whenever a unit share decides a read. A user the account service does not know stands in no
  unit; where it fails, the read is refused 502 `upstream-failed`; where the settings name no account service, no
  reader stands in any unit. The administrator's reads ask it nothing, since no unit decides them.
- **The headers are trusted as the caller's credential vouches for them.** The console's are its word for its
  signed-in reader (bmo-ui's `docs/console-bundles.md`, 「Roles and the owner key」); a holder of the service's token
  names the account it acts for. No body field names a reader or an owner.
- **The share reads, the share head and the readings** are held to their token alone, whoever reads.

## Documents

A document is one row of the document tables and one file under the file root (docs/artifact.md, 「Where it
lives」).

| Field | Holds |
|---|---|
| `id` | the document's number |
| `kind` | one of the kinds below |
| `title` | its title |
| `scope` | what it is held under, null where none was given |
| `actorName` | who filed it |
| `owner` | the account key of the account that owns it, as `Owner-Key` named it when it was filed; null for a document with no owner (docs/artifact.md, 「Owners」) |
| `labels` | the caller's metadata |
| `description` | what it reads as before it is opened: the one the filer gave, else a page's declared description, else the opening prose of its body as plain words; cut at 160 characters; null where it has none |
| `attachmentCount` | how many files it keeps |
| `hasPicture` | whether it keeps a picture, so `GET /documents/{documentId}/picture` has a cover to send |
| `createdAt`, `changedAt` | when it was filed and last rewritten |
| `permission` | for an account reader that reads the document alone, through a share or as the administrator reading one another account owns, the permission it reads it under, `read` (「Readers」); absent where the reader reaches it wholly, and in every event |

### Kinds

Each kind is a module of its own answering one shape (docs/artifact.md, 「Kinds」); `GET /document-kinds`
lists them in the catalogue's order, each `name`, `description`, `hosted` (whether the document is opened
as the page it keeps), `contentsDepth` (how many levels of its headings its table of contents lists,
0 for none) and `icon` (the name of the icon the screens draw the kind by, in the Material Symbols set);
its `after` is a kind's place in the catalogue, counted from 1. A row whose kind no module
answers reads as `record`.

| Kind | What it is | Its file |
|---|---|---|
| `record` | data put in order and kept | the markdown body as filed |
| `learning` | study material, a book of parts, chapters and sections | the markdown body as filed |
| `import` | a page brought in whole through `POST /imports` | the markdown the import wrote |
| `page` | an HTML page kept as it stands | the HTML as filed |

### Labels

`GET /label-values` answers the distinct values the label `key` holds across the listed documents, each
`value` and `count` (how many documents carry it), the most carried first and, among equals, in the
values' order. A page answers at most `limit`, `more` where values stand past it, and takes no cursor. A
blank `key` is refused `bad-request`. The values are the callers' own strings, matched as they are written.

### Routes

| Route | Takes | Answers |
|---|---|---|
| `GET /documents` | query `kind?`, `scope?`, `label?`, `q?`, `picture?`, `before?`, `after?`, `limit?` | 200, a page of documents |
| `POST /documents` | `kind`, `title`, `body?`, `html?`, `scope?`, `actorName`, `labels`, `description?` | 201, the document |
| `GET /documents/{documentId}` | | 200, the document |
| `PATCH /documents/{documentId}` | `title?`, `body?`, `html?`, `labels` | 200, the document |
| `DELETE /documents/{documentId}` | | 204 |
| `GET /documents/{documentId}/content` | query `scheme?`, `download?` | 200, the file |
| `GET /documents/{documentId}/picture` | query `width?` | 200, the cover picture as JPEG |
| `POST /documents/{documentId}/reads` | | 204 |
| `GET /document-reads` | query `limit?` | 200, a page of the documents most read |
| `GET /document-reads/recent` | query `limit?` | 200, a page of the documents the reader read last |
| `GET /document-kinds` | query `after?`, `limit?` | 200, a page of kinds |
| `GET /label-values` | query `key`, `limit?` | 200, a page of the values one label holds |

- **`GET /documents` filters in the reading itself,** so a filtered page is a full one. `kind` holds it to
  one kind and `scope` to one scope; `label` is `key:value` for documents whose label `key` is `value`, or
  `key` alone for those carrying it; `q`, cut to 200 characters, holds it to documents saying the phrase,
  matched whole and without regard to case in the title, the labels and the body, and, where the phrase is a
  number with or without a leading `#`, to the document of that number as well. A blank `q` is no search.
  `picture` set to `true` holds it to the documents keeping a picture, those `hasPicture` marks.
- **`POST /documents` files a document.** `page` takes `html` and nothing in `body`; every other kind takes
  `body` and no `html`. The title is required, except for a page whose own `title` element gives one.
  Each distinct http or https link the body makes becomes one of the document's references. Its owner is the
  account the call names in `Owner-Key`, none where it names none (「Readers」). Refused
  `bad-request` for a kind no module answers, an empty title or body, or a body and HTML given the wrong
  way round.
- **A number that stands for no document answers 404 `not-found`** on `GET /documents/{documentId}` and
  every route under it: none was filed under it, or its row left no document (docs/artifact.md, 「Where it
  lives」).
- **`PATCH /documents/{documentId}` rewrites it under its number;** its attachments and share links stay.
  `labels` replaces the labels whole; a `title`, `body` or `html` given replaces that one, and its
  references are read again from a new body. A body for a page or HTML for any other kind is refused
  409 `conflict`.
- **`DELETE /documents/{documentId}` lets go of the document** as letting go of its scope lets go of each it
  holds (「Scopes」): its row, its labels, its files, attachments and references, its member and unit shares, and its
  share link, which leaves its note behind. A reading issued for it opens nothing after. An import that filed it keeps its own
  record. It is a `document.deleted`, and its link a `share.deleted`.
- **`GET /documents/{documentId}/content`** sends the file: `text/markdown` for a markdown kind, never
  cached. A page goes out as a hosted page (docs/artifact.md, 「An HTML page」): its prelude ahead of its
  first element in the scheme `scheme` names, `light`, `dark` or `system`, the design's default where none is named, under the
  sandboxed page policy. Its sounds stand in the player speaking the language `language` names, a locale of the
  console bundle's words, English where it names none or one the words do not hold, and play from a reading
  address (「Readings」). `download=true` sends the file itself to keep, under its name. 404 `not-found`
  where the document has no file inside its folder.
- **`GET /documents/{documentId}/picture`** sends the picture a link's preview card shows: the first its
  body draws by a kept file's name (docs/artifact.md, 「Attachments」), else the first it keeps, upright,
  fitted inside 1200 pixels, never enlarged, laid on white, as JPEG. `width` asks it drawn small: that card's
  copy at the first step of the ladder at or above it, as docs/artifact.md 「A picture drawn small」 says, the card
  itself where it is no wider. 404 `not-found` where it has none.

### Reads

A read is one reader's day on one document (docs/artifact.md, 「Reads」). The service keeps the reads of the
last seven days, a day being the service clock's day in UTC, each with the moment the reader last opened the
document that day.

A document most read:

| Field | Holds |
|---|---|
| `document` | the document, as `GET /documents` answers it |
| `reads` | how many reads it has in the last seven days |

A document the reader read last:

| Field | Holds |
|---|---|
| `document` | the document, as `GET /documents` answers it |
| `readAt` | when the reader last opened it, in milliseconds since the epoch |

- **`POST /documents/{documentId}/reads` keeps a read** of the document for the call's reader: the account its
  `Owner-Key` names, or, where it names none, one reader standing for every caller without an account. The same
  reader's second read of the document on the same day keeps no second read and moves the first to that moment.
  A document the reader does not reach is `not-found` (「Readers」).
- **`GET /shares/{token}/document` with `visitor` keeps a read** for that visitor, the token a browser keeps for
  itself and a share page sends, 8 to 64 letters, digits, `-` and `_`; a `visitor` of any other shape is refused
  `bad-request`, and without one nothing is kept (「Share links」).
- **Keeping a read lets go of every read older than the seven days.**
- **`GET /document-reads` answers the documents most read** in the last seven days, the most first and, among
  equals, the newest first; at most `limit`, `more` where documents stand past it, and no cursor. A document with
  no read in the seven days is not among them. For an account reader it holds the documents that account reads
  alone, each with its permission where it reads it through a share (「Readers」).
- **`GET /document-reads/recent` answers the documents the call's reader read last** in the last seven days,
  the one opened last first and, among equals, the newest first; at most `limit`, `more` where documents stand
  past it, and no cursor. The reader is the one `POST /documents/{documentId}/reads` keeps reads for, so it holds
  that reader's reads and never another's; a visitor's reads through a share link are no account's. For an
  account reader it holds only the documents that account still reads, each with its permission where it reads
  it through a share (「Readers」).
- **A read raises no event,** and goes with its document when the document is let go of.

## Attachments

| Field | Holds |
|---|---|
| `id`, `documentId` | the attachment and its document |
| `name` | the name it is shown and named by in the body; a last part such as `-ko` before the extension is the language it is in (docs/artifact.md, 「Attachments」) |
| `mediaType` | its type as kept |
| `kind` | what it is read as: `image`, `audio`, `video`, `text`, `page` or `file` |
| `byteCount` | its size |
| `role` | the caller's word for its part in the document, null for none; `head` marks a file a reader is shown at a markdown document's head, a narration's sound among them, which the reader plays in the listen band; a sound named as a head sound is with another language stands beside it there (docs/artifact.md, 「How a document is drawn」). A page stands its sounds where it writes them, by their names (docs/artifact.md, 「An HTML page」) |
| `createdAt` | when it was kept |

| Route | Takes | Answers |
|---|---|---|
| `GET /documents/{documentId}/attachments` | query `after?`, `limit?` | 200, a page of attachments, in the order they were kept |
| `POST /documents/{documentId}/attachments` | `name`, `mediaType`, `role?`, `content` in base64 | 201, the attachment |
| `GET /documents/{documentId}/attachments/{attachmentId}` | query `scheme?`, `language?`, `download?`, `width?`; the `Range` header | 200 or 206, the bytes |

- **A file is kept as docs/artifact.md 「Attachments」 says:** its kind read from its bytes and its name, a
  picture kept upright as AVIF, a name the document already keeps refused `bad-request`. One file is at
  most 64 MiB, refused `too-large` past it.
- **`width`** asks a kept picture drawn small: the copy at the first step of the ladder at or above it,
  answered 200 whole, as docs/artifact.md 「A picture drawn small」 says; a width a picture is not shrunk to
  sends the file as kept. The share and reading routes for an attachment take it alike.
- **The bytes are served** with `Range` answered 206, an HTML file as a hosted page as the content route
  does, and every other file under the policy that lets nothing on it run. `download=true` sends it to
  keep. An attachment of another document is `not-found`.

## References

A reference is a page a document refers to: a link its body makes, the page an import came from, or a
work that page cites.

| Field | Holds |
|---|---|
| `id`, `documentId` | the reference and its document |
| `url` | the page's address |
| `title` | the page's title as the document or its preview gives it, null where neither does |
| `preview` | the description the page declares for a preview, null where it declares none or has not been read |

| Route | Takes | Answers |
|---|---|---|
| `GET /documents/{documentId}/references` | query `after?`, `limit?` | 200, a page of references, in the document's order |
| `GET /documents/{documentId}/references/{referenceId}` | | 200, the reference, its preview read |
| `GET /documents/{documentId}/references/{referenceId}/image` | | 200, the page's preview picture |

- **A preview is read through bmo-explorer's client** (「What this service calls」) and kept under the file
  root: a preview for a week, a picture for a month, a failure for a day. The list answers what is kept; one
  reference read alone reads its preview where none is kept fresh, and waits for bmo-explorer's answer however
  long it takes.
- **The image** goes out as the type its bytes are, not sniffed and sandboxed; 404 `not-found` where the
  page declares none or it could not be fetched.

## Share links

A share link opens one document for whoever holds its token. A document has at most one live link.

| Field | Holds |
|---|---|
| `token` | the link's token, 8 to 128 letters, digits, `-` and `_` |
| `documentId` | the document it opens |
| `url` | the share address the settings name, a `/` and the token |
| `createdAt` | when it was made |
| `expiresAt` | when it stops opening, null for a link with no end |

| Route | Takes | Answers |
|---|---|---|
| `GET /shares` | query `documentId?`, `after?`, `limit?` | 200, a page of live links, oldest first; `after` is a `createdAt` |
| `POST /shares` | `documentId`, `expiresAt?` | 201, the link made; 200, the live link that already stands |
| `GET /shares/{token}` | | 200, the link |
| `DELETE /shares/{token}` | | 204 |

The **share reads** open, to whoever holds a live link's token, the one document it opens, each answering
what the route under `/documents/{documentId}` it stands for answers for that document:

| Route | Takes | Answers |
|---|---|---|
| `GET /shares/{token}/document` | query `visitor?` | 200, the document; with `visitor`, a read kept (「Reads」) |
| `GET /shares/{token}/kind` | | 200, the document's kind, as `GET /document-kinds` lists it |
| `GET /shares/{token}/content` | query `scheme?`, `language?`, `download?` | 200, the file |
| `GET /shares/{token}/picture` | | 200, the cover picture as JPEG |
| `GET /shares/{token}/attachments` | query `after?`, `limit?` | 200, a page of attachments |
| `GET /shares/{token}/attachments/{attachmentId}` | query `scheme?`, `language?`, `download?`, `width?`; the `Range` header | 200 or 206, the bytes, answering any origin |
| `GET /shares/{token}/references` | query `after?`, `limit?` | 200, a page of references |
| `GET /shares/{token}/references/{referenceId}` | | 200, the reference, its preview read |
| `GET /shares/{token}/references/{referenceId}/image` | | 200, the page's preview picture |

- **A page a share read sends** has its sounds play from the share route of their attachments
  (docs/artifact.md, 「An HTML page」).
- **Every read and every making sweeps away the links whose time has passed,** each a `share.deleted`.
- **`POST /shares`** refuses an `expiresAt` not in the future `bad-request`, and a document that does not
  stand `not-found`.
- **A share read answers for a live token alone,** and only with what that link's document holds: an
  attachment or a reference of another document is `not-found`. `GET /shares/{token}` and the share reads
  are the whole of what the link opens.
- **A link that has ended leaves a note of what it opened** (docs/artifact.md, 「Share links」): its token and
  that token alone answers 410 `gone`, with the sentence naming the document's title and its number, where the
  link ended by its time, by `DELETE /shares/{token}` or by its document being let go of. `GET
  /shares/{token}` and every share read of that token answer so, and no live link stands there any more.
- **A token that never named a link is `not-found`,** with the plain sentence saying so.
- **A reader with no token of the service's** opens a link through whoever serves the service to a
  browser: it reads the link, then the document and everything of it through the share reads, each carrying
  the token.

The **share head** answers what a live link reads as before it is opened, the preview a chat app or a browser
draws its card from, as data a console writes into the head of the page it serves at the link's address
(bmo-ui's `docs/console-bundles.md`, 「Share pages」; docs/artifact.md, 「Where a person reads it」):

| Route | Takes | Answers |
|---|---|---|
| `GET /shares/{token}/head` | query `language?` | 200, the head |

| Field | Holds |
|---|---|
| `title` | the document's title, as the page shows it |
| `description` | the document's description led by its kind's word where the page names the kind over the title, joined in the pack's sentence; absent where the document has none |
| `image` | `/shares/{token}/picture` with the link's token, the share read of the cover picture; absent where the document keeps none |
| `url` | the link's address, as `GET /shares/{token}` answers it |
| `locale` | the pack's `artifact.preview.locale` |
| `siteName` | the pack's `artifact.preview.site_name` |
| `type` | `article` |
| `kind` | the pack's word for the document's kind, as written where the pack has none; absent for a kind the page draws whole with no title over it |

- **The words are the console bundle's pack,** read through the console face: each word in the language
  `language` names, a locale of the bundle's words, and in English where none is named or the words hold that
  word in none of it.
- **It is held to the token** as a share read is: a token that never named a link is `not-found`, one whose
  link has ended `gone`, and where no console face stands it is `not-found`.

### Readings

A reading token opens one document's attachments to the page the console's own reader opens, for twelve
hours, and the player's script goes out on its own (docs/artifact.md, 「Readings」). Both are share reads: a
console relays them with no session, as a sandboxed page asks for them with none.

| Route | Takes | Answers |
|---|---|---|
| `GET /readings/{token}/attachments/{attachmentId}` | query `scheme?`, `language?`, `download?`, `width?`; the `Range` header | 200 or 206, the bytes, answering any origin |
| `GET /player/{token}` | | 200, the player's script, the token being its file's name |

- **A reading token whose signature does not hold, or whose time has passed,** is `not-found`, and so is an
  attachment of another document.
- **The player's file name** is `bmo-audio-player`, the first sixteen hexadecimal digits of the SHA-256 of
  its bytes and `.js`; it goes out as `text/javascript`, kept a year. Any other name, or a player folder with
  no script, is `not-found`.
- **The player's words** are the bundle's pack's `artifact.player` words of each locale, which the build writes
  beside the script in the player folder the settings name, where the console face does not answer them.

## Member shares

A member share opens one document to read to one member of the owner's company: the account the console's
people directory names by its key (bmo-ui's `docs/console-bundles.md`, 「How a bundle mounts」, the people
directory). A document is shared with each account at most once.

| Field | Holds |
|---|---|
| `id`, `documentId` | the member share and its document |
| `user` | the member's account key, as the console writes `Owner-Key` |
| `permission` | `read` |
| `sharedAt` | when it was shared, in epoch milliseconds |

| Route | Takes | Answers |
|---|---|---|
| `GET /documents/{documentId}/members` | query `after?`, `limit?` | 200, a page of member shares, in the order they were shared |
| `POST /documents/{documentId}/members` | `users`, `permission` | 200, a page of the member shares of those users, made now or already standing |
| `PATCH /documents/{documentId}/members/{user}` | `permission` | 200, the member share |
| `DELETE /documents/{documentId}/members/{user}` | | 204 |

- **The document's owner shares it,** the administrator a document with no owner, and a reader of every document
  any (「Readers」): an account reader that reads it alone is refused 403 `forbidden` on each route here, and one
  that does not read it at all is answered `not-found`.
- **A document that does not stand is `not-found`** on every route here.
- **`GET /documents/{documentId}/members`** pages by `after`, the `id` of the last member read, and `limit`, 30
  where none is given and 200 at most.
- **`POST /documents/{documentId}/members`** shares the document with every key in `users`, 1 to 200 keys of 1
  to 200 characters each, refused `bad-request` otherwise. A key already a member keeps its share as it stands.
  For a document with an owner, each key is asked of the account service (`GET /users/{user}/unit`) before
  anything is stored: a user whose company is not the owner's, or whom the account service does not know, is
  refused 422 `other-company`; an owner the account service knows in no company 409 `conflict`; and the account
  service failing, or none named, 502 `upstream-failed`.
- **`permission` is `read`.** The document screens write nothing, so a share opens reading alone: `edit` is
  refused 422 `read-only` before anything is stored, on `POST` and `PATCH` alike.
- **`PATCH /documents/{documentId}/members/{user}`** sets the permission; a key that is no member is
  `not-found`.
- **`DELETE /documents/{documentId}/members/{user}`** takes the member off, and their reading stops at once; a
  key that is no member is `not-found`.
- **A member share goes with its document,** and with the scope that held it. No event is raised for one.

## Unit shares

A unit share opens one document to read to everyone standing in one organisation unit of the owner's company or in
any unit under it (docs/artifact.md, 「Unit shares」). The unit is named by its number, as the account service
numbers its units (bmo-web's `docs/boundary.md`, 「Organisation units」); a company is the unit at the top of its
tree. A document is shared with each unit at most once.

| Field | Holds |
|---|---|
| `id`, `documentId` | the unit share and its document |
| `unit` | the unit's number |
| `sharedAt` | when it was shared, in epoch milliseconds |

The units a document may be shared with are its owner's company's tree, read from the account service when asked:

| Field | Holds |
|---|---|
| `company` | the company's number, the unit at the top of the tree |
| `units` | every unit of the company, the company first and each unit after its parent, its siblings in the account service's order: each `id`, `name`, and `parent`, null for the company |

| Route | Takes | Answers |
|---|---|---|
| `GET /documents/{documentId}/company-units` | | 200, the units of the owner's company |
| `GET /documents/{documentId}/units` | query `after?`, `limit?` | 200, a page of unit shares, in the order they were shared |
| `POST /documents/{documentId}/units` | `units` | 200, a page of the unit shares of those units, made now or already standing |
| `DELETE /documents/{documentId}/units/{unit}` | | 204 |

- **Who shares, and reads the units to share with,** is as for member shares (「Member shares」): the document's
  owner, the administrator for a document with no owner, a reader of every document any; 403 `forbidden` for an
  account reader that reads it alone, `not-found` for one that does not read it, and for a document that does not
  stand.
- **`GET /documents/{documentId}/company-units`** reads the owner's company through `GET /users/{user}/unit`, then
  the company and each unit under it through `GET /units/{unit}`, one read a unit, and keeps none of it: a unit made
  or moved counts from the next ask. A document with no owner, or whose owner the account service knows in no
  company, is refused 409 `conflict`, and the account service failing, or none named, 502 `upstream-failed`.
- **`GET /documents/{documentId}/units`** pages by `after`, the `id` of the last unit share read, and `limit`, 30
  where none is given and 200 at most.
- **`POST /documents/{documentId}/units`** shares the document with every unit in `units`, 1 to 200 numbers,
  refused `bad-request` otherwise. Before anything is stored, the owner's company is read through `GET
  /users/{user}/unit` and each unit through `GET /units/{unit}`: a unit that does not stand is `not-found`, a unit
  of another company 422 `other-company`, and the account service failing, or none named, 502 `upstream-failed`.
  A document with no owner, or whose owner the account service knows in no company, is refused 409 `conflict`,
  since it belongs to no company. A unit already sharing it keeps its share as it stands.
- **`DELETE /documents/{documentId}/units/{unit}`** takes the unit off, and every reader who read the document only
  through it stops reading at once; a unit it is not shared with is `not-found`.
- **A unit share goes with its document,** and with the scope that held it. No event is raised for one.

## Publications

A publication files a document and opens a link with no end to it in one call: what a document is handed to
somebody as, the address they keep rather than a copy they were sent.

| Field | Holds |
|---|---|
| `document` | the document filed, as `POST /documents` answers it |
| `share` | the link that opens it, its `expiresAt` null |

| Route | Takes | Answers |
|---|---|---|
| `POST /publications` | `kind`, `title`, `body?`, `html?`, `scope?`, `actorName`, `labels`, `description?` | 201, the document and its link |

- **The document is filed as `POST /documents` files one,** taking the same body, its owner the account the call
  names in `Owner-Key`, and refusing the same ways.
- **The link has no end** and names the document rather than a body: a rewrite of that document keeps the
  link, and the link serves the newest body whenever it is opened (docs/artifact.md, 「Share links」).
- **The link is a share link like any other** once made: `GET /shares`, the share reads and `DELETE
  /shares/{token}` answer it, and its token answers what stood once it has ended.
- **Only the service's token publishes.** The console's does not reach the route, since no screen files a
  document.
- **Each half is told:** a `document.created` for the document and a `share.created` for the link.

## Imports

An import brings a page in whole as a document of kind `import` (docs/artifact.md, 「A page imported
whole」).

| Field | Holds |
|---|---|
| `id` | the import's number |
| `url` | the address asked for |
| `state` | `fetching`, `filed` or `failed` |
| `documentId` | the document it filed, null until it is `filed` |
| `error` | why it failed, null otherwise |
| `createdAt` | when it was asked for |

| Route | Takes | Answers |
|---|---|---|
| `POST /imports` | `url`, `title?`, `scope?`, `actorName` | 201, the import, `fetching` |
| `GET /imports/{importId}` | | 200, the import |

- **The page is read after the answer,** and the import moves to `filed` or `failed` once it is, each an
  `import.changed`. The document it files carries the import's `scope` and `actorName`, the `title`
  given, else the page's own, and as its owner the account the asking call named in `Owner-Key`, none where it
  named none.
- **`POST /imports`** refuses an address that is not http or https `bad-request`. A page that cannot be read
  is no refusal of the route: the import is `failed`, its error the page reader's sentence.
- **An import still `fetching` when the service stops** is `failed` when it starts again, its error saying
  the service stopped before the page was read.

## Scopes

| Route | Takes | Answers |
|---|---|---|
| `GET /scopes/{scope}` | | 200, what the service holds under the scope |
| `DELETE /scopes/{scope}` | query `dryRun?` | 200, what was let go of, or with `dryRun=true` what would be |

- **Only a reader of every document** reaches a scope: an account reader is refused 403 `forbidden`.
- **The answer** is `scope`, `counts` (`documents`, `attachments`, `shares` and `imports` held under it)
  and `state`: null for a read, `released` once let go of, `preview` for a dry run. A scope holding
  nothing is `not-found`.
- **Letting go** removes every document held under the scope with its files, attachments, references, member
  and unit shares and share links, each as `DELETE /documents/{documentId}` lets go of one, and every import asked under it.
  Each document removed is a `document.deleted` and each link a `share.deleted`.

## Events

| Kind | When | `data` |
|---|---|---|
| `document.created` | a document filed, by a caller or an import | the document |
| `document.changed` | a document rewritten | the document |
| `document.deleted` | a document let go of, alone or with its scope | `id` |
| `attachment.created` | a file attached | the attachment |
| `share.created` | a link made | the link |
| `share.deleted` | a link taken away, swept or let go of | `token` |
| `import.created` | an import asked for | the import |
| `import.changed` | an import filed or failed | the import |

- **Only a reader of every document** reads the events, which carry every document filed or changed: an account
  reader is refused 403 `forbidden`.
- **`GET /events`** is the stream, as server-sent events, each `event` its kind, `id` its number and `data`
  `{"id", "kind", "at", "data"}`; asked with `Accept: application/json` it answers a page of the same
  events, oldest first.
- **It narrows** by `kinds`, a comma-separated list, and by `scope`, to the events of members held under
  it; it resumes after `Last-Event-ID` or `after`, and pages by `limit`, 100 where none is given and 1000
  at most.
- **The service keeps a week of events.** A resume from before the window is refused 410 `gone`, and the
  reader reads the collections again.
- **A comment line every fifteen seconds** keeps the stream open, and an event reaches a reader within a
  second of being kept.
- **The stream ends when the service is stopped,** and the reader resumes after its last event once the
  service answers again.

## Health

- **`GET /health`** answers 200 with `ready`, `reason` (null when ready) and `version`. It is not ready
  while the database does not answer, and the reason says so.

## The command

`bmo-artifact` is one binary: the service, its command and its harness plugin alike.

- **`bmo-artifact serve`** settles the document tables and answers the routes.
- **`bmo-artifact migrate`** brings the database the settings name up to the schema and prints how many
  migrations it ran; the deployment and the testbed's template run it.
- **`bmo-artifact health`** asks the service the settings name, waits for the answer however long it takes,
  and ends 0 where it answers ready; a refused or broken connection ends it 1.
- **`bmo-artifact openapi`** prints the OpenAPI document the code generates.
- **`bmo-artifact console-reach`** prints, as one JSON object on one line, how the testbed's console probe
  reaches the service the settings name: `address`, the address its routes answer at, with an unspecified
  listen address read as the loopback one; `consoleToken`, the console's token; and `token`, the service's.
- **`bmo-artifact mcp [--address <url>]`** serves the harness tools (「The harness plugin」).
- **`bmo-artifact html-read <file> [--pictures <dir>] [--stem <name>] [--from <line>]`** reads an HTML file
  for its words in order, cut at the answer ceiling (docs/artifact.md, 「An HTML page」).
- **`bmo-artifact archive doc`** files or rewrites a document from a shell (「The archive verb」).
- **`bmo-artifact nouns`** prints the nouns `bmo` dispatches to this command, one a line: `archive doc` and
  `html-read`. It reads no settings, so it answers wherever the caller stands; no noun carries a mark.
- **The nouns' words:** `html-read` is `bmo-artifact html-read <file> [--pictures <dir>] [--stem <name>]
  [--from <line>]`. It calls no route: it reads the file where it stands. `archive doc` is below.
- **The settings a command stands on** are the ones in the directory it was started in, where `settings.toml`
  stands there; from any other directory, the deployed release's own, above the binary's `bin/`. A path a
  setting names that is relative is read from the directory those settings stand in, wherever the process
  was started.
- **How a command ends:** 0 on success; a refusal on standard error and 1 otherwise.

## The archive verb

`bmo archive doc --title <words> (--body <file> | --html <file>) [--file <path>]... [--replace <id>] [--chat <room>] [--by <name>]`
files a document from a shell and prints its number and the permanent link to hand to a reader, a line each,
then a line for each file it attached, naming the attachment's number and its name.

| Option | What it does |
|---|---|
| `--title <words>` | Its title; required |
| `--body <file>` | Its words, as markdown: a `record` |
| `--html <file>` | An HTML page as it stands: a `page`; exactly one of `--body` and `--html` is named |
| `--file <path>` | A file attached to the document filed or rewritten, under its own name, the last part of its path; named again for each file |
| `--replace <id>` | Rewrites that document under its number, which keeps its link and its attachments, in place of filing a new one |
| `--chat <room>` | The room it is held under as its `scope`; it is the owner's own where left out |
| `--by <name>` | Who files it: `BMO_TURN_AGENT` where left out, then `USER` |

- **It acts for the account `BMO_OWNER_ACCOUNT` holds,** naming it in `Owner-Key` on every call, so a document a
  supervisor of an account files is its user's, and a rewrite reaches only what that user owns (「Readers」). Where
  the variable is unset or empty it names no account, and files a document with no owner. No option names the
  account.

- **A new document goes through `POST /publications`,** a rewrite through `PATCH /documents/{documentId}` and
  `POST /shares`, which answers the live link that stands.
- **Each file is attached through `POST /documents/{documentId}/attachments`** once the document is filed
  or rewritten, in the order the files are named, its type the one the service reads from its first bytes and
  its name (docs/artifact.md, 「Attachments」). A body draws a picture so attached by that name.
- **A file is refused before anything is filed** where it cannot be read, where it weighs over the ceiling of
  one file, where two files share a name, or, with `--replace`, where the document already keeps a file of
  that name, which it reads through `GET /documents/{documentId}/attachments`: a kept file is never replaced, so a changed file is attached under a new name, which the body
  names.
- **A picture the body does not draw is refused, and nothing is filed.** A document's pictures stand in its
  body (docs/artifact.md, 「Attachments」), so every picture handed with a markdown body is one the body draws by
  its name, as a markdown picture whose target is that name. The refusal names each picture left out and the
  form that draws it. A page draws what it carries inside itself, so a file handed with `--html` is held to
  nothing here, and a file that is no picture is attached whether or not the body names it.
- **It speaks no narration.** A narration is a sound a client attaches (`POST /documents/{documentId}/attachments`).

## The client

`bmo_artifact::client` is the service's own Rust client: one method for each route, sending and reading
the types of `bmo_artifact::wire`, which the routes answer with and the OpenAPI document is generated from.
A refusal comes back as the route's status with its `code` and `error`. A call waits for its answer however
long the service takes, and ends only with the answer or a refused or broken connection. The command and the
MCP server reach the service through it. The console bundle reads the TypeScript types generated from `docs/openapi.json`.

## The harness plugin

The plugin is `bmo-artifact`: the MCP server `bmo-artifact mcp` starts and the skill
`skills/bmo-artifact/SKILL.md`, which tells a harness when and how to use it. The server reaches the
service at `--address`, or at the settings' `server.listen` where none is given, with the settings' token.
Each tool is one call of the client, answered as structured content; a refusal is a tool result marked
`isError` whose text is the route's status, `code` and `error`. A harness publishes what it wrote with
`artifact_publish`, which hands it back the document's number and the link to hand to a reader; finds earlier
documents with `artifact_search`; and opens a link on a document already filed with `artifact_share`.

**The server acts for the account `BMO_OWNER_ACCOUNT` holds in the environment it is started in,** naming it in
`Owner-Key` on every call as the archive verb does (「The archive verb」): a supervisor of an account files its
user's documents and finds and reads only what its user reads. Where the variable is unset or empty it names none,
and reaches every document with the settings' token. No tool's input names the account.

**`artifact_attach` refuses a picture the document's body does not draw,** as the archive verb does (「The
archive verb」): before it attaches a picture to a markdown document it reads the body as filed, and a picture
the body draws by no markdown picture of that name is refused `bad-request` with the form that draws it, and
nothing is attached. So the body is filed, or rewritten, naming its pictures first, and the pictures attached
after. A page's files and every file that is no picture are attached as handed.

| Tool | Calls | Input | Output |
|---|---|---|---|
| `artifact_documents` | `GET /documents` | the route's query | a page of documents |
| `artifact_file` | `POST /documents` | `body`, the document's input | the document |
| `artifact_publish` | `POST /publications` | `title`, `description?`, `body` or `path`, `kind?`, `scope?`, `actorName` | the document and its permanent link |
| `artifact_document` | `GET /documents/{documentId}` | `documentId` | the document |
| `artifact_rewrite` | `PATCH /documents/{documentId}` | `documentId` and the route's body | the document |
| `artifact_content` | `GET /documents/{documentId}/content` | `documentId` | `content`, the file in base64, and `mediaType` |
| `artifact_attach` | `POST /documents/{documentId}/attachments` | `documentId` and the route's body | the attachment |
| `artifact_shares` | `GET /shares` | the route's query | a page of links |
| `artifact_share` | `POST /shares` | the route's body | the link |
| `artifact_unshare` | `DELETE /shares/{token}` | `token` | an empty object |
| `artifact_import` | `POST /imports` | the route's body | the import |
| `artifact_search` | `GET /documents` | `q`, `kind?`, `scope?` | a page of documents |

- **`artifact_publish`'s words come as `body`, or as the `path` of a file on the machine the harness runs
  on,** read whole and refused over the size one call may carry. Its link is absolute to the share address the
  settings name, so the harness hands the reader what the service answered and adds nothing to it.

## The console

The document screens are a console bundle by bmo-ui's console plugin contract (bmo-ui's
`docs/console-bundles.md`), the one statement of what a bundle, its manifest and its guard are. The service
serves its console face through bmo-chat's `bmo-console-face` (bmo-chat's `docs/console-face.md`), which
answers the manifest, the bundle's files and their guard, and lays the screens' guard over the service's own
routes. The bundle is built from `client/` (docs/layout.md, 「The console bundle」). This service uses these
parts of the contract: a share page with its head, a former screen path, the console's title bar and the
screen's own row, and the console's translator. What follows is what this service hands the face.

```mermaid
flowchart TD
  D["the declaration:<br/>this service's choices"] --> F["bmo-console-face"]
  B["the bundle folder<br/>the build of client/ writes"] --> F
  T["the settings'<br/>console token"] --> F
  F --> M["the manifest and<br/>the bundle's files"]
  F --> G["the screens' guard<br/>over this service's routes"]
  F --> H["the share head's<br/>words (「Share links」)"]
```

- **The declaration.** The bundle `artifact`, its menu title the key `artifact.title.screen`, its icon, its
  screens under `/artifact`, the former screen path `/archive`, the service `artifact`, the role `member`,
  and the API prefix left at its default. No screen needs the owner key, reads the event stream or opens a
  socket, and no event is raised as a notice.
- **The app.** The service name `artifact` is the key of the Artifact app the screens are, which a person's
  settings switch on and off on the account service's delegation chain. The service ships no tool, so its
  deployment publishes the app from `deploy/app.toml`, the `[app]` table alone, offered and on by default, with
  `bmo pocket app` once the service answers ready (bmo-pocket's `docs/api.md`, 「An app on the delegation
  chain」).
- **The share reads** are the share link's own read, `GET /shares/{token}`, and every read of what it opens
  under it (「Share links」), the reading's attachment read and the player's script (「Readings」); the share
  head is `GET /shares/{token}/head`, which is no share read.
- **The calls** are the other services the settings' `console.calls` names (「What this service calls」), none
  where they name none; the service names no other service itself.
- **The bundle folder** is the one the settings' `console.bundle` names, holding the module `bundle.js`, the
  stylesheet `bundle.css` and the words `words.json` beside what bmo-ui's bundle build writes there.
- **Where no face stands.** With no console token set, or where the face refuses the folder or the
  declaration when the service starts, the service runs with no face: `serve` says why, the console's token
  is refused 401 `unauthorized` on every route, and the face's routes are `no-route` to the service's token,
  so a console leaves the service out.
- **The routes the screens use** are the reads of documents, their content, pictures, attachments,
  references, kinds and label values, the keeping of a read and the documents most read, the reading, making and ending of share links, the reading, granting,
  changing and taking back of member shares, and the reading of the owner's company's units and the reading, granting
  and taking back of unit shares; the share reads and the share head are reached as well. Past the
  face's guard, each route reads for the reader 「Readers」 states. The face's guard holds the console's calls to them; the service's token
  reaches them too. Every other route refuses the console's token 403 `forbidden`.

## What this service calls

| Unit | Through | For |
|---|---|---|
| bmo-explorer | its published client crate, `bmo_explorer::client`, at 0.1.0-rev.5 | an import's page and figures; a reference's preview and picture |
| the account service, bmo-web's server | its published client crate, `bmo_web_client`, at 0.1.0-rev.1 | the unit a reader stands in and the units above it; the company a member, a unit and a document's owner stand in; the units of an owner's company |

- **The client is opened over the data root the settings name,** where bmo-explorer keeps its own folder
  (bmo-explorer's `docs/api.md`, 「The client」). It checks every address before it connects and paces each
  host.
- **An import** reads the page's markup with `fetch_page` and each of its pictures with `fetch_raster`.
- **A reference's preview** is `link_preview`, and its picture `fetch_raster`.
- **The account service is reached at the settings' `accounts.address` with `accounts.token`,** its server's
  token (bmo-web's `docs/boundary.md`, 「Wire conventions」), through two reads and nothing else:
  - `GET /users/{user}/unit`, the user's unit, its company and the path from the company down to it: which units a
    reader stands in or under, for a unit share (「Readers」), and the company of a member and of a document's
    owner, for a member or unit share made;
  - `GET /units/{unit}`, a unit with its company, its name and the units directly under it: the company of a unit,
    for a unit share made, and the tree of an owner's company, walked down from the company for the units a
    document may be shared with (「Unit shares」).
  - A 404 is a user or a unit that does not stand; any other failure is refused 502 `upstream-failed`. Nothing it
    answers is kept: each read asks again, so a move or a new unit counts from the next call on.

The console bundle makes two calls of its own, through the console rather than through this service:

| Unit | Through | For |
|---|---|---|
| the console's translator, the gate | bmo-ui's `translation` module, relayed by the console | a document's reading in the reader's language (`docs/artifact.md`, 「Where a person reads it」) |
| the console's people directory | the object the console provides under `bmo-console-people` (bmo-ui's `docs/console-bundles.md`, 「How a bundle mounts」) | the people a document may be shared with, and the name, organization and job title each member stands by (`docs/artifact.md`, 「Member shares」) |

- **The people are the console's to find.** The directory answers the accounts under the top-level organization
  the reader's own unit belongs to; the bundle keeps none of them and the service keeps each member by the account
  key alone, asking the account service only the company of a member it is asked to share an owned document with. Where the console provides no directory, the share dialog shares by link
  alone.

- **The translator is the console's to name.** The console names the service and its translating route in its
  runtime configuration (bmo-ui's `docs/console-bundles.md`, 「Translation」), the gate's `POST /translations`
  wherever the gate translates; the bundle names neither, and where the console names none, nothing is
  translated. The deployment names the same service in `console.calls`, so the manifest names it under
  `calls` and the console relays the screen's call to it.
- **The ask** is `language`, the reader's language tag, and `originals`, the title and the sections of the body
  as markdown; the answer is one reading for each, its `text` and whether it is `whole`. The module carries as
  many originals in one ask, and holds as many asks out at once, as the console names, waits out a busy
  translator, and keeps each whole reading for the life of the tab.
- **The gate keeps every translation it makes** under its original and its language and answers a kept one
  without its translator, so a section is translated once whoever asks for it again.

## Settings

The settings are read at start in layers, each overriding the one before: the repository's
`settings.toml`, then `settings.local.toml` beside it, then `BMO_ARTIFACT_<TABLE>__<KEY>` in the
environment. A path a key names that is relative is read from the directory those settings stand in,
wherever the process was started.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, as `host:port` |
| `server` | `token` | the service's token, which reaches every route |
| `console` | `token` | the console's token, which reaches what the screens use (「The console」) |
| `console` | `bundle` | the folder the build of `client/` writes the bundle into (「The console」); `client/dist` where none is named |
| `console` | `calls` | the other services the screens call through the console, each by the name the console relays it under, which the manifest names under `calls`; none where none is named |
| `player` | `folder` | the folder the build of `client/` writes bmo-ui's audio player script and its words into (「Readings」); `client/player` where none is named |
| `database` | `url` | the PostgreSQL database the document tables stand in |
| `files` | `root` | the folder each document's folder stands under |
| `explorer` | `data_root` | the data root bmo-explorer's client is opened over |
| `accounts` | `address` | the account service's base address (「What this service calls」); none where none is named, and then no reader stands in any unit, and a unit share and a member share of an owned document are refused |
| `accounts` | `token` | the account service's server token, presented as its bearer |
| `share` | `address` | the address a link's `url` stands under |
| `wording` | `file` | the document wording file; the English default in `lang/` where none is named |

## The document wording

- **The lines the service writes into a document a person reads,** an import's line for a figure that
  could not be captured among them, come from the wording file, each filled in by its `{name}` marks. A
  file holds every key the English default holds and no other; the service does not start on one that
  cannot be read, lacks a key or holds one more, and the refusal names the file and the key.

## Missing in siblings

What another repository must gain or settle for this one to stand on it, each for that repository's own
worker:

- **The console that serves this bundle** follows bmo-ui's `docs/console-bundles.md` on the release of bmo-ui
  the bundle names as its layer, and sets this service's share address to the bundle's share page; this
  service answers only callers holding one of its tokens.
- **The callers of the routes this service no longer serves:** research runs and their settings,
  narrations, the holding read by a chat id and the filing verbs are gone. A research run or a narration
  is a client's own work that files through this service's MCP server, and what a caller files is held
  under the scope it writes.
- **The deployment's settings** name the account service's address and its server token under `accounts`, or no
  reader reads through a unit share and no owned document is shared with a member or a unit.
- **Agency's chat line** starts `bmo-artifact mcp` in the supervisor's program, whose environment carries
  `BMO_OWNER_ACCOUNT`; whatever wraps the start keeps that variable, or the supervisor's documents are filed
  with no owner.
- **bmo-ui's share dialog** has a link part and a member part and takes no part a host brings, so the document screen
  offers unit shares in a dialog of its own beside it (docs/artifact.md, 「Where a person reads it」).
