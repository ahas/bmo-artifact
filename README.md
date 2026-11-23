**English** | [한국어](README.ko.md) | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# bmo-artifact

Documents kept, searched and shared, as one service with its own API: records, learning books,
imported web pages and HTML pages, each with its files, the pages it refers to, its share link and the
members of the reader's company it is shared with to read. A
harness reaches it through its MCP server, `bmo-artifact mcp`, and the `bmo-artifact` skill; any other
caller through its HTTP API and its Rust client. It reads web pages through bmo-explorer's client and
calls no other unit.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The library and `bmo-artifact`, the one binary: the routes, the store, the client, the MCP server and the commands |
| `lang/` | The document wording file's English default |
| `skills/` | The `bmo-artifact` skill |
| `client/` | The console bundle |
| `tests/` | The tests, the repository checks among them |
| `deploy/` | The neutral service unit template |
| `docs/` | The canonical statements of what the service does |

At the root stand the Cargo manifest, `VERSION`, `build.rs`, `testbed.toml`, `rustfmt.toml`,
`CLAUDE.md`, `PERSONAL-DATA.md`, `LICENSE`, `settings.toml` and `settings.local.toml.example`, the example
of the local settings a deployment gives.

## Documents

| Document | States |
|---|---|
| `docs/api.md` | The API: the routes, the events, the commands, the client, the MCP tools and the call to bmo-explorer |
| `docs/openapi.json` | The API as an OpenAPI 3.1 document, generated from the code |
| `docs/artifact.md` | The design of the documents, which the API reaches and the layout places |
| `docs/layout.md` | The layout, the plumbing, the console bundle, the storage, the version, the declaration and the checks |
| `PERSONAL-DATA.md` | The personal data no file may hold, and the check that holds it |

## Version

- **One version for the whole repository,** `0.1.0` and then `0.1.0-rev.<n>` before the first release.
  `VERSION` holds it, the crate and the console bundle's package equal it, and the health route, the
  OpenAPI document and the MCP server answer it.

## Commands

```sh
cargo check --all-targets
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin bmo-artifact -- serve
cargo run --bin bmo-artifact -- mcp
cargo run --bin bmo-artifact -- openapi
```
