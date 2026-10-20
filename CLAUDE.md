# Working in bmo-artifact

These rules hold every change to this repository.

## The documents come first

- `docs/` is the canonical statement of what the service does; the code is one realisation of it.
  Before changing code, read the document covering that area.
- A change that would make a document wrong or incomplete repairs the document first, stating what is
  true now, and the code is written to match.
- A document states what is true now: no change history, no rationale told through an earlier
  design, no pasted source, signature or config fragment, and no file-and-line pins.

## Boundaries

- **Every other unit is reached through its published client,** never through its code, its tables or
  its files. The routes this service serves and every call it makes to another unit are listed in
  `docs/api.md`.
- **Dependencies:** crates from the registry pinned in `Cargo.toml`, and among other repositories only
  the base libraries `bmo-chat`, `bmo-chat-client` and `bmo-ui`, and the client of each unit
  `docs/api.md` says this service calls. The dependency check in `tests/` holds this.
- **The plumbing knows no command, no route and no tool.** A route registers itself beside its own
  answer and its OpenAPI operation, a harness tool beside its call, and a command takes what it needs
  from the shell. Adding one changes no file but its own.
- **The OpenAPI document follows the code.** `docs/openapi.json` is what `bmo-artifact openapi` prints,
  and the console bundle's `shared/openapi.ts` is generated from it; a change to the API regenerates
  both.
- **No lookup table standing in for polymorphism.** A value that differs per member belongs to that
  member.
- **No hand-written parser for a general format.** TOML, JSON, HTML, markdown, command lines and their
  kin are read by a maintained library.

## Language

- Everything in this repository is English: code, comments, strings, tests, documents, commit messages
  and branch names. No Korean in code or tests; the Hangul check in `tests/` holds this.
- What the service answers is an English sentence for a person, or a status and a kebab-case code for
  a machine. Words a person reads in another language come from the wording file and the console
  bundle's language pack, the one place another language stands.
- No personal data: no user names, home directories, machine names, internal addresses, mail addresses or
  names of any agent (`PERSONAL-DATA.md`). Names the service needs at runtime come from its settings, never
  from the code.

## Layout

- `Cargo.toml` is the only manifest at the root, and no directory is named after its parent; the
  layout check in `tests/` holds both.
- Tests live in `tests/`. No test module stands in `src/`.

## The stages of a change

1. **Writing:** the only check is `cargo check --all-targets`.
2. **Review:** read the whole diff against the document governing it, then `cargo fmt --all --check`
   and `cargo clippy --all-targets -- -D warnings`.
3. **Verification:** the tests touched, then `cargo test` once, output written to a file and the file
   read.

A change is not done before stage 3 has passed.
