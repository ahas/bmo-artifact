---
name: bmo-artifact
description: Keep, find and share documents with the bmo-artifact MCP server. Use when a task should leave a document a person reads later (a record, study notes, an HTML page, a web page brought in whole), when a document written for somebody has to be handed over as a permanent link rather than a file, when earlier documents have to be found again by their words, when a file such as a picture or a sound belongs beside a document, or when someone has to be handed a link that opens a document; and when building or checking a caller of the artifact service.
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

Where the environment it starts in holds `BMO_OWNER_ACCOUNT`, as a supervisor of an account's program does,
the server acts for that account: every document it files is that account's own, read by nobody else but the
administrator until the account shares it, and the documents it lists, searches and reads are only those the account reads. With no
such variable it files documents with no owner and reaches every document.

## Filing

- `artifact_file` takes `body`, the document: `kind`, `title`, `actorName` (who files it, any name the
  caller uses for itself), `labels` (an object of strings, `{}` for none), an optional `scope` (the
  caller's own grouping string, such as a project), and either `body` or `html`.
- `record` and `learning` take a markdown `body`; `page` takes `html` and no body, and its `title` may be
  empty where the page has a `title` element. `import` is written by imports, not filed by hand.
- The label `category` is the document's category in the archive's list: one word naming the work that
  made it, such as `security-audit`, `security-patch` or `completion` for a report, or the research shape
  that wrote it. Set it whenever a document is filed; the list shows it as a badge and filters by it.
- The label `pending` names what a document still lacks, comma-separated words; the archive shows them on
  that document as missing. Set it only for a part you will still give, and rewrite the labels without it
  once that part is given or once you will not give it.
- Every web link the markdown makes becomes one of the document's references.
- `artifact_rewrite` rewrites a document under its number: `documentId`, `labels` (required, they replace
  the labels whole) and any of `title`, `body`, `html`. Its attachments and share links stay.

## Handing a document to a person

A document written for somebody is handed over as a link, never as a file attached to a message:
`artifact_publish` files it and opens its link in one call.

- Its input is `title`, `description` (a line or two on what it holds), and its words as markdown `body`
  or as the `path` of a file on the machine the harness runs on, read whole; `kind` is `record` where none
  is named, `learning` or `page` beside it, and `scope` and `actorName` are as `artifact_file` takes them.
- Its answer is the document and its link: `document.id` is the document's number, and `share.url` is the
  address to hand over, absolute already. Give that address as it came back, adding nothing to it.
- The link has no end and opens with no console login. It names the document rather than a body, so a
  rewrite keeps it and it serves the newest body; it keeps opening across the service's restarts and
  redeployments. Where it does end, its token answers what it opened rather than nothing at all.

## Attaching files

`artifact_attach` keeps a file beside a document: `documentId`, `name`, `mediaType`, `content` in base64
and an optional `role`. A body draws a kept picture by naming it: `![a caption](name.png)`, so file the
body first, then attach the pictures it names; from a shell, `bmo archive doc --file <path>`, named once for
each file, files the body and attaches each file under its own name in one call.

A document's pictures stand in its body, at the place its words speak of them, each drawn by the name it is
attached under. An attachment the body does not draw is for a file a reader saves or plays, not for a
picture the reader is meant to look at: the files after the body show a picture only as a small thumbnail.
`bmo archive doc` and `artifact_attach` both refuse a picture a markdown document's body does not draw, so
the body names its pictures before they are attached.

`role: "head"` marks a file a reader is shown at the document's head, a narration's sound among them: the
reader plays it in its own listen band, above a page as under a markdown document's title, so a page carries
no player of its own. A narration in several languages names each sound with its language last, as
`narration-ko.ogg` and `narration-en.ogg`; one marked `head` brings the others into the band beside it, each
labelled with its language. A name the document already keeps is refused, so a changed file is attached
under a new name; one file is at most 64 MiB.

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
one by its `token`, and its token then answers what it opened, as 410 `gone`, rather than a body.

## Importing a web page

`artifact_import` takes `url`, `actorName` and optional `title` and `scope`, and answers the import at
once, `fetching`. The page is read in the background: its main content becomes a markdown document of kind
`import`, its pictures are kept as its files, and the page itself and the outside pages it links to are its
references. The import moves to `filed`, with `documentId`, or to `failed`, with `error`. The tools do not
read an import back; list the documents of its `scope` or search for its title to find what it filed.

## Refusals

A refused call comes back as a tool error whose text is the service's `status`, `code` and `error`:
`bad-request` for input not of the tool's shape (the sentence says what), `not-found` for a document,
attachment or link that does not stand, `gone` for a link that has ended, `conflict` for a body given to a
page or HTML to any other kind, `too-large` for a file over its ceiling, and `unauthorized` where the token is
wrong. A call that reached no service carries only `error`.
