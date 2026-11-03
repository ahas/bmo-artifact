# Layout: bmo-artifact

## Root

The root holds one Rust crate and one console bundle. The bundle sits in its own subfolder.

- **No repeated name.** The repository's folder is already named for what it is, so no folder inside
  it repeats that name.
- **One manifest at the root.** `Cargo.toml` is the only manifest there. The bundle's manifest lives
  under `client/`, one level down.

```
Cargo.toml
VERSION
LICENSE
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
    bmo-artifact.rs        the one binary: its line built from every registered command
  commands/                serve, migrate, health, openapi, mcp, nouns, version and html-read, each registered beside its run
  settings.rs              every setting (docs/api.md, "Settings")
  pool.rs                  connection pools and how a statement is bound and run on them
  wire.rs                  the members, bodies and queries the routes answer and take
  openapi.rs               the OpenAPI document, gathered from every route and tool linked in
  client.rs                the service's own Rust client, one method a route
  events.rs                the event kinds and the publisher every change goes through
  reader.rs                the page reader, over bmo-explorer's client
  wording.rs               the document wording file and its English default
  design.rs                the document design read from client/design/document.json, and a page's sheet
  console.rs               the console bundle as the service answers it: its manifest, its files and the guard's floor
  plumbing/                routes, commands, refusals, the bearer check and each route's OpenAPI operation
  store/                   the statements on the document tables, by resource, and the clock their columns keep
    migration/             the migrations and the migrator
  kinds/                   the catalogue, and the five kinds: search, record, learning, import, page
  documents/
    filing.rs              a document filed and rewritten: its row, body, references and file
    keeper.rs              a file kept beside a document, read for what it is
    page_reading.rs        an HTML page read for its title, description and words; a markdown body's description
    import.rs              a page imported whole through the page reader
    preview.rs             the previews and pictures of the pages a document refers to, kept under the file root
    cover.rs               a document's cover picture
    ground.rs              a picture laid on white where its transparency would be lost, and a link's card picture
  http/                    one module a resource, each route registered beside its answer
    files.rs               a kept file sent as kept or as a hosted page, under its isolation
    hosted_page.rs         the sandboxed HTML page, its prelude and its bridges
  mcp/                     the MCP server and its tools, each registered beside its call
  helpers/                 the answer ceiling, a line's marks filled in, and a picture read and kept as AVIF
lang/
  documents.en.yaml        the document wording file's English default
skills/
  bmo-artifact/SKILL.md   the harness plugin's skill
tests/
client/                    the console bundle (see "The console bundle")
  package.json
  console-bundle.json      the declaration a console building the bundle in reads
  bun.lock
  nuxt.config.ts
  module.config.ts         the build of the browser module
  module/                  the module's entry, its stylesheet's entry and what stands in for Nuxt's build there
  dist/                    the module's build, which git ignores
  i18n/                    the bundle's words, one pack a locale
  vendor/                  the packed bmo-ui the bundle's own install takes
  patches/                 bmo-ui's patches to its dependencies, applied in this install
  design/                  the document design the server and the bundle both read
    document.json          the palettes, the page sheet's rules and which pictures open the viewer
    hosted-page.json       the sandbox a kept page runs in and the rest of the policy it is sent under
    bridges/               the clipboard, link, picture, scheme and sound scripts written ahead of a kept page
  app/
    pages/artifact/        the list, one document, and one shared document
    components/            the document body, its text, attachments, references, pictures and page frame
    domain/                the service's calls through the relay, what a screen reads off a document, the text's reading
    assets/css/            the masthead's and the text's stylesheets
  shared/                  the generated API types, the head a share link previews by, and the design files read
  tests/
deploy/
  bmo-artifact.service     the neutral service unit template
```

## Plumbing

The plumbing is what every command and every route stands on, and it knows none of them.

- **Each command registers itself** beside its own run, under the words a caller runs it by. The binary
  builds its command line from every command linked in, and refuses to start where two claim one name.
- **A command** answers the lines it prints, or a refusal said in its own sentence; the binary prints the
  lines on standard output and a refusal on standard error, and ends 0 or 1.
- **The shell** a command stands on is its settings, the database they name, opened on the first
  statement asked of it and settled before the first statement on the document tables, and the
  document wording they name, read when the shell is made. `nouns` stands on none: it is answered
  before the shell is made, so `bmo`'s question reaches the binary run from any directory.
- **A route** declares its method, its path, its answer and its OpenAPI operation: its parameters, its
  body, its answers and the codes it can refuse with. It also declares how far the console's token reaches
  it: not at all, with no role, or as a screen's call, under the role and owner key the console bundle
  needs. Adding a route changes no file but its own; the router and the OpenAPI document gather every route
  linked in, and the server refuses to start where two name one method and path.
- **A call** carries the request and what the server stands on: the settings, the database, the wording,
  the page reader, the preview cache and the event publisher. A route reads its path's keys, its query and
  its JSON body each as its own type, and a request not of that shape is refused `bad-request`.
- **Every call presents a bearer token** the settings name, the service's or the console's; a route answers
  only past that check and, for the console's, past the reach the route declares. A method and path no route
  answers is `no-route`.
- **A refusal** is a status and an English sentence, answered with the code bmo-chat's conventions give
  that status. A failure of the database is one: status 500, `failed`.
- **Each harness tool registers itself** beside its call; the MCP server lists them and the OpenAPI
  document states them, by name.
- **The page reader is an interface,** so a test hands the server a reader of its own and reaches no
  network.

## The console bundle

`client/` is a Bun project of its own, a console bundle by bmo-ui's contract (bmo-ui's
`docs/console-bundles.md`).

- **One set of sources, two builds.** The pages, components, domain modules, stylesheets and words under
  `client/` are built both as the browser module the service answers and as a Nuxt layer on bmo-ui packed
  for a console that builds its bundles in.
- **The browser module.** `bun run module` builds `module/entry.ts`, whose default export is the routes of
  the three screens, into one ECMAScript module, `dist/bundle.js`, its stylesheet, `dist/bundle.css`, and
  its words, `dist/words.json`, each locale's pack under its code, beside bmo-ui's audio player script,
  `dist/bmo-audio-player.js`, copied as the packed bmo-ui carries it. Every import of `vue`, `vue-router`,
  `vue-i18n`, `nuxt/app`, `@nuxt/ui` and bmo-ui stays an import, written as the package's own path; the
  rest, marked and DOMPurify among it, is carried in the module. What Nuxt's build would supply is supplied
  by this one: the Vue and Nuxt functions a file uses without importing them are imported from `vue` and
  `nuxt/app`, a component a template names is imported from its file, the bundle's own, Nuxt UI's or
  bmo-ui's, the link component is made from the console's Nuxt app, and a page's meta is stated on its
  route. The stylesheet is the bundle's own sheets and the utility classes its files use, built against
  bmo-ui's stylesheet without carrying it. The classes its own sheets declare for a document's ground and
  masthead are named under the bundle's name, `artifact-`, so no rule a console keeps under a name of its own
  reaches them: a console's sheets stand above a bundle's, and a rule of the same name there would win.
- **The Nuxt layer.** It extends bmo-ui and adds the same pages, components, domain modules and
  stylesheets. Its declaration, `console-bundle.json`, names the bundle `artifact`, the service `artifact`,
  its screens under `/artifact`, the path `/archive` they stood under before among its former screen paths, the
  read a console asks for a share page's head, and its words in `i18n/`, in the fields the service's manifest
  answers.
- **bmo-ui is a peer.** The manifest names bmo-ui as a peer at the one version the bundle is built
  against, marked optional so no install looks for it in a registry. For the bundle's own install, a
  packed copy kept under `client/vendor/`, named by version, is a development dependency taken from that
  file; moving to another bmo-ui version replaces the copy and both manifest lines together. bmo-ui's
  patches under `client/patches/` apply in this install.
- **Under the console's title bar.** The list's and one document's routes ask for the console's bar in their meta,
  so a console that offers its bar (bmo-ui's `docs/console-bundles.md`, 「How a bundle mounts」, the title bar) draws
  it over them under the manifest's title and icon, and they draw no bar of their own; a document's screen lays the
  way back to the list and the document's number beside the bar's name, in the place the offer names. Found without
  that offer each draws its own bar, a document's holding the same two. The shared document's screen stands under
  no bar either way.
- **Names.** Every component is named with the prefix `Artifact`, every word is a key under `artifact.`
  in the bundle's own pack beside the `common.*` keys the console answers for bmo-ui, and every page
  stands under `/artifact`: the list, one document at `/artifact/<id>`, and one shared document at
  `/artifact/share/<token>`.
- **Calls.** A screen calls this service's routes at the address bmo-ui's `relay` module answers for the
  service `artifact`, and nothing else. That address stands under the console app's base path, so a
  picture, a file or a frame the browser loads by its address reaches the relay as a call does. The types
  it reads are generated from `docs/openapi.json` into `shared/openapi.ts` by the `api` script.
- **A document's text is drawn by the bundle.** The service answers a markdown document's file as filed;
  the bundle's reading, built on marked and cleaned by DOMPurify, turns it into HTML: a heading takes its
  contents mark, a picture or a link naming a kept file by its name is drawn from it or goes to it, a
  picture of any other address is a link, a footnote mark is raised and brings its source entry
  forward in a popover or a dialog (`docs/artifact.md`, 「How a document is drawn」), and bmo-ui's code modules colour code, give fenced blocks their
  copy button and draw diagrams. The reading is loaded in the browser alone, since DOMPurify cleans
  against a real document.
- **Its own files by relative path.** No file of the bundle reaches outside `client/`, which a test in
  `client/tests/` holds by reading the imports of every file the package carries.
- **How a console takes it.** A console that finds services while it runs reads the service's manifest and
  loads the module (docs/api.md, 「The console」). A console that builds its bundles in takes the layer
  packed from `client/` with `bun pm pack`, which prepares Nuxt's types first and writes
  `bmo-artifact-client-<version>.tgz`, carrying the package manifest, the declaration, the words, the Nuxt
  configuration, `app/`, `shared/`, `design/` and `patches/`, and nothing of its own tests, its module's
  build or its copy of bmo-ui; it attaches it by the declaration. The layer's addresses stand under the app's
  base path where the console runs bmo-ui 0.1.0-rev.6 or later.

## Storage

- **Database.** PostgreSQL, at the address the settings name. The document tables, `imports` and
  `events` are this repository's own (`docs/artifact.md`, "Where it lives").
- **Schema.** The migrations run in order: the document tables in the shape the earlier archive declares
  them; the research profiles' table, then the pair that makes and takes away two tables no longer this
  repository's; the redesign, which adds a document's scope, actor, labels, description and change time,
  an attachment's role, and the imports and events; and the research profiles' table taken away. A
  migration a database may already have run is never rewritten or removed, since the migrator refuses a
  database whose record names a migration it does not know. `bmo-artifact migrate` brings the database
  the settings name up to date.
- **Settling** happens once in each process, before the first statement on the document tables
  (`docs/artifact.md`, "The tables"). `migrate` does not settle, since it may run before the tables exist.
- **Statements** bind each number at its widest width, so the driver never refuses a statement it has
  prepared because a value comes in a different width; a statement prepared before a table's shape changed
  under the process is run once more on a fresh connection.
- **File root.** The folder holding `searches/<document id>/` and `web-cache/` is a setting.

## Version

- `VERSION` at the root holds the version string; `Cargo.toml` and `client/package.json` both mirror it.
  `build.rs` embeds it into the binary and fails the build where `Cargo.toml` disagrees with it.
- It is numbered as bmo-chat's `docs/conventions.md` says (「Versions」): before the first release
  `0.1.0`, then `0.1.0-rev.<n>`, one revision a version bump, whatever the change.

## The deploy declaration

`testbed.toml` at the root is the declaration bmo-testbed reads. It names:

- how the suite runs: the Cargo package's tests, and the install a fresh tree's console bundle needs
  before its check runs;
- the static check, in order: the compile check, the linter and the formatter;
- the quick checks for the layout and the dependencies, with the base libraries and bmo-explorer allowed;
- the test database: the migration files that name its template, and the `migrate` command that makes it;
- the one binary a release carries;
- the steps before the flip: the document tables brought to the schema the release carries, and the console
  bundle's module built in the release, into the folder the settings name by default;
- the deployment: the deploy tree under `~/.local/share/bmo-artifact`, the data root it shares with the
  services whose files stand under it, the link that gives a release its local settings, the one unit,
  and the health a deployment waits on.

The service is one unit, `deploy/bmo-artifact.service`, started as `bmo-artifact serve` and marked `bot`:
a deployment links and enables it, restarts it, and is up once `bmo-artifact health` answers ready on the
address the settings name. A stop ends the event streams and lets the calls under way finish,
and the unit waits for the service to end however long that takes, never escalating the stop to a kill. A
release reads its own values from `settings.local.toml`, a link into the data root's `env/artifact.toml`, so
the deployment's backup carries them. A backup of this unit must include the document tables, `imports`,
`events` and the `searches/` folder under the file root; the whole-data-root backup and its restore drill
belong to the testbed.

## Tests

- **Server.** The Rust tests live in `tests/`; no test module stands in `src/`. They are one test target,
  `tests/main.rs`, each file of tests a module of it (bmo-chat's `docs/conventions.md`, 「Rust builds」), but
  the plumbing's: its stand-in verbs and routes register themselves in the process, so they stand in a target
  of their own, `tests/plumbing/main.rs`.
- **Routes.** A route test stands the router up over a scratch PostgreSQL database and a file root of
  its own, with a page reader the test fills, and calls it as a caller would, token and all.
  - The scratch database is made on the server `BMO_TEST_POSTGRES` names: the first test of a binary
    migrates a template with this repository's migrator, each test gets a copy of it, dropped when the
    test ends, and the first template a binary makes drops every template and copy whose process is gone.
- **The MCP server** is started as a harness starts it and held to its twelve tools; one test reaches a
  listening service through it, and one publishes a document from a file.
- **The console** routes are called over a folder of the test's own holding a module, a stylesheet and
  words: the manifest is read through bmo-ui's `consoleManifest` by Bun in `client/`, the module's path
  follows its bytes, and the guard is held to its 401s and 403s on each kind of route.
- **The OpenAPI document** committed is held to what the code generates.
- **Repository checks.** Tests in `tests/` also hold the repository to its rules: no Hangul outside the
  bundle's language pack, no personal data in any tracked file (`PERSONAL-DATA.md`), no folder named after
  its parent, Cargo's manifest as the only manifest at the root, no test module in `src/`, and the dependency
  check below.
- **Client.** The bundle's tests live in `client/tests/`, every one run with happy-dom's document
  registered. The bundle's `check` script runs its typecheck and its tests, and is what the testbed runs
  for it as a project of its own. The files under `client/tests/browser/` drive a real Chromium through
  playwright-core, and are skipped where the browser it pins is not on the machine.

## Dependency check

- **Allowed.** `Cargo.toml` and `client/package.json` may name `bmo-chat`, `bmo-chat-client` and `bmo-ui`
  among internal dependencies, and `bmo-explorer`, the one unit this service calls, taken from its checkout
  beside this repository's; no other `bmo-*` package.
- **Enforced.** A test in `tests/` fails the moment any other internal name appears in any `Cargo.toml` or
  `package.json` of the repository, or as the first segment of a path in a Rust file. A manifest's path
  dependency that reaches outside the repository fails it too, but for bmo-explorer's. The same test holds
  `client/package.json` to naming bmo-ui as an optional peer, and to taking it for its own install from a
  file inside the repository.
- **Other units.** Every other unit is reached through the calls `docs/api.md` lists.
