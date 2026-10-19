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
    migration/             the document tables' and the research profiles' migrations, the pair that makes and drops two old tables, and the migrator
    entity/                one SeaORM entity per document table
    shares.rs              the share link rows, the one module that writes them
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
  clients/                 bmo-agency, bmo-voice, bmo-receiver, bmo-gate, and the tool host for bmo-explorer
  verbs/
    archive_docs.rs        archive doc and archive share
    html_read.rs           archive html-read
    search/                launch, runner, request, result, document, record, images, gallery, import, share, page reading
    research/              profiles, admission, branch, run
  control/
    research.rs            research profiles and their checks
    research_settings.rs   the research-settings routes
  design.rs                the document design read from client/design/document.json, and a page's sheet
  documents/
    read.rs                the list, one document, its files and its refs, as the read API answers them
    preview.rs             the previews and pictures of the pages a document points at, kept under the file root
    ground.rs              a picture laid on white where its transparency would be lost, and a link's card picture
  markup/
    markdown_structure.rs  a markdown text's headings and sections
    finding_markdown.rs    a finding's structure lines as the markdown the console draws
    page_head.rs           the head a kept page declares, which a link to it previews by
  http/
    documents.rs           list, kinds, read, file, attachments, source previews and pictures, moved screens
    files.rs               a kept file sent as kept or as a hosted page, under its isolation
    hosted_page.rs         the sandboxed HTML page, its prelude and its bridges, and the head it declares
    shares.rs              share links and the token routes
    rooms.rs               the room-holding answer
  kinds/                   the catalogue, and the five kinds: search, import, learning, record, page
lang/
  documents.en.yaml        the document wording file's English default
tests/
client/                    the console plugin bundle (see "The console plugin bundle")
  package.json
  bun.lock
  nuxt.config.ts
  vendor/                  the packed bmo-ui the bundle's own install takes
  patches/                 bmo-ui's patch to a dependency, applied in this install
  design/                  the document design the server and the bundle both read (see "The design files")
    document.json          the palettes, the page sheet's rules and which pictures open the viewer
    hosted-page.json       the sandbox a kept page runs in and the rest of the policy it is sent under
    bridges/               the clipboard, link and picture scripts written ahead of a kept page
  app/
    pages/                 the archive list, one document, and the share page
    components/            the document body and its findings, sources, attachments, pictures and share dialog
    domain/                what the screen reads off a document, a finding's reading, and the catalogue of kinds
    assets/css/            the masthead's and the finding's stylesheets
  host/                    the components the host owes, declared for the bundle's own typecheck
  shared/                  the API's types, the head a share link previews by, and the design files read
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
  database they name, the document wording they name, the bmo-voice client, the tool host client
  that reaches bmo-explorer, and the bmo-agency client. The database is
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
  name, opened and read the same way as the shell's, with the tool host client and the bmo-agency
  client. The bmo-agency client is an interface, so a test hands the server or the shell a stand-in.

## Why the console screen sits in `client/`, not in bmo-web

The split plan lists each service's own plugin bundle as a kind of client, distinct from bmo-web's
own client. A worker is fixed per repository. If the archive screen lived in bmo-web, a change to
it would be a bmo-web diff, reviewed and tested against bmo-web's unrelated code. Keeping the bundle
here means this repository's tests cover its own screen. bmo-web takes the bundle as a package and
extends it, and reaches nothing else of this repository but its API. The screen's wording keys stay
in bmo-web's language pack.

## The console plugin bundle

`client/` is a Bun project of its own: its manifest, its lockfile and its Nuxt configuration stand
there, one level below the root.

- **A Nuxt layer on bmo-ui.** It extends bmo-ui and adds the archive screen's pages, components,
  domain modules and stylesheet. A host extends it the way it extends bmo-ui.
- **bmo-ui is a peer.** The manifest names bmo-ui as a peer at the one version the bundle is built
  against, marked optional so no install looks for it in a registry, which carries none. The host
  supplies it, as it extends bmo-ui itself. For the bundle's own install, a packed copy kept under
  `client/vendor/`, named by version, is a development dependency taken from that file. Moving to
  another bmo-ui version replaces the copy and both manifest lines together. The lockfile holds the
  packages bmo-ui and the bundle share at the versions bmo-ui's own lockfile holds, so bmo-ui's patch
  under `client/patches/` applies.
- **Its own files by relative path.** In a host, Nuxt binds `~`, `@` and `#shared` to the host's
  folders, so no file of the bundle reaches another through them. bmo-ui's modules are imported by
  the package's path.
- **The design files.** The palette, the rule of which pictures open the viewer, the sandbox a kept
  page runs in and the scripts written ahead of a kept page are one set of files, kept in
  `client/design/` inside the bundle's own package. The bundle reads them by relative path, and the
  server compiles them into its binaries from the same folder, so there is one copy and no step
  that makes another. No file of the bundle reaches anything outside `client/`, which a test in
  `client/tests/` holds by reading the imports of every file the package carries.
- **How a host takes it.** The bundle is packed from `client/` with `bun pm pack`, which prepares
  Nuxt's types first and writes `bmo-artifact-client-<version>.tgz`. The package carries what a host
  extends and nothing of the bundle's own tests, its typecheck declarations or its copy of bmo-ui:
  the manifest, the Nuxt configuration, `app/`, `shared/`, `design/` and `patches/`. A host:
  - keeps the packed bundle in its own tree and depends on it by that file;
  - depends on bmo-ui itself, at the version the bundle's peer names, as it takes any base library;
  - names `bmo-artifact-client` in its Nuxt configuration's `extends`, which brings bmo-ui's layer
    with it;
  - installs `@nuxtjs/i18n` as its module, and applies the patch in `patches/` in its own install,
    its lockfile holding the patched package at the version the patch names;
  - registers the components it owes (below).

  A host built this way serves the three pages as they stand in this repository: the design files
  and the stylesheets travel inside the package, so the host's build reads nothing outside its own
  install. A host's install keeps a package taken from a file by the integrity its lockfile holds,
  so a new bundle reaches a host as a new version, or by updating that one package in the host's
  lockfile; reinstalling alone keeps the old one.
- **The host's proxy** hands on `/api/archive` itself as well as every road under it, and every road
  under `/api/share/`.
- **Calls.** The screen calls this repository's read API and share API (`docs/api.md`) under the
  console's own base path. The host proxies them to this service behind its sign-in, and names the
  signed-in user as `by` when a link is made. The share page's roads take no sign-in.
- **A finding is drawn by the bundle.** The read API answers each finding as markdown, its structure
  lines already turned into words and its folds, galleries and maps written as the blocks
  `docs/api.md` names. The bundle's own reading, built on marked and cleaned by DOMPurify, turns
  that markdown into the finding's HTML: a heading takes the outline's mark, a footnote mark becomes
  a raised link, a picture the record keeps is drawn from the console's copy and any other is a
  link, and a block becomes a place the bundle fills with its fold, gallery or map component.
  bmo-ui's code modules colour its code, give its fenced blocks their copy button and draw its
  diagrams. The finding's stylesheet is the bundle's, so the host knows neither the finding's
  syntax nor its design. The reading is loaded in the browser alone, since DOMPurify cleans against
  a real document.
- **What the host owes.** Beside what bmo-ui asks of it:
  - its i18n module, and a language pack answering the keys the screen reads;
  - the console's table of contents, `DocumentToc`;
  - the three pieces of the console's title bar, `DeployMark`, `TodoOpenButton` and `UsageMeters`.

  `client/host/` declares each component by the props the screen hands it, and the bundle's typecheck
  reads the screen once more with those declarations, so it holds the screen to what it may ask of
  the host. The folder stands outside `app/`, so a host extending the bundle reads none of it and
  keeps its own components' types.

## Storage

- **Database.** PostgreSQL, as today. The address is a setting, read from the settings files or the
  environment, and is never written into source. The six document tables and `research_profiles`
  are this repository's own. No other unit's table is read or written.
- **Schema.** Four migrations, run in order:
  - the first makes the six document tables and their four indexes in the shape the old archive
    declares for them. It also gives `web_search_shares` the nullable `user_id` column that the old
    archive added. In the old archive that column's default is read from another unit's settings
    table; here it has no default, since no other unit's table is read;
  - the second makes `research_profiles`, one row per research key, in the shape the old archive
    declares for it;
  - the third makes `news_briefs` and `holidays`, and the fourth takes both away with their rows, so
    a migrated database holds neither. A migration a database may already have run is never
    rewritten or removed, since the migrator refuses a database whose record names a migration it
    does not know.

  SeaORM keeps the migrations' record in its own table beside them. The `migrate` verb brings the
  database the settings name up to date with all four.
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
- **The research settings**, under `[research]` (`docs/research-runs.md`, "Research profiles"):
  - `pool_capacity_most`, the most places one pool limit may hold;
  - `[research.limits]`, the bounds each profile field is checked against, each a `min` and a `max`
    either of which may be left out;
  - `[[research.opens]]`, each capability a run's settings open with the run variable that opens it;
  - `[research.search]`, the rulebooks a search is bound by, those given only on request, and the
    heading of the section that decides whether one applies;
  - `[[research.keys]]`, one per research key in the order the console lists them, each declaring its
    `key`, its `line`, whether it is `clocked` and `branchless`, the `strategies` it accepts, and its
    `[research.keys.seed]`.

  All of it is committed: it is what every deployment's research runs are held to.
- **The prompt tree,** `code` under `[prompts]`: the tree whose `prompts/agents/<agent>/` holds an
  agent's documents, such as a search's request and a branch's. No value is committed; where none is
  set, a search run and a branch are refused with "the prompt tree is not set: set code under
  [prompts] in the settings".
- **The address of each unit this repository calls**, under `[units]`, by the unit's short name:
  `voice` for bmo-voice, `tool_host` for the tool host that bmo-explorer is reached through,
  `agency` for bmo-agency, `receiver` for bmo-receiver and `gate` for bmo-gate.
  `docs/api.md` lists the calls. No address is committed; a call to a unit whose address is unset is
  refused with "the <unit> address is not set: set <key> under [units] in the settings".
- **The tokens bmo-agency's and bmo-receiver's servers take,** `agency_token` and `receiver_token` under
  `[units]`, each of which goes with every call to its unit. No value is committed: a deployment sets them
  in `settings.local.toml` or in the environment.
- **A search's runner reads the same settings as its launcher.** The launcher hands bmo-agency the
  folder it started in and every variable of its own environment that begins with `BMO_ARTIFACT_`,
  with `PATH` and `HOME`, so the runner reads the same files with the same values over them.

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

- how the suite runs: the Cargo package's tests, and the install a fresh tree's console plugin
  bundle needs before its check runs;
- the static check, in order: the compile check, the linter and the formatter;
- the quick checks for the layout and the dependencies, with the three base libraries allowed;
- the test database: the migration files that name its template, and the `migrate` verb that makes
  it;
- the two binaries a release carries.

It declares no deployment root, so a deployment of this repository puts nothing live. The service
is one unit, from the neutral template `deploy/bmo-artifact.service`. A backup of this unit must
include the six document tables, `research_profiles` and the `searches/` folder under the file root. This repository runs
no backup itself. The whole-data-root backup and its restore drill belong to the testbed.

## Tests

- **Server.** The Rust tests live in `tests/`, including the three carried integration tests and
  the test modules that moved out of their files. No test module stands in `src/`.
- **Repository checks.** Tests in `tests/` also hold the repository to its rules: no Hangul in code
  or tests, no folder named after its parent, Cargo's manifest as the only manifest at the root, no
  test module in `src/`, and the dependency check below.
- **Client.** The client tests live in `client/tests/`, every one run with happy-dom's document
  registered. The bundle's `check` script runs its typecheck and its tests, and is what the testbed
  runs for it as a project of its own. The files under `client/tests/browser/` drive a real Chromium
  through playwright-core, and are skipped where the browser it pins is not on the machine.
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
  file. A manifest's path dependency that reaches outside the repository fails it too. The same test
  holds `client/package.json` to naming bmo-ui as an optional peer, and to taking it for its own
  install from a file inside the repository.
- **Other units.** Every other unit is reached through the calls `docs/api.md` lists.
