# bmo-artifact

The document archive as one service with its own API. It owns the filed documents: their six tables,
the folder holding each document's files, the searches and research runs that produce them, and the
share links that open them. Every other unit is reached through its API, as `docs/api.md` lists.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The library: the settings and the plumbing every verb and route stands on |
| `src/bin/` | `bmo-artifact`, the HTTP server, and `bmo-artifact-cli`, the command line |
| `tests/` | The tests, the repository checks among them |
| `deploy/` | The neutral service unit template |
| `docs/` | The canonical statements of what the service does |

At the root stand the Cargo manifest, `VERSION`, `build.rs`, `testbed.toml`, `rustfmt.toml`,
`CLAUDE.md`, `settings.toml` and `settings.local.toml.example`, the example of the local settings a
deployment gives.

## Documents

| Document | States |
|---|---|
| `docs/api.md` | The API: the routes and CLI verbs, their answers and refusals, and the calls to other units |
| `docs/archive.md` | The design of the archive, which the API reaches and the layout places |
| `docs/layout.md` | The layout, the plumbing, the settings, the version, the deploy declaration and the checks |
| `docs/research-runs.md` | The research runs a search is made of, their profiles and their branches |
| `docs/inventory.md` | What is taken from the old repository, and what near it is left to the unit owning it |

## Version

- **One version for the whole repository,** in semver. `VERSION` holds it, the package's version
  equals it, and both binaries report it.
- **A major change is a change to the API contract;** anything else is a minor or a patch.

## Commands

```sh
cargo check --all-targets
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin bmo-artifact
cargo run --bin bmo-artifact-cli -- version
```
