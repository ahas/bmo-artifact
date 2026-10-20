---
name: bmo-artifact
description: Keep, find and share documents with the bmo-artifact MCP server. Use when a task should leave a document a person reads later (a record, study notes, an HTML page, a web page brought in whole), when earlier documents have to be found again by their words, when a file such as a picture or a sound belongs beside a document, or when someone has to be handed a link that opens a document; and when building or checking a caller of the artifact service.
---

# bmo-artifact

The artifact service keeps documents: markdown records and learning books, HTML pages kept as they stand,
and web pages imported whole. Each document has its file, the files attached to it, the web pages it
refers to and, on request, a share link that opens it. The `bmo-artifact` MCP server reaches the service
through its own client, one tool for each route a harness needs.

## Starting the server

Register the server with the harness as a stdio MCP server:

```
bmo-artifact mcp
```

It reaches the service at the settings' `server.listen` with the settings' token. `--address <url>` names
another service; the token is still the settings' (`BMO_ARTIFACT_SERVER__TOKEN` sets it from the
environment).

## Filing

- `artifact_file` takes `body`, the document: `kind`, `title`, `actorName` (who files it, any name the
  caller uses for itself), `labels` (an object of strings, `{}` for none), an optional `scope` (the
  caller's own grouping string, such as a project), and either `body` or `html`.
- `record` and `learning` take a markdown `body`; `page` takes `html` and no body, and its `title` may be
  empty where the page has a `title` element. `import` is written by imports, not filed by hand.
- Every web link the markdown makes becomes one of the document's references.
- `artifact_rewrite` rewrites a document under its number: `documentId`, `labels` (required, they replace
  the labels whole) and any of `title`, `body`, `html`. Its attachments and share links stay.

## Attaching files

`artifact_attach` keeps a file beside a document: `documentId`, `name`, `mediaType`, `content` in base64
and an optional `role`. A body draws a kept picture by naming it: `![a caption](name.png)`, so file the
body first, then attach the pictures it names. `role: "head"` marks a file a reader is shown at the
document's head, a narration's sound among them. A name the document already keeps is refused; one file is
at most 64 MiB.

## Finding and reading

- `artifact_search` finds documents by a phrase in their title, labels or body, newest first, narrowed by
  `kind` and `scope` where given.
- `artifact_documents` lists documents by `kind`, `scope`, `label` (`key:value`, or `key` alone), `q`,
  and pages with `before` or `after` (a document's `id`) and `limit`; `more` says whether more stand past
  the page.
- `artifact_document` answers one document's head; `artifact_content` its file, `content` in base64 with
  its `mediaType`: the markdown as filed, or the page's HTML.

## Sharing

`artifact_share` makes a link for `documentId`, ending at `expiresAt` (epoch milliseconds) or never where
none is given; asking again for the same document answers the link that stands. Hand the reader the
link's `url`. `artifact_shares` lists the live links, by `documentId` where given; `artifact_unshare` ends
one by its `token`.

## Importing a web page

`artifact_import` takes `url`, `actorName` and optional `title` and `scope`, and answers the import at
once, `fetching`. The page is read in the background: its main content becomes a markdown document of kind
`import`, its pictures are kept as its files, and the page itself and the outside pages it links to are its
references. The import moves to `filed`, with `documentId`, or to `failed`, with `error`. The tools do not
read an import back; list the documents of its `scope` or search for its title to find what it filed.

## Refusals

A refused call comes back as a tool error whose text is the service's `status`, `code` and `error`:
`bad-request` for input not of the tool's shape (the sentence says what), `not-found` for a document,
attachment or link that does not stand, `conflict` for a body given to a page or HTML to any other kind,
`too-large` for a file over its ceiling, and `unauthorized` where the token is wrong. A call that reached no
service carries only `error`.
