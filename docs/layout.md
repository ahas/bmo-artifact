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
build.rs
settings.toml
settings.local.toml.example
deploy.toml
src/
  lib.rs
  main.rs                  the one binary: the HTTP server and every CLI verb
  settings.rs              storage address, file root, wording file, time zone
  pool.rs                  connection pools
  store/                   schema, migrations and the queries the carried code runs
  plumbing/                errors, refusals, CLI and HTTP answers
  helpers/                 copied helpers and the five Rust twins
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
- **File root.** The folder holding `searches/<document id>/` is a setting. Each document's folder
  holds its markdown or HTML file, `images/` and `attachments/`. The default lies under the data
  root the deploy declaration names.

## Settings

Every setting lives in `settings.toml`, overridden by `settings.local.toml` and then by the
environment. `src/settings.rs` reads them all.

- **Storage address and file root**, as above.
- **The document wording file's path.** Sentences a room reads come from this file:
  - a filed document's reply;
  - a search's notices.

  The English default ships in `lang/`. bmo-web's language pack supplies the live wording under the
  same keys.
- **The time zone** used when a sentence or a document states a time.
- **The research profiles and their seeds**, which carry the old research policy files.
- **The address of each unit this repository calls**, as `docs/api.md` lists them.

## Version

- `VERSION` at the root holds the version string. `Cargo.toml` and `client/package.json` both mirror
  it.
- `build.rs` embeds the version into the binary, so the running server can report its own version.
- A change to the shape of `docs/api.md` is a major change. It runs this repository's suite and
  every consumer's boundary-contract tests.
- Any other change is a minor change, and runs only this repository's suite.

## The deploy declaration

`deploy.toml` at the root is the declaration bmo-testbed reads. It is written in the testbed's
neutral convention, so the testbed needs no knowledge of this repository. It names:

- how to build the server and the client;
- how to test the server and the client;
- that the deployment is one service, `bmo-artifact.service`;
- what a backup of this unit must include:
  - the six document tables;
  - the `searches/` folder under the file root.

This repository runs no backup itself. The whole-data-root backup and its restore drill belong to
the testbed.

## Tests

- **Server.** The Rust tests live in `tests/`, including the three carried integration tests and
  the test modules that moved out of their files.
- **Client.** The client tests live in `client/tests/`.
- **Scope.** A change confined to these two trees runs only `cargo test` here and the client's test
  command under `client/`.
- **Database.** A test that needs a database gets a scratch PostgreSQL database of its own.

## Dependency check

- **Allowed.** `Cargo.toml` and `client/package.json` may name `bmo-chat`, `bmo-chat-client` and
  `bmo-ui` among internal dependencies, and no other `bmo-*` package.
- **Enforced.** A check fails the build the moment any other internal name appears in either
  manifest or in an import.
- **Other units.** Every other unit is reached through the calls `docs/api.md` lists.
