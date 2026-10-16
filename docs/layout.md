# Layout: bmo-artifact

## Root

The root holds one Rust crate and one console plugin bundle. The bundle sits in its own subfolder.

- **No repeated name.** The repository's folder is already named for what it is, so no folder inside
  it repeats that name.
- **One manifest at the root.** `Cargo.toml` is the only manifest there. The client's manifest
  lives under `client/`, one level down.

```
Cargo.toml
VERSION
README.md
CLAUDE.md
build.rs
rustfmt.toml
settings.toml
settings.local.toml.example
testbed.toml
src/
  lib.rs
  bin/
    bmo-artifact.rs        the HTTP server
    bmo-artifact-cli.rs    the CLI's line, built from every registered verb, and the version and migrate verbs
  settings.rs              every setting (see "Settings")
  pool.rs                  connection pools and how a statement is bound and run on them
  store/                   the queries the carried code runs, and the clock its columns keep
    migration/             the six document tables' migration and its migrator
    entity/                one SeaORM entity per document table
  plumbing/                errors, refusals, CLI and HTTP answers
  helpers/                 copied helpers and the Rust twins
    js.rs                  JavaScript's white space and a text's characters
    web_address.rs         the web addresses a text names
    marks.rs               a wording line's {name} marks filled in
    ceiling.rs             the answer ceiling and its counts
    lines.rs               a launch's trees, its required documents and their {{name}} marks
    toolbook.rs            a toolbook proxy call read as the tool it reached
    run_log/               the run log's JSON and the lines a run's stream leaves
    picture/               a picture's type read off its bytes, and a kept picture written as AVIF
  wording.rs               the document wording file and its English default
  clients/                 bmo-job, bmo-agency, bmo-voice, bmo-receiver, and the tool host for bmo-explorer
  verbs/
    archive_docs.rs        archive doc and archive share
    html_read.rs           archive html-read
    search/                launch, runner, request, result, document, record, images, gallery, import, share, page reading
    research/              profiles, admission, branch, run
  control/
    research.rs            research profiles and their checks
    research_settings.rs   the research-settings routes
  http/
    documents.rs           list, read, file, attachments, source previews and pictures, moved screens
    hosted_page.rs         the sandboxed HTML page and its bridges
    shares.rs              share links and the token routes
    rooms.rs               the room-holding answer
  kinds/                   the five kinds: search, import, learning, record, page
lang/
  documents.en.yaml        the document wording file's English default
tests/
client/
  package.json
  app/                     pages, components and domain code of the archive screen
  shared/
  tests/
deploy/
  bmo-artifact.service     the neutral service unit template
```

The carried Rust files keep their old module names under `verbs/` and `control/`, so a diff against
the old repository reads file by file. The `http/` and `kinds/` modules are the Rust port of the
console's TypeScript server side. Their behaviour, cases included, follows the TypeScript files that
`docs/inventory.md` lists for steps 10 and 11.

## Plumbing

The plumbing is what every verb and every route stands on, and it knows none of them.

- **Each verb registers itself** beside its own answer, under the words a caller runs it by and any
  other spelling it keeps. The CLI binary builds its command line from every verb linked in, and
  takes a verb's two words given as two arguments as the one name. The binary refuses to start where
  two verbs claim the same name or spelling.
- **A verb** answers the lines it prints, or a verb error. A verb error is one of three:
  - a refusal, said in the verb's own sentence;
  - an end with a code of the verb's own, said in its own sentence;
  - a partial answer: the lines of what was done, and the refusal of the rest.

  The CLI binary prints the lines on standard output and a refusal on standard error. It ends with
  success, with the verb's own code for an end, and with failure otherwise.
- **A refusal of the carried code** is a status and an English sentence. A failure of the database
  is one: status 500, with "the archive's database" and the failure as its sentence. A verb ends
  with failure on it and prints its sentence. A route answers with its status, and the code is that
  status's own name in snake_case.
- **The shell** is what a verb run from the CLI stands on: the settings it was started with, the
  database they name, the document wording they name, the bmo-voice client, and the page reader that
  reaches bmo-explorer through the tool host. The database is
  opened when a verb asks its first statement of it, and settled (see "Storage") before the first
  statement on the document tables. The wording is read when the shell is made, so a wording file that is
  refused stops every verb before it runs. Each later piece a verb needs (a client of another unit)
  is added to the shell, not opened by the verb.
- **A route** declares its method and its path and answers one call. A call carries the request,
  its query and its JSON body, and what the server stands on. A route answers a response or a
  refusal: a status, a snake_case code and an English message.
- **Each route registers itself** beside its own answer, so adding a route changes no file but its
  own. The server refuses to start where two routes name the same method and path.
- **What the server stands on** is its settings and the database and the document wording they
  name, opened and read the same way as the shell's.

## Why the console screen sits in `client/`, not in bmo-web

The split plan lists each service's own plugin bundle as a kind of client, distinct from bmo-web's
own client. A worker is fixed per repository. If the archive screen lived in bmo-web, a change to
it would be a bmo-web diff, reviewed and tested against bmo-web's unrelated code. Keeping the bundle
here means this repository's tests cover its own screen. bmo-web loads the bundle through the plugin
contract and never reads its source. The screen's wording keys stay in bmo-web's language pack.

## Storage

- **Database.** PostgreSQL, as today. The address is a setting, read from the settings files or the
  environment, and is never written into source. The six document tables are this repository's own.
  No other unit's table is read or written.
- **Schema.** One migration makes the six tables and their four indexes in the shape the old archive
  declares for them. It also gives `web_search_shares` the nullable `user_id` column that the old
  archive added. In the old archive that column's default is read from another unit's settings
  table; here it has no default, since no other unit's table is read. SeaORM keeps the migration's
  record in its own table beside the six. The `migrate` verb brings the database the settings name
  up to date with this migration.
- **Settling.** Once in each process, before the first statement on the document tables, a row that
  names no kind is given its kind and its writer, and the pictures of `search_images` not yet among
  the attachments are copied there, as `docs/archive.md` sets out under "`searches`" and "The other
  five tables". `migrate` does not settle, since it may run before the tables exist.
- **Statements.** Every statement binds each number at its widest width, so the driver never refuses
  a statement it has prepared because a value comes in a different width. A statement prepared
  before a table's shape changed under the process is run once more on a fresh connection, and no
  connection opened before that change is handed out again.
- **File root.** The folder holding `searches/<document id>/` is a setting. Each document's folder
  holds its markdown or HTML file, `images/` and `attachments/`. The default lies under the data
  root the deploy declaration names.

## Settings

Every setting lives in `settings.toml`, overridden by `settings.local.toml` and then by the
environment. `src/settings.rs` reads them all.

- **Where they are read.** Both files are read from the directory the process starts in. Git
  ignores `settings.local.toml`; `settings.local.toml.example` shows its shape.
- **The environment.** A variable named `BMO_ARTIFACT_`, then the table, two underscores and the
  key, sets that key: `BMO_ARTIFACT_DATABASE__URL` sets the database address.
- **The listen address** of the HTTP server.
- **Storage address and file root**, as above.
- **The document wording file's path,** `file` under `[wording]`. Sentences a room reads come from
  this file:
  - a filed document's reply;
  - a search's notices.

  Where no path is set, the English default in `lang/` is read. bmo-web's language pack supplies the
  live wording under the same keys, which `docs/api.md` lists under "Wording".
- **The time zone** used when a sentence or a document states a time, `zone` under `[time]`, by its
  IANA name. The committed default is UTC; a deployment sets its own.
- **The documents the agent files**, under `[documents]`:
  - `room`, the owner's private room, which is also the room a document goes to where none is named;
  - `asker`, who asked where nobody is named;
  - `writer`, the writer name a document the agent writes is recorded under;
  - `sender`, the name a share link the verbs make is recorded as made by;
  - `kindless_import_prefix`, the prefix that marks a row naming no kind as an import.

  `writer` and `sender` are committed. The other three have no committed value: a deployment sets
  them.
- **The console's public address**, `address` under `[console]`. A document's console address and
  its share links stand under it. No value is committed.
- **The research profiles and their seeds**, which carry the old research policy files.
- **The address of each unit this repository calls**, under `[units]`, by the unit's short name:
  `voice` for bmo-voice, and `tool_host` for the tool host that bmo-explorer's page reading is
  reached through. `docs/api.md` lists the calls. No address is committed.

## Version

- `VERSION` at the root holds the version string. `Cargo.toml` and `client/package.json` both mirror
  it.
- `build.rs` embeds the version into both binaries, so each can report its own version, and fails
  the build where `Cargo.toml` disagrees with `VERSION`.
- A change to the shape of `docs/api.md` is a major change. It runs this repository's suite and
  every consumer's boundary-contract tests.
- Any other change is a minor change, and runs only this repository's suite.

## The deploy declaration

`testbed.toml` at the root is the declaration bmo-testbed reads, in the schema of the testbed's own
declaration document, so the testbed needs no knowledge of this repository. It names:

- how the suite runs: the Cargo package's tests;
- the static check, in order: the compile check, the linter and the formatter;
- the quick checks for the layout and the dependencies, with the three base libraries allowed;
- the test database: the migration files that name its template, and the `migrate` verb that makes
  it;
- the two binaries a release carries.

It declares no deployment root, so a deployment of this repository puts nothing live. The service
is one unit, from the neutral template `deploy/bmo-artifact.service`. A backup of this unit must
include the six document tables and the `searches/` folder under the file root. This repository runs
no backup itself. The whole-data-root backup and its restore drill belong to the testbed.

## Tests

- **Server.** The Rust tests live in `tests/`, including the three carried integration tests and
  the test modules that moved out of their files. No test module stands in `src/`.
- **Repository checks.** Tests in `tests/` also hold the repository to its rules: no Hangul in code
  or tests, no folder named after its parent, Cargo's manifest as the only manifest at the root, no
  test module in `src/`, and the dependency check below.
- **Client.** The client tests live in `client/tests/`.
- **Scope.** A change confined to these two trees runs only `cargo test` here and the client's test
  command under `client/`.
- **Database.** A test that needs a database gets a scratch PostgreSQL database of its own. It is
  made on the server `BMO_TEST_POSTGRES` names, which the suite sets to its own test server.
  - The first test of a binary that asks for one migrates a template with this repository's
    migrator.
  - Each test gets a copy of the template, which is dropped when the test ends.
  - The first template a binary makes drops every template and copy whose process is gone.
- **Verbs.** A verb's test runs the CLI binary from the repository's root, with the settings it needs
  given in the environment over the committed ones: the scratch database, a file root of the test's
  own, and the rooms, console address and unit addresses the case is about. A unit the verb calls is
  a stand-in the test serves on a local port.
- **Page reading.** Every piece of the search line that reads a page or fetches a picture is handed
  the page reader as an interface, so its tests hand it a fake reader and reach no tool host.

## Dependency check

- **Allowed.** `Cargo.toml` and `client/package.json` may name `bmo-chat`, `bmo-chat-client` and
  `bmo-ui` among internal dependencies, and no other `bmo-*` package.
- **Enforced.** A test in `tests/` fails the moment any other internal name appears in any
  `Cargo.toml` or `package.json` of the repository, or as the first segment of a path in a Rust
  file. A manifest's path dependency that reaches outside the repository fails it too.
- **Other units.** Every other unit is reached through the calls `docs/api.md` lists.
