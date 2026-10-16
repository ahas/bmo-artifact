# The document archive

This is the canonical design statement of bmo-artifact. The code is one realisation of it.
`docs/api.md` is the boundary this design is reached through, and `docs/layout.md` is where its parts
sit in the repository.

## What the archive is

- **One place for every document the agent keeps for reading.** In prose it is called "the document
  archive", because "the archive" alone means the database every record of the system lives in.
- **Several kinds of document:**
  - what a search found;
  - a page brought in whole;
  - study material the searcher wrote;
  - a record of data the owner asked to have put in order;
  - an HTML page kept as it stands and opened as that page.
- **A document can carry attachments.** Pictures, sound, video, text files and any other file are kept
  beside it, and shown or played where it is read.
- **Two writers, never more.**
  - The search runner writes what the searcher produced.
  - The agent writes a document through `bmo archive doc` in two cases: when the owner asks for
    something to be organised and recorded, and whenever something is better shown than told.
  - Nothing else writes a document, and no agent is handed a recording tool.
- **A search is one run.** One question from a room starts one run. When the run has written its
  answer, the search ends. The run's record rolls away with the agent log. What the run found out is
  therefore kept one level above it, as a document here.

## What this repository leans on

Every other unit is reached through the calls `docs/api.md` lists:

- **Page reading through the tool host.** bmo-explorer provides four things: reading a page whole,
  fetching a raster picture, checking an address, and pacing requests per host and engine.
- **bmo-agency** drives the research session and the co-worker pool. It also provides the tiers,
  providers and accounts.
- **bmo-job** queues a search run and holds its live state.
- **bmo-voice** speaks a narration.
- **bmo-receiver** says who sent the asking message and when.
- **bmo-web** hosts the console, which loads this repository's console plugin bundle. It also holds
  the language pack that supplies the screen's wording.

## Kinds

Every document has a kind. The kind decides how the document is outlined:

- which of its headings stand in the table of contents, and how deep that table goes;
- whether it opens with a summary;
- which properties its head shows.

Every document has the contents that fit it, and no kind borrows another's.

- **Each kind is its own module.** Each answers one common shape: its name, its label's wording key,
  whether it carries a summary, how its body becomes table-of-contents entries, and which properties
  its head shows.
  - One catalogue lists the modules.
  - The console screen and the share page ask the document's kind for its outline, and never branch
    on a kind's name.
  - Adding a kind is adding its module and its line in the catalogue.
- **`search`**: what a search found, in the shape set below.
  - It opens with the key answer and the short summary.
  - Its contents go down to the sections inside each finding: each section, each part label, each
    candidate's name.
  - Its head shows the question, the topic, the keywords, the applied rule, the options and the depth
    it ran at.
- **`import`**: a page brought in whole (see "A page imported whole").
  - It has no summary written by the agent. The page's own summary stands where the page has one.
  - Its contents are the page's own headings, their levels kept.
  - Its head shows the address the page came from.
- **`learning`**: study material.
  - It has no summary.
  - Its contents are a book's three levels: a top heading is a part, the heading under it a chapter,
    and the one under that a section. A document with no parts starts at its chapters.
  - Its head shows the topic and the keywords.
  - The agent writes one from material the owner hands it. A search writes one when it is asked to
    teach a subject (see "Study material from a search").
  - Which chapters a subject is given is set by the searcher's rulebook, not by this document.
- **`record`**: data the owner asked the agent to put in order and keep.
  - It has no summary unless the body writes one.
  - Its contents are the body's headings, to two levels.
  - Its head shows the request it was written for.
- **`page`**: an HTML page kept as it stands, whether the owner handed it over or the agent wrote it.
  - The file is the document. It is opened as that page wherever the document is opened (see "An
    HTML page").
  - It has no summary and no contents, since the page is read as its author drew it.
  - Its head shows the description taken from the page.
- **A kind may be a page of its own.** Its module says so. A document of that kind is opened as the
  page it keeps, rather than read as a body. The console and the share page ask the kind, never its
  name.
- **An unknown kind reads as `search`**, so a row the catalogue does not know is still drawn.
- **Heading levels are counted by rank**, not by the number of heading marks. The shallowest heading
  depth in a document is its first level.
  - The headings counted are the ones the text is drawn with, structure lines included. An import
    writes a page's top headings as `SECTION:` lines.
  - Structure lines stand above every markdown heading. A markdown heading keeps the depth its author
    wrote.
- **The state, the room and the times** stand on every document's head, whatever its kind.

## Where it lives

A document is made of rows in this repository's database and one reading file under the file root.

- **Why both.** They do different jobs:
  - The rows are the identity: they are where a screen builds its list, and they are what the owner
    can delete.
  - The file is what a person reads.
- **The database** is PostgreSQL. Its address is a setting. The six tables below are this repository's
  own, and no other unit reads or writes them.

### `searches`

One row per document, of every kind. The table keeps its name because the ids, the paths and the
share links of every document already written hang on it. A row holds:

- **The run:**
  - which room asked, and what the question or request was;
  - the depth it ran at;
  - its current state, which is the run's own ending word as it stands;
  - when it started and ended;
  - where its document file is.
- **Its kind and its writer:** `searcher` for what the search runner recorded, and the agent's writer
  name for what the agent wrote. Settings hold that name; the committed one is `bmo`. A search opens
  with the status `open` and the writer `searcher`; it is of the kind its launch names (`import`,
  `learning`), and of kind `search` where the launch names none.
- **Who asked:** the chat id of the person whose message the document answers, where the launch named
  that message.
- **The search options the launch gave:** period, region, excluded sources and result shape, as one
  JSON object. It is empty where none was given. A page imported whole keeps its address here as
  `source`.
- **The document's head**, kept apart from its body: the headline title, the topic, the keywords (a
  JSON list) and the rule the search applied.
  - Each is written when the result is recorded, and is empty for a document that wrote none.
  - A document the agent writes carries its title here.
- **Whether it left a document at all.** A search that answered one value keeps none, and this mark
  holds it out of the list. A row that says nothing reads as one that did leave a document.
- **The row is closed before the document is written.** The document records how and when the search
  ended. A document drawn before the close would say forever that the search is still running.

Some rows need extra rules:

- **A document with no question** is one the agent writes. It fills the row's required places with its
  own values:
  - the room that asked;
  - the request as it was put;
  - its kind, in place of a depth;
  - the state `done`;
  - the moment it was written, as both its start and its end.
- **A row that names no kind** is read by what it is:
  - A row whose question opens with the fixed prefix the import request was written with is an
    `import`. That prefix is data the rows carry, matched as it stands, never a sentence of the
    wording file. Settings name it; where they name none, no such row is an import.
  - Every other such row is a `search`.
  - All of them are the searcher's.
  - The rule is written into the row each time a process opens the database, before its first
    statement on the document tables. A reader of an older copy applies the same rule itself.

### The other five tables

- **`search_findings`**: one row per finding. It holds:
  - which document it belongs to;
  - which run produced it;
  - which branch it came from;
  - its position in the document;
  - the content itself.

  A document the agent writes keeps its body as one finding. Every kind is then read and drawn by the
  same path.
- **`search_sources`**: one row per source. It holds which finding it belongs to, which page, that
  page's title, what the page supports, and the footnote mark the text cites it by.
- **`archive_attachments`**: one row per attached file. It holds:
  - which document it belongs to;
  - what it is: `image`, `audio`, `video`, `text` or `file`;
  - the address it came from, where it came from one;
  - the file it was kept as, its type and its size;
  - the name it is shown by, and its place in the document;
  - when it was kept.
- **`search_images`**: the pictures searches kept, one row each, as they were first kept.
  - Current code never writes it. It stays standing for readers that still read it.
  - Each time a process opens the database, every row of it not yet in `archive_attachments` is
    copied there as an `image`, named by its file, with no place. Nothing is copied twice.
  - A reader of a copy that has no `archive_attachments` reads its pictures from here.
  - A picture is looked up by the address it was found at. Where two rows carry the same address, the
    one kept last is the copy.
- **`web_search_shares`**: one row per share link. It holds the token, the document, who made the
  link, when it was made and when it stops opening. A link with no end has no expiry.

### The file root

- **The document file** is in `searches/<document id>/` under the file root, which is a setting.
  - It is a reading text. Its header holds the title, the kind and the facts of the document, and
    the body stands under it.
  - A search's file also puts the topic, keywords and rule in the header. It folds the research trail
    (what each branch asked and opened) at the end.
- **The files beside it:**
  - the pictures a search kept, in `searches/<document id>/images/`;
  - every other attachment, in `searches/<document id>/attachments/`.

  The rows name each file by its whole path, so a document's files never move.
- **A document outlives its room.** Removing a room removes none of its documents. Before
  bmo-receiver removes a room, it asks this repository what it holds for that room, and the answer
  says the documents stay.
- **Why sources are rows of their own**, instead of being packed into one field of a finding: tracing
  back. It must be possible to ask which page a claim came from, and also which claims one page
  supports.

## The shape of a search's result

The search run writes its result as plain text in structure lines, which the runner reads. A person
reads it as a document, so the lines set a fixed order.

### The head

The runner lifts these lines out of the text and keeps them in the search's own row. They are shown
and searched apart from the body, and never repeated inside it.

- **`TITLE:`**: a headline, short like a news title. It never restates the question.
- **`TOPIC:`**: what was searched, in a few words.
- **`KEYWORDS:`**: the search keywords, comma-separated.
- **`RULE:`**: the name of the filtering rule applied. Only the restaurant-search rulebook sets one,
  so only a restaurant or delivery recommendation carries this line.
- **`KIND:`**: the kind of document the result is. Only study material writes it, and only as
  `learning`.
  - It is read against the catalogue. Only a kind whose module says a search may write it is taken,
    and it replaces the kind the row was opened as.
  - A result without the line, or with a word no such kind answers to, leaves the row's kind as it
    was.
- **`DOCUMENT:`**: written only as `no`, and only by a search that leaves no document at all (see "A
  search that leaves no document"). It is not kept on the row; it is read once, as the record is
  written.

### The narration

Only a deep dive with a body writes a narration: a line holding only `LISTEN:` under the head, with
the script running from it to `VERDICT:`.

- The runner lifts it out of the result before anything is recorded or handed back. It is therefore
  not a finding, the room's session never reads it, and the console never shows its letters.
- Once the room has its answer, the runner asks bmo-voice to start the briefing.
  - It is spoken in the agent's voice.
  - It is kept as the document's sound, named by the document's title.
  - It stands under the masthead, as every sound a document keeps does (see "How a document is
    drawn").

### The body

The body has three fixed places and a middle that the question shapes.

1. **`VERDICT:`** is the key answer: the answer itself, the pick, the recommendation or the
   conclusion.
   - It is a numbered line per pick, or one small table. It gives no grounds beyond the one line a
     kind's rulebook asks of a pick.
   - It stands first, above the summary: a summary sums up a document, and the answer is read before
     the document.
   - Every result with a body has it. A brief result has none, because its summary line is the value
     itself. Study material has none either.
2. **`SUMMARY:`** is the short summary: a numbered list, one line per item.
   - It stands under the verdict, and every result has it.
   - It says what the document found around the answer, without writing the verdict again.
3. **The middle** holds the findings, in sections the document's nature arranges.
   - Each section opens with `SECTION:` and what it holds.
   - A comparison is a table. Its rows are the things compared and its columns are what they are
     compared on. Each cell carries its as-of date, and an unchecked cell holds the unchecked word.
   - A set of comparable things follows as one entry per thing, between `===ITEM===` lines.
   - Figures that are compared or that move are drawn as a Mermaid chart, beside the table that keeps
     their exact values. Examples are values across the candidates, a price or rate over time, and
     the shares of a whole.
4. **`UNCONFIRMED:`** is a numbered list of what could not be checked. It is gathered at the end
   rather than mixed into the sentences. Every result has it, and it stands last.

### The summary

- **It takes one of three forms**, whichever the search's answer fits:
  - background, fact, verdict;
  - the answer, then the two conditions that narrow it;
  - three important facts standing side by side. This form is rare and never the default.
- **Three lines is the aim, not a quota.**
  - A fourth line is written where one more axis of its own stands beside the three.
  - Two lines, or one, are written where there is nothing more to say.
  - A line that restates another, or exists to make up the count, is never written.
- **A comparison's summary sets its candidates on different criteria.**
  - Each line names the candidate that comes out best on one criterion, and the criteria differ from
    line to line.
  - The criteria are the search's to choose; that they differ is the rule.

### The middle

- **It has no fixed parts.**
  - There is no introduction and no conclusion: the topic already says what was asked, and the
    verdict already states the answer.
  - The document's nature decides the order of its sections. Background comes first where the facts
    cannot be read without it. The most important comes first where the facts are themselves the
    answer.
  - Each section is named for what stands in it.
  - A result whose middle is a single table or a single set writes no section line at all.
- **A product search** keeps the same frame:
  - the verdict, with the pick in it;
  - the summary;
  - the comparison table directly under it;
  - a section holding the product line's recent news, apart from the prices;
  - one entry per candidate;
  - what could not be confirmed.
- **An older result's part labels are still read.** A line holding only `INTRO:`, `BODY:` or
  `CONCLUSION:` is drawn as the introduction, body and conclusion headings of the wording.
- **A brief result** answers a question after one value or one fact. It carries only the head, the
  summary (the value and its as-of time) and what could not be confirmed. Its size follows the
  question, never a ceiling in code.

### Entries of a set

- **An entry** opens with `NAME:`. It may carry:
  - `ADDRESS:`;
  - `MAP:`, a map link;
  - `ROADVIEW:`, a street-view link;
  - `IMAGE:` lines and `REVIEW:` lines;
  - a `MENU:` block;
  - a `MORE:` line, after which the long explanation is folded on screen.
- **A place's map is drawn in the document.** An entry's `MAP:` line stands as an embedded map of the
  place, with the link under it. The map is found by the entry's `ADDRESS:`, or by the place the link
  searches for where the entry has no address.
- **A menu is a block of its own.**
  - `MENU:` opens it, followed by where the menu was read, where that is worth saying.
  - Every line up to the next structure line is one item: a dish and its price, separated by `::`,
    with a note after a further `::` where one is needed.
  - A line with no price is a group heading of the menu.
  - On screen the whole block is folded under a menu heading, since a full menu is looked up rather
    than read through.
- **A product's entry carries four more lines**, which are what a reader compares products on. A
  value the search could not check is written as the unchecked word. An unchecked value is thereby
  told apart from one that does not exist.
  - `RELEASED:` gives the release date, as its source writes it.
  - `NEWER:` names the successor or the newest model, or the none word where the search looked and
    found none.
  - `STOCK:` gives the selling state in the product page's own words, when it was read, and the page
    it was read on. The three parts stand together, since a state with no time on it was true only
    once.
  - `DEAL:` gives the price now, the usual price, and whether the discount is real. It is written only
    where the page is running an offer.

### Pictures, reviews and footnotes

- **An `IMAGE:` line** carries an address and a caption.
  - It stands wherever what it shows is written about. Where the result is a set of things, that is
    inside the entry it belongs to. Where the answer itself is something to look at, such as a design,
    a screen or a chart, that is the body.
  - A question after one value carries none.
- **A picture of something chosen between** (a place, a business, a product) carries three more
  fields:
  - the entry's name;
  - the shot, meaning the composition or kind of shot;
  - the source: `official` for the business's or maker's own sample, `customer` for a customer's
    photo out of a review or a blog post.
- **How pictures are grouped:**
  - Such an entry's pictures stand grouped by shot, several to a shot, so a reader compares like with
    like.
  - On screen and in the document, the caption is followed by the shot and the source. The entry is
    the heading the picture already stands under.
  - Two or more `IMAGE:` lines written one after another form one group, and stand as one slide
    gallery. Blank lines between them are allowed.
  - A picture standing alone stands on its own line.
- **A `REVIEW:` line** carries what one review said, in its own words, and never a summary of several.
  - It holds the rating as the source writes it (or the none word), the review's words, and the page.
  - The pictures the review carried join its entry's pictures, with captions saying they came from a
    review. The entry's pictures stay one gallery.
- **Footnotes.** A claim cites its page by a mark in the text, `[n]`, and the block closes with a
  `SOURCE [n]:` line giving the address and what it supports.
  - A value read only from a search-result snippet, with the page never opened, is marked `[n†]`.
    Its source line carries the same mark, so which numbers were checked on the page is read from the
    mark.
  - A plain `SOURCE:` line with no mark is read as a source with no footnote.

### Rules for the whole result

- **No repetition.** A number stands in the section it belongs to and in the summary, not in three
  places.
- **The structure words never show.** The console turns every structure line into a heading, a
  label, a link, a picture or a fold. None stands as its own letters.
- **A search with no head** keeps its `TITLE:`, `SUMMARY:` and `NAME:` lines in the finding text.
  They read as a heading, a labelled summary and a name, and such a search is called by its question.
- **The words a search writes** (the unchecked word, the none word, a section's name) are its
  rulebook's. This repository reads them as text and never matches on them.

## Study material from a search

A search asked to teach a subject writes study material: a `learning` document, whose shape is its
own and borrows nothing of the frame above.

- **How it is asked for:**
  - The launch gives the result shape `learning`, and the row is opened as `learning` from the start.
  - Or the search finds, in the question itself, a request to be taught a subject, and says so with
    `KIND: learning`.
- **No summary.** It writes no `VERDICT:` and no `SUMMARY:`. The contents are what a reader takes in
  first.
- **Its levels are the result's own markdown headings**: a part, a chapter and a section, or a chapter
  and a section where the subject has no parts.
  - It writes none of the search's structure headings: no `SECTION:`, no entries, and no
    `UNCONFIRMED:` line. A structure heading would rank above every part and push the sections out
    of the contents.
  - What could not be confirmed is its last top-level heading.
- **What stays.** Footnotes, `SOURCE` lines, pictures and fenced code are read as in every result. A
  result with headings has a body, so it always leaves its document.
- **Where it is drawn.**
  - The console and the share page outline it as the `learning` kind does: part, chapter and section,
    to three levels.
  - The document file sets its headings under its own result heading. The shallowest is at the third
    level, and the ones under it keep their distance.

## A search that leaves no document

A question after one value or one fact (a price, a rate, a date, a version) is answered in the room
and leaves nothing anybody would open later.

- **The run says so** with `DOCUMENT: no` in its head. It writes that only on a result made of the
  head, the summary and what could not be confirmed.
- **The word is checked before it is taken.** A result carrying a body of any kind keeps its document,
  whatever its head says. A body here means a section line or an older part label, an entry, a
  picture, a review, or a heading of its own. A run that misjudged its own answer costs an unwanted
  document rather than the record.
- **What is kept:** the search's own row: the question, who asked, the depth, the state, the times,
  and the head the result gave.
- **What is not kept:** the findings, the pages and the file.
- **Where it shows:** nowhere in the list, which is a list of documents. The row stays readable one by
  one at its own address, where the screen says the search answered a value and kept no document.
- **A share link is refused** on it, because a link hands out a document and there is none.
- **Everything else is unchanged.**
  - The answer reaches the room as every answer does.
  - The run is recorded as every run is.
  - A later, deeper run of the same question writes its document the ordinary way and clears the mark.

## A page imported whole

Somebody hands over one address and asks for the page itself, not a question about it. The page is
already a finished document, so nothing is researched, gathered or summarised: the page is read as it
stands and becomes the search's document.

- **The launch** is `bmo search --import <address>`, with the chat and the message as for any search.
  - The address is kept on the row as the `source` option.
  - The question is the request as it was put. Where none was given, it is the wording file's
    import-question line holding the address.
  - It always runs at the first depth, since no model runs to go deeper.
- **No model runs.** The runner asks the tool host to have bmo-explorer read the page.
  - bmo-explorer opens the page in its browser, behind its address check, at a desktop width and
    twice the density.
  - It follows every redirect to the end, and reads the page only once its address has stopped
    moving.
  - It reads a page a script draws after it has drawn, scrolling through it once first.
  - The whole reading is bounded at two minutes. A page that has not drawn by then fails the search,
    and the room is told so.
  - The runner turns the reading it receives into the result, and keeps the figures that were
    captured.
- **The reading is the page's own**, taken in document order out of the page's main text:
  - Its one top heading is the head's `TITLE:`. The page's declared keywords, where it has them, are
    `KEYWORDS:`. `TOPIC:` names the host it came from.
  - Its top headings are `SECTION:` lines, and the headings under them are markdown headings, with
    their levels kept.
  - A top heading that is the page's own summary is `SUMMARY:`, holding the page's own words.
  - Its own table of contents is a section under that table's own title.
  - Bold, italic and monospace text are read off the computed style, and kept as markdown.
  - Tables, code blocks with their language, nested and numbered lists, and links to other pages are
    kept.
  - A framed box is a quotation, with its bold title on the first line.
  - What the page hides is not read. A paragraph that opens like a structure line has its colon
    escaped, so it is shown as written.
- **Nothing of the agent's own is added:** no summary of its making, no comment and no reordering.
  The result carries only:
  - what the structure needs: the verdict line, the head, the page's body, and `UNCONFIRMED:` only
    where a figure could not be captured;
  - the sources:
    - the page it was read from;
    - the address asked for, where the page moved on from there;
    - the outside links of the page's own reference section.
- **Every picture, where it stood:**
  - An image is an `IMAGE:` line with its own address and its caption or alt text. It is kept the way
    any picture of a search is.
  - A figure the page draws itself (an inline SVG, a canvas) is captured out of the drawn page, and
    named after the page address with a figure number. An SVG's own source is kept beside the capture.
    Its caption is the figure's own caption, label or title, or a numbered figure caption where it
    has none.
  - A picture never stands inside a quotation. A box holding a figure is closed before it and opened
    again after.
- **What the room is told:** that the page is in, its title, and the address of its document. It is
  never told the page's text, a summary of it or its pictures. The document is the answer.

## What the search is given

The search run's line is composed outside this repository, by bmo-agency. The line carries the
searcher's rulebooks and the tools it may call, and bmo-explorer provides the web tools. This
repository decides only these things:

- **The research profile:** the depth's effort, its branch count, and the provider it runs on, as
  the research-settings routes keep them.
- **The question, with the options the launch gave:** period, region, excluded sources and result
  shape. An option not given is left to the run's own judgement from the question.
- **Who asked, and when.** The launch names the message that asked. The launcher asks bmo-receiver
  who sent it and when, and the run is told. Nobody writes this into the query by hand. The moment
  is told in the time zone the settings name, with the zone's abbreviation and the weekday.
- **The result shape it reads back:** everything this document sets out.

## Attachments

A document keeps its files beside it, and each one is a row of `archive_attachments`.

- **What an attachment is:** a picture, a sound, a video, a text file, an HTML page or any other file,
  kept as a copy under the document's own folder. Its kind is read from its bytes and its name, not
  taken on the writer's word.
  - The bytes decide first. The name decides only between the forms one container holds: an Ogg or a
    WebM is a sound or a video by what it was called.
  - A text is what decodes as UTF-8, with no NUL in it. Anything else is a file that is only handed
    over.
  - A text is recorded as a page (`text/html`) in either of two cases: its name ends in `.html` or
    `.htm`, or its first tag after any comment is a doctype or an `html` tag.
- **How one is handed over.** Every writer keeps a file the same way, giving three things: a file on
  this machine, the name it is shown by (where the file's own name is not it), and the piece of the
  document it belongs to.
  - `bmo archive doc` and the search runner keep files through this repository's own keeper, which
    takes files on this machine only.
  - The runner has the pictures its result names fetched first, through the tool host's checked
    fetch.
  - A picture is written as AVIF, upright. A GIF or a WebP is kept as the bytes that came, because it
    may be animated.
  - A file is copied, never moved. The original stays where it was, and if it later goes missing, the
    document loses nothing.
  - An attachment is kept under `searches/<id>/attachments/`, named by a digest of where it came from
    and its own name.
  - A picture fetched from an address is kept under `searches/<id>/images/`, named by the address's
    digest.
  - A refused file is refused with the reason, and nothing is written for it. A file is refused when
    it is missing or unreadable, is not a file, failed to fetch, or is over the ceiling.
- **Where it stands in the document:**
  - A picture a line of the body names is drawn where that line stands.
  - Every attachment the body does not draw stands in the attachment section after the body, in the
    order it was attached.
  - A body names an attached file by the name it is kept under, in place of an address:
    - a markdown picture with that name as its target is drawn from the kept file;
    - a markdown link with that name goes to the file, and for a page, opens the page.
  - The name is resolved under whichever route the document is read on, so the same body works on the
    console and on a share page.
  - A picture the body draws this way is not listed again in the attachment section. A file the body
    only links to still stands there.
- **How each is shown:**
  - a picture inline;
  - a sound or a video in a player that plays where it stands;
  - a text file as a preview;
  - a page as a button that opens it in a tab of its own;
  - and each of them as a file to download.

  A picture is drawn on white, whatever the theme. Diagrams drawn for paper, with black lines on a
  transparent ground, then stay readable on a dark screen.
- **The console and a share page show the same attachments**, each through its own address. Neither
  reaches a file outside that document's folder.
- **Bounds:** a ceiling of 1 GiB on one file. A picture is converted as described; every other kind is
  kept as the bytes that arrived.

### The pictures a search keeps

A picture the result names lives on somebody else's server, and that address is the first thing to
rot. So the search keeps its own copy.

- **Only a picture the run looked at stands.** Before anything is kept, recorded or handed on, an
  `IMAGE:` line is taken out of the result if it names a web address the run never opened with its
  picture viewer. This applies to the document and to the room. The run's log says how many went.
  - A pooled branch's report is held to the same rule, against the branch's own record.
  - A record showing no tool call at all cannot be judged, and leaves the result as it was written.
  - A page imported whole is read, not researched, so its pictures are not held to this rule.
- **What is kept:** every address an `IMAGE:` line of the result names, and nothing else. A page
  imported whole names every picture it has. The figures it draws itself are captured rather than
  fetched.
- **When:** as the run ends, by the runner that writes the document. The run itself is shown no file
  tool.
- **In what form:** AVIF.
  - A picture that will not convert is kept as the bytes that arrived, under the type they arrived as.
  - Transparency is kept as it came. Whatever shows or sends a kept picture lays it on white itself.
- **Bounds:**
  - The fetch goes through the tool host's checked fetch, so an address that resolves inside this
    network is refused before a connection is made.
  - Bytes that are not a picture are not kept.
  - There is a ceiling on how many pictures one search keeps, and a ceiling on how long the keeping
    may take. A slow host costs a copy, never the answer.
  - A picture that was not kept goes on naming its address.
- **Where no copy exists:** a picture whose copy is gone from the disk, or that was never kept, reads
  as a picture with no copy. The address answers for it.
- **What the address is still for:** the finding keeps the line as the run wrote it, so the document
  says where the picture came from. The kept copy is what is shown and sent.

#### What each reader gets

- **The room.** The result handed to the room's session carries the kept file's path in place of the
  address, for every picture that was kept.
  - Beside the result, the session is handed the albums to send. They are worked out of the `IMAGE:`
    lines in the order they stand:
    - one album to each entry and shot, captioned with both;
    - every picture labelled with its entry, shot and source;
    - pictures with no shot together in one album, each labelled with its entry's name, or with its
      caption where it belongs to no entry.
  - Each album is one photo call, in that order.
- **The console.** A picture is served from the kept copy where there is one. It is fetched from its
  address, through the page reader, only where there is not.
- **The document.** Its picture points at the kept copy.

## An HTML page

The archive hosts HTML pages the way a static host does. A page document is its HTML file: what goes
out is that file with the archive's design and the reader's scheme put in front of it, and nothing is
written in its place.

- **Two ways in.**
  - A file the owner hands over is kept as it stands.
  - A page the agent writes is kept the same way. Whatever is better seen than told (a comparison, a
    layout, a table too wide for a message, a picture of an idea), the agent writes as a page, files
    and hands over as a link.
- **Filing it:** `bmo archive doc --html <file>` files the page as a document of kind `page` (see "A
  document the agent writes").
  - The file is copied in untouched.
  - A page carries what it draws inside itself or over https, so it takes no body and no other
    attachment.
- **Its title and description** are read out of the page, and never written into it.
  - The title is `--title` where one is given, and the page's own title otherwise. A page with neither
    is refused.
  - The description is the page's description meta. Failing that, it is the start of its words, about
    two hundred characters.
  - The description stands under the title in the list and in the document's head.
- **Reading it:** `bmo archive html-read <file>` gives the page's title and its words in order.
  - It reads a block to a line, with headings as markdown headings and list items as dashes. Scripts
    and styles are not words.
  - Every picture written inside the page as a data address stands in the reading as a markdown
    picture named `picture-01` and onward. `--stem` names them otherwise, where one document takes
    the pictures of two pages.
  - With `--pictures <dir>`, those pictures are written into that folder under the same names.
  - The words are cut at the answer ceiling, and `--from` carries on at the line the cut names.
  - The reading is structural and does not run the page. Nothing is filed out of it.
- **Opening it:**
  - The document's own address opens the page.
  - On the console, the document's screen holds the page in a frame filling the screen, under a bar
    with its title and its actions. The actions are: open in a tab of its own, save the file, the
    share link, and the properties. The page's own scroll is the only one there.
  - Under a share link, the page fills the whole window in the same frame, with nothing around it.
- **Its design is the archive's where it brings none.** The archive's page sheet and the reader's
  scheme go ahead of the page's own lines. What they do depends on what the page brings:
  - **No stylesheet of its own:** the page is drawn wholly in the archive's design: the reading
    column, the masthead, the type, the tables, the boxes, and the palette of the reader's scheme.
  - **A style element marked `data-archive`:** this is an addition to the design, not a sheet of its
    own. The design still draws the page, and the addition draws in the design's colour variables, so
    it follows the scheme as well.
  - **Its own stylesheet, with no dark scheme declared:**
    - The page keeps its layout and every part it names by a class or an id.
    - It takes the archive's type, in the light palette, over what it wrote for bare elements.
    - Where the reader's scheme is dark, it is drawn inverted with its hues turned back, and its
      pictures and films inverted again.
  - **A dark scheme of its own:** the page draws itself. Its own questions about the scheme are
    answered with the reader's scheme.
  - The sheet stands before anything the page draws, so the first paint is already in the scheme.
- **The parts a page builds with.** A page written on the archive's design is plain HTML in a body the
  sheet lays out. It draws headings, paragraphs, lists, tables, quotations, code, figures and folds as
  a markdown document's. It also offers these named parts:
  - a `header` opening the body as the masthead, with an `eyebrow` line over its title and a paragraph
    under it saying what the page is;
  - a `lede` box for the key answer;
  - an `aside` for a note, toned `ok` or `warn`;
  - a `chip` for a short state, toned `ok`, `warn` or `bad`;
  - a `listen` band holding an audio player and a line on what it says;
  - a `grid` of boxes side by side, which fold to one column on a phone;
  - a table, which scrolls sideways inside itself where it is wider than the column;
  - a `footer`, which closes the page in the muted ink.
- **Hosting it.** The page is served on the same route as every attachment, behind the console's
  sign-in or under a share link.
  - What goes out is the file as kept. Only a few lines go ahead of it: one lets its copy buttons work
    in the sandbox, and one hands a pressed picture to its frame (see "Pictures open whole"). A
    download is the file itself.
  - **Links:** in a frame, a link the page opens in its own window opens in a tab of its own instead.
    A link within the page still moves the page, and a press the page takes for itself is left to it.
  - **Share preview:** under a share link, the share page's own preview head is sent before the frame
    is drawn. Each preview tag the page writes itself stands there over the link's own.
  - **Scripts:** the page runs its own scripts.
  - **Sandbox:** it runs sandboxed, with an opaque origin of its own. No cookie, storage or request of
    the console's is its, and it reaches nothing of the console it was opened from.
    - It opens no connection and submits no form.
    - It loads what it is drawn with from inside itself or over https.
    - The one thing it can ask of its frame is to open pictures in the viewer.
  - **Framing:** it is framed only by the console, and only as the whole of a document's screen or of
    a share link's window. No other site may frame it.
  - Every other attachment keeps the policy that lets nothing on it run.

## Diagrams

Structure is drawn rather than told. Whatever a document explains about how parts connect, the order
work flows in, the stages something passes through, or how one thing depends on another, it draws as
a diagram. A line of prose beside it says what the picture cannot.

- **In a markdown body,** a diagram is a fenced `mermaid` block, which the console screen and the
  share page draw as its picture. A search, the agent and a job write it the same way.
- **Laid out to read on a phone:**
  - A flow runs top to bottom, unless it is a short chain.
  - A box carries a few words rather than a sentence.
  - A diagram shrinks to the column by at most a fifth. One wider still scrolls sideways in its own
    box.
- **The default layout and look stand.** A diagram that needs the older placement or look names it in
  its own front matter.
- **Colour is the console's.** A diagram names no colour of its own, so it is right in both themes.
  - A pie or a chart keeps to eight series and folds the rest into one "other".
  - Boxes of a flow that differ in kind are marked with the classes `c1` to `c8`: one class to a kind,
    and the same class for that kind throughout.
- **In an HTML page,** a diagram is inline SVG written into the page. It is drawn in the page's own
  colours and scales by its view box.
- **No other diagram language is drawn.**

## Findings and sources

A finding is one claim and the pages it stands on. The pages are recorded in two kinds:

- **Cited sources:** the pages the result itself put under the claim.
  - Every chunk carrying a claim closes with one line per page, saying what that page supports.
  - A claim with no page to support it gets no arbitrary page. It is written as unconfirmed.
  - These lines are for the document, not for the room: the session receiving the result does not
    read them out.
- **Opened pages:** every page the run actually opened, including those it chose not to cite.

How the two relate:

- **They are kept apart** because they answer different questions: "what backs this claim" and "what
  did this search look at".
- **No double entry.**
  - A cited page stands under its claim, and is not written again among the opened ones.
  - A page two branches opened stands only once.
- **The verdict line is not a finding.** The result's first line, `ANSWERED:`, is the run's judgement
  of its own answer. The runner uses it to choose the run's ending value: `done` or `unanswered`.

## Who writes it down

- **Not the search run.** Its line has the web and co-workers, but no file-writing tool and no shell.
- **The runner that supervises the run writes it.** The runner holds this repository's database, and
  reads the run's record through the research session.
- **The runner reads two materials:**
  1. the result text the run returned, with its findings and cited sources;
  2. the run's record: what was asked of the web, which pages were opened, and which branch each call
     came from.
- **Which branch a claim belongs to** is decided by the pages it cites.
  - A claim standing only on pages one branch opened belongs to that branch.
  - A claim on pages spanning two branches, or on a page nobody opened, belongs to the result.
  - Nothing is attached by guesswork.

## Branches and merging

- **Branches.** Thorough and exhaustive searches split into branches. A branch is one branch call
  inside the run, and every line under that call belongs to it.
- **Pooled branches.** A branch given a place from the co-worker pool is a run of its own.
  - The parent's record keeps only the call and the report the branch returned.
  - Such a branch's findings stand on the sources its report cites.
- **Branch names.**
  - A branch's name carries a number, since agents of the same kind are often called several times.
  - The number means something only inside one run, so a branch is named by its number plus its run.
  - Where a search has only one run, the document leaves the run number out.
- **Grouping.** The document records which branch each finding came from. Its research trail groups
  what was asked and opened by branch.
- **Merging** happens inside the run: when the branches return, the run writes them as one result.
  - In the document, merging means keeping that result as the body, with the branches' trail folded
    under it.
  - It does not pretend the branches were separate runs.

## Branches are taken from the pool

- **The run does not decide the count.** How many branches open is not the run's call.
- **How a branch opens.** A search run gets no built-in subagent. It opens a branch only through the
  branch tool, which runs this repository's `bmo search branch` verb.
  - The verb keeps to the research profile's branch count.
  - It starts a branch only after taking a place in the co-worker pool, which bmo-agency holds.
- **The limits:**
  - **One level only.** A branch has no branch tool.
  - **Per-depth counts** are set by the research profile. A request beyond the count is refused until
    that run ends.
  - **Concurrent requests** cannot exceed the count, nor the pool's places.
  - **A refused run** writes its result from what it already has.
- **In the record,** a branch is one branch-tool call, and the text that call returned is its result.
- **No workflow tool** is shown to a search run.
- **Two sources share the verb.** `bmo search branch` accepts `--source search` and `--source browse`,
  so bmo-explorer's browse runs open their branches through the same verb.

## A stopped search still leaves a document

- **No ceiling.** A search has no time limit, so time never cuts a document short.
- **A person can stop it.** When the owner stops a running search, bmo-job marks the run stopped.
  The runner then salvages the last structured result the run wrote, and records it the usual way:
  - the findings and sources go in;
  - the search is closed as `stopped`;
  - the document is drawn.
- **Nothing written.** If the run wrote no result at all, there is nothing to keep, and no document is
  made.

## What does not move

- The three depths and what they mean. The research profile sets the effort and the branch count for
  each depth.
- The way results return to a room. The runner does not speak to the room: it hands the result to
  that room's session as a quiet instruction, and the session answers in its own voice.
- The run row bmo-job keeps for each run, and its ending values.

## The log is not the document

- The search run's record accumulates in the agent log, which bmo-agency keeps. It holds which tools
  were called with which arguments, and what came back.
- That log rolls: it keeps a bounded number of lines per agent, for a bounded time.
- So the log is the material the runner reads when the run ends, and the document is the record that
  remains.

## A document the agent writes

When the owner asks the agent to put some data in order and keep it, the agent writes the document
itself and files it with one verb.

- **The launch** is `bmo archive doc`. It is given:
  - the room and who asked. Each one not named is its default in settings. The default room is the
    owner's private room; where settings name none, a launch that names no room is refused;
  - the title;
  - the kind: `record` unless `learning` or `import` is named;
  - the body, as markdown from a file or from standard input;
  - any number of files to attach;
  - how long its share link lives:
    - a week, unless named;
    - `--share-days <n>` for a span of days;
    - `--share-forever` for a link that never ends.

    A span and no end are not both named, and a span must be some time at all.
- **A page** is filed with `--html <file>`, in place of the kind, the body and the attachments.
  - The kind is `page`, and the file is its one attachment.
  - The title may be left to the page.
  - The description taken from the page is the document's topic and its one finding.
  - No standard input is read for it.
- **What it writes:**
  - the row, with the agent's writer name as its writer and its title as the request;
  - the body, as the document's one finding;
  - the document file, headed by the title, the kind, the request, when it was written, and what is
    attached;
  - each attachment, copied into the document's folder.
- **A spoken briefing.** A markdown document that is also to be heard carries the recording as one
  attachment. A sound file attached to a document stands at its head as its listening band.
- **A narration.** `--narration <file>` gives a markdown document a script for the ear.
  - Once the document is written, the verb asks bmo-voice to start the briefing. It is spoken in the
    agent's voice, and kept as the document's sound, named by its title.
  - The answer says, on a line of its own, that the narration is being spoken.
  - bmo-voice may refuse the launch, for example because briefings are switched off. The document is
    then still written whole, but the answer says why it was not spoken and the verb ends as a
    failure, so whoever filed it knows the sound is missing.
- **Writing one again.** `--replace <id>` writes one of the agent's own documents again, under its
  number.
  - Its kind, title, request, body and attachments are the new ones.
  - The files it kept before are removed. That includes a narration, unless `--narration` gives it one
    again.
  - Its number, its room, when it was first written, and its share link stay. Every address already
    handed out opens the new version.
  - A document the agent did not write is not written again.
- **What it refuses:**
  - a document with no title, or an empty body;
  - a kind that is not one the agent writes, or `page` named as a kind instead of filed with `--html`;
  - a page file that is not HTML;
  - a page given a body, other attachments or a narration;
  - a document to write again that is not the agent's or does not exist;
  - a file that document already keeps, handed back to it;
  - a file it cannot read, or one over the attachment ceiling;
  - a narration it cannot read, or one that is empty.

  Nothing is written unless every part of it can be: all of it is checked before the row exists.
- **A share link in every room but the owner's private room.** That room is named in settings.
  - A room of other people cannot open the console address, which is behind the console's sign-in.
    A document filed for any other room therefore gets its share link as it is written, unasked.
  - A link that already stands is kept as it is, except for its end, which a link asked never to end
    takes off.
  - The owner's private room never gets a link; it is given the console address. The refusal says so
    in plain words that name no person.
  - Both addresses stand under the console's public address, which settings name. A launch is
    refused before anything is written where settings name none.
  - A link the verbs make is recorded as made by the agent's sender name, which settings hold.
- **What it answers:**
  - the document's id;
  - the address to give the room, on a line of its own;
  - the console address;
  - the share link and when it stops opening, or why there is none.
- **A link for a document already written.** `bmo archive share <id> --chat <chatId>` hands one
  document of any kind to a room the same way.
  - It gives the link that stands, or a new one living `--days` (a week, unless named).
  - `--forever` asks for a link that never ends: a new one with no expiry, or the standing one with
    its expiry taken off.
  - In the owner's private room it makes none, and gives the console address.
  - A number that names no document, or a search that kept none, is refused.
- **Study material a search writes** belongs to the searcher, through the search runner like every
  other search.

## A document a job shows the owner

A job that puts a document in front of the owner files it in this archive, and nowhere else. Such a
document might be a design to approve, a comparison to choose from, or an explanation of how
something works.

- **Filing it:** `bmo archive doc` with a title and a body, or with `--html` for a page.
  - With no `--chat` and no `--by`, the document goes to the default room and author in settings.
  - It is written under the agent's writer name, like every document the agent writes.
  - `--replace <id>` writes it again under its number, as the design moves.
- **A long reading takes a narration.** A document the owner reads as research carries `--narration
  <file>` as well.
- **Handing it over.** The job puts the console address the verb prints into its question or its
  report.
- **Never outside.** The job filing a document anywhere else is bmo-job's rule to refuse.

## How a document is drawn

One design draws every document, a markdown one and a page alike, on the console and under a share
link, in a light and a dark scheme. The design is one data file of this repository, which the server
reads to write a page's sheet and the console plugin bundle reads to draw a document.

- **The palette** is one set of document colours per scheme. It holds:
  - a warm paper, with a raised and a sunk surface;
  - the ink in three strengths;
  - a rule and a softer one;
  - an accent;
  - a calm tone and a caution tone.

  It is kept in that one file, which the document screen and the page sheet both read.
- **The reader's scheme** is the console's screen mode: light, dark, or whatever the system asks.
  - A reader who never chose one reads dark, and so does a share link's holder.
  - The mode is kept in a cookie, so the server knows it when it sends a page.
  - The console's frame names the scheme it stands in, so a page in it changes with the screen mode.
- **No white first.** Every document address paints its ground in the scheme from the first frame.
- **The masthead:**
  - the kind's label stands over the title, letter-spaced in the accent;
  - the title is large and bold, runs the whole reading width, and leaves no word alone on its last
    line;
  - the title breaks only between words, unless a word has no break in it (an address, an
    identifier);
  - the row of the state and the actions follows it on the console;
  - a rule in the ink closes it.
- **The body:**
  - the words are 16px, on a loose line, in the ink;
  - a section is ruled off under its heading;
  - a quotation is a box on the sunk surface, with an accent rule down its left;
  - a table is a rounded box on the raised surface. Its head row sits on the sunk surface, and its
    rows are parted by hairlines;
  - code stands on the sunk surface, and links are in the accent.
- **A sound stands at the head.** A document's sound files stand under its masthead in a listening
  band, each in the console's player, before the body. A narration is attached the same way. A page
  carries its own sound in a `listen` band.

## Pictures open whole

A picture in a document opens in the console's picture viewer, for a markdown document and a page
alike, on the console and under a share link.

- **What opens it:** a plain press (the main button, no key held) on a picture the document draws.
  - The viewer stands the picture whole over the screen, to zoom, move and pinch.
  - It steps to the document's other pictures by the arrows, the keys and a sideways stroke.
- **The run it walks** is every picture of the document that would open it, in the order they stand.
  - That covers the body's own pictures, a gallery's slides, the pictures among the attachments, and,
    for a page, every picture of its own.
  - A picture that is not drawn (hidden, or under a closed fold) is not in the run.
  - The head names the document, and where the picture stands among them.
- **Its properties:**
  - the picture's caption: its alternative text, or else the caption of its figure;
  - its resolution, once it has loaded;
  - the address it is fetched from.

  A picture with an address also opens in a tab of its own. A picture written into a page as data has
  no address, so it has neither of those two.
- **What stays with the document.** Where the document gives a picture's press a meaning of its own,
  the press does what the document says, and the viewer does not take it. That covers:
  - a picture inside a link, a button, a label or a fold's summary;
  - a picture that answers a press written into the markup, or stands in an element that does;
  - a press the document's own script has already handled, by taking its default away;
  - a picture marked `data-archive-no-viewer`, or standing in an element so marked.
  - A press with a key held, or with any button but the main one, is the browser's.
- **The pointer says it.** A picture that opens the viewer shows the zoom pointer.
- **A page asks from inside its frame.** The frame gives the page an origin of its own, so the console
  cannot read the page.
  - The lines ahead of the page's own read its pictures by the same rule, and hand the run to the
    frame as one message.
  - The console takes that message only from that frame, only in that shape, and only for pictures
    over the web or written into the page as data.
  - The page's file is never touched for this.
  - A page opened alone, in a tab of its own, has no frame to ask, and its pictures are left to it.
- **One rule for both.** Which pictures open is one rule, kept in the design's file, which the console
  and the page's lines both read.

## Where a person reads it

### The console screen

- **The archive screen** is at `/archive`, and one document is at `/archive/<id>`. It is this
  repository's console plugin bundle, loaded by bmo-web.
  - An address under `/searches` is sent on for good to the same place, with its id and query kept.
    `/collected` is sent on to `/archive`.
  - The screen reads only the rows this document describes, through this repository's read API.
- **Every document is open to the console.** A caller signed in to the console sees every document,
  with its attachments, pictures and source previews.
- **The list:**
  - It shows each document by its title, with its kind. The topic, the keywords and the rule stand on
    one line under it.
  - It can be narrowed to one kind, to the shared ones, and to the documents holding a phrase. Each
    narrowing is held in the address. A phrase is matched in the title, the question, the head, the
    room's name and the body.
  - It is a list of documents, so a search that left none is not in it.
- **A page opens as its page** (see "An HTML page"). Everything below about the body, the
  attachments, the sources and the contents applies to every other kind.
- **One document:**
  - The title stands at the top, or the question where there is no title.
  - The question, the topic, the keywords, the applied rule and the options are the document's
    properties, apart from the body. The kind decides which of them stand.
  - The findings are the body, with the attachments after them and the sources as a list beside them.
  - A footnote mark in the body is a link that brings its source forward in that list. On a phone it
    opens the list first.
  - The document file can be downloaded as it is.
- **A table of contents stands above the sources**, on the console and on a share page alike.
  - Its entries are the kind's outline, and pressing one brings the document to that section.
  - The entries and the heading marks come out of one reading of the text, so an entry always names a
    heading that is there.
  - What is folded away has no entry.

### The share page

- **A share link hands out one document.** It is made on the console or by the `archive` verbs.
  - Whoever holds it reads that document, with no account of their own, until the link's time runs
    out.
  - It opens the document alone: not its state, its properties or its document file. It reaches no
    other document and no other screen.
  - The link is `/share/<token>`, for every kind.
  - Expired links are swept by this repository.
- **A share link reads as its document before anything runs.** The page a link opens carries the
  document's own head in the HTML the server sends:
  - **Title:** the document's title, in the page title and in the OpenGraph and Twitter titles.
  - **Description:** the opening prose of the body as plain words, cut at about 160 characters.
    - That is the key answer and the summary, where the document opens with them.
    - Names, headings, pictures, tables, field lines and folded text are not prose. Footnote marks and
      inline markup fall away.
    - A document with no prose in its body is described by its topic.
  - **Picture:** the first picture the body draws, or else the first picture the document keeps as an
    attachment.
    - The link's own route hands it over as a JPEG made from the kept copy, fitted inside 1200 pixels,
      and asks for the large card.
    - A document with no picture names no preview image at all, and asks for the small card.
  - **Address:** the link itself is the canonical address, and the document is an article.
  - **Kept out of indexes:** the page asks search engines not to index or follow it.
  - **A page document's own tags:** its share page carries the preview tags its page writes, in place
    of the link's own tags of the same name.
- **The owner is never handed a share link.**
  - Every address given to the owner's private room is the document's console address.
  - A share link is for a room of other people, or one the owner makes on the console.
- **A long result reaches a room as one address, then the answer.** This applies to a result longer
  than about two and a half thousand characters.
  - The first line is the address of the whole document: the console address in the owner's private
    room, and the search's own share link in every other room.
  - The key answer and a few lines of summary follow.
  - The share link is made when the search closes, and lives a week. A link that already stands on the
    search is the one handed out.
  - The CLI verbs and the HTTP API write share rows through one module of this repository, so they
    cannot disagree about what a link is.
  - A search that left no document is answered with a summary alone, and so is one whose deployment
    names no console address, since neither address can be made without it.

## Out of scope

- **No recording tool for agents.** The web tools serve search runs, browse runs and job workers
  alike, so a recording tool there would be handed to agents with no document. The runner writes the
  document instead.
- **Talk and translations.** A document's talk and its translations belong to bmo-brain's document
  viewer and to bmo-job's work-order translation. This repository holds no table and serves no route
  for them.
- **Visibility per account.** Every console caller sees every document.
