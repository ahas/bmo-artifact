# Layout: bmo-artifact

## Root

One Rust crate at the root, shaped like `lab-app/server` (stage-0-7's reading of
`~/repos/lab-app/server`), plus one client plugin bundle in its own subfolder. No `X/X` nesting —
this repository's folder is already named for what it is (`artifact/` under `bmo-next/`, named
`bmo-artifact` as a repository), so nothing inside repeats that name. No mixed manifests at the
root: `Cargo.toml` is the only manifest at the root; the client's `package.json` lives under
`client/`, one level down, never beside it.

```
Cargo.toml
VERSION
CLAUDE.md
README.md
testbed.toml
build.rs
src/
  lib.rs
  bin/
    bmo-artifact.rs       # the API server
    migration.rs          # sea-orm-migration CLI, as lab-app-server's own migration bin
    archive-move.rs        # the SQLite<->Postgres tool (docs/inventory.md §1)
    html-read.rs           # the ported scripts/html-read.ts (docs/inventory.md §2d)
  entity/                  # SeaORM entities, one per table in docs/inventory.md §4's bmo-artifact rows
  controller/              # Axum routes, one file per docs/api.md section
  service/
    document.rs
    share.rs
    search/                # launch, runner, request, result, document, record, images, import, gallery
    kinds/                 # the five kind modules: search, import, learning, record, page
    translation.rs
    talk.rs
  migration/                # schema + versioned migrations, carried from archive/bmo-archive
  settings.rs               # storage engine choice and file root, both read from settings*.toml
settings.toml
settings.local.toml.example
sql/
tests/
  document.rs
  archive_move.rs
  breaks.rs
  kinds.rs
  share.rs
client/
  package.json
  apps/
    plugin/                 # the console plugin bundle: pages, components, router alias (docs/inventory.md §3a/§3b)
  packages/
    document-kit/           # domain/shared logic specific to documents (docs/inventory.md §3c)
  tests/
```

## Why the console screen and the share page sit in `client/apps/plugin/`, not in bmo-web

`#301`'s section on which units stand as their own service and which do not lists each service's
own plugin bundle as a kind of client, distinct from bmo-web's own client. A worker is fixed per
repository so its prefill never breaks (`#301`'s workers-and-checkouts section); if the archive
screen's Vue files lived in bmo-web's repository instead, a change to this unit's screen would be
a bmo-web diff, reviewed and tested by bmo-web's worker against bmo-web's unrelated code, which is
exactly the entanglement `#301`'s principles section rules out: the same thing used from several
places at once is the wrong structure. Keeping the plugin bundle here means this repository's own
tests cover its own screen, and bmo-web only ever loads the bundle through the plugin contract,
never reads its source.

## Storage

Two settings, both read from `settings.toml` (lab-app-server's own pattern: manifest-adjacent
`settings*.toml`, never a hardcoded path or engine), both behind `src/settings.rs`:

- **Database engine** — SQLite or Postgres, chosen by a `[database] url` setting. `#301`'s
  artifact section explicitly allows this: an agent's runtime data need not live in the shared
  web-console database and may return to SQLite. The `archive/bmo-archive` code already reads
  both engines through one Rust type (`docs/inventory.md` §2a), so this is a setting, not two code
  paths.
- **File root** — the folder holding `searches/<document id>/…` (markdown/HTML files, `images/`,
  `attachments/`), set by a `[storage] root` setting, defaulting to this repository's own data
  directory under whatever runtime data root the deploy declaration names, never a path written
  into source.
- Neither setting changes behaviour against the old bmo: the old repository already reads both
  engines and already keeps files under a runtime data root; only the knob that picks them moves
  from a shared `bmo-policy`/environment-variable scheme into this repository's own settings file.

## Version

`VERSION` at the root holds the semver string, the single source both manifests mirror:
`Cargo.toml`'s `version` field and `client/package.json`'s `version` field are kept equal to it.
`build.rs` embeds `CARGO_PKG_VERSION` into `$OUT_DIR/version.rs`, the same way `lab-app-server`'s
`build.rs` does, so the running server can report its own version. Per `#301`'s deployment-and-
versioning section — a MAJOR change is an API contract change (`docs/api.md` changes shape); a MINOR change is anything
else. Which suite a change runs is decided by that distinction: a MAJOR bump runs this
repository's suite plus every consumer's boundary-contract tests; a MINOR bump runs only this
repository's own suite.

## The deploy declaration

`testbed.toml` at the root, in the neutral shape bmo-testbed reads without knowing this repository
(`#301`'s testbed section: the testbed does not know any one repository, and runs whatever each
repository declares for its own test, build and deploy method in a standard convention):

```toml
[build]
server = "cargo build --release"
client = "cd client && bun install && bun run build"

[test]
server = "cargo test"
client = "cd client && bun test"

[deploy]
kind = "service"
unit = "bmo-artifact.service"

[data]
# paths and tables bmo-testbed's generic backup/restore-drill mechanism includes,
# per docs/inventory.md §4 and §5 — this repository names them, it does not run backup itself.
paths = ["searches/"]
tables = [
  "searches", "search_findings", "search_sources", "search_images",
  "archive_attachments", "web_search_shares", "document_translations", "document_talk",
]
```

## Tests

`tests/` at the Rust root and `client/tests/` under the client's own manifest — never colocated
with runtime code (`#301`'s principles section: each repository's tests sit in its own `tests/`
folder, never beside the runtime code). Fixing this repository runs only `cargo test` here and
`bun test` under `client/`; no other unit's suite runs for a change confined to these two trees.

## Dependency check

Per `#301`'s principles section and its addendum: this repository's `Cargo.toml` and `client/package.json` may
name `bmo-chat`, `bmo-chat-client` and `bmo-ui` among internal dependencies and no other `bmo-*`
package; a CI step fails the build the moment any other internal name appears in either manifest
or in an import, the same check `#301` asks every unit to carry, not a check this document invents
on its own.
