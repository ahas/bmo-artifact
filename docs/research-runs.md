# Research runs: profiles, clocks and branches

## What this document covers

- **The runs.** A search, which this repository runs, and a browse, which bmo-explorer runs, are both
  research runs: each starts once and ends. This document settles what coordinates them:
  - the research profile of each run's key, which this repository keeps for both;
  - whether the person named the depth, and what a full pool does to it;
  - how branches are taken from the common co-worker pool;
  - what makes a search stop.
- **Elsewhere.** How a search's findings are recorded and its document written is `docs/archive.md`;
  every call named here is listed in `docs/api.md`.

## The search

### When the agent searches

- **A room's session searches through its `search` tool,** which launches a search as `bmo search`
  does from a shell; the console's search button runs `bmo search` itself. The session has no web
  tool of its own, so every search it wants goes through this launch.
- **An answer that turns on facts that move is looked up rather than remembered,** and so is a word
  the agent does not recognise in what somebody said: a tool, a product, a term. The agent looks it
  up before it
  asks what it means, since the web answers that in a minute and the question handed back costs the
  asker a round trip.
- **What only the owner could know is still asked:** which of several things they mean, what they
  intend by it. A lookup that finds nothing is said plainly.
- **When someone keeps circling a problem,** the agent offers in one short line to look it up, and searches
  only when the offer is taken.

### The depth ladder

- **Three depths:** `basic`, `thorough` and `exhaustive`. Each has a research profile (see "Research
  profiles") and a tier assignment for its work key.
- **The agent launches `basic` without asking** which depth anyone wants; the deeper rungs are reached by
  escalation.
- **Escalation.** When the person who asked comes back wanting more — asking again, calling the answer
  thin, saying to dig deeper — the next search on that question runs one rung up. Someone who names a
  depth outright gets it at once.
- **The rung belongs to the question and the person who asked,** so a different question starts again
  at `basic`. At `exhaustive` there is nothing above, and the agent says so rather than running the same
  depth again as though it were the next rung.
- **That judgement is the room's session's,** made from the search procedure it reads before it
  launches. The launch passes the depth and whether the person insisted on it (see "A named depth, and
  a full pool"); the runner only takes the depth it was handed.
- **The depths differ in breadth and checking,** never in time:
  - `basic` opens no branch: a few queries in the run itself, only the pages it will quote opened;
  - `thorough` splits the question into the two to four angles that decide it, sends branches over
    them, and checks every quoted number against a second source;
  - `exhaustive` sweeps every angle that could bear on the answer.
- **The effort is not part of the prompt,** so changing it costs no cache.

### A follow-up writes into the earlier document

- **A search on what an earlier search of the room covered carries that search on** rather than filing
  a new document: more pictures, more detail, a narrower look, or the same question a rung deeper. The
  owner reads one document per topic.
- **The launch names the earlier search by its number** (`--into`). The agent is handed that number with
  every result.
- **The new run joins the earlier search's row,** opened again for it. What it finds is added after
  what the earlier runs found, and the document is written again from all of it under the same number,
  so its share link and console address keep opening it.
- **The head the first run gave stands;** a later run's head fills only what is still empty.
- **A follow-up refuses** another room's search, a document the agent wrote, a page imported whole, and a
  search still running. A launch that never came up puts the earlier search back as it stood.

### The launch and the runner

- **The launch sends nothing to the room.** It opens the search's row, opens the run's row in
  bmo-agency's run ledger, has bmo-agency launch `bmo runner search`, and returns at once. The session that launched it says in that same turn, in its own voice, that it is looking
  something up, or stays silent where the room's persona wants a quiet lookup.
- **The run stands outside the service,** so restarting the service cannot cut it short, and a slow
  search never holds the room's session open.
- **The runner searches the web in a run apart,** which bmo-agency starts, drives and reads: web access
  and pooled branches, no filesystem, no shell, no persona. It serves the agent and never speaks to the room.
- **Its instructions** are the search task, with the search-kinds rulebook appended and, of the other
  rulebooks the settings name, only the section that decides whether each applies. The run reads a
  whole rulebook through its web server's `read_rulebook` tool once one applies.
- **Its prompt is the request:** the question, who asked and when, the options, and the depth, with
  the knowledge bmo-gate picks for the question in the room under it, in the shape a turn's reference
  stands in.
- **It hands what it found back to the agent that asked** as a quiet instruction carrying the result
  and the run it ended, queued at bmo-receiver for that agent in the room it asked in, the agent the
  launch hands the runner (`docs/api.md`, "bmo-receiver"). That agent's session answers in its own voice,
  and until then neither waits nor answers from memory.
- **Every web request goes through the web tools** bmo-explorer serves, never the run program's own. A
  search engine that refuses this machine is not an empty result: the search tool says the search
  could not run and names the engines that refused, and the run writes what it has.
- **What the runner fetches itself** — the pictures a result names, a page brought in whole — goes
  through bmo-explorer by the tool host, so every outward request keeps its checks.
- **Nothing is handed to background work:** work a run leaves running is killed when the run ends, so
  a run that finished on a note about work still going would leave the room waiting for nothing.
- **The launch names the message that asked,** and the launcher reads its sender and time off
  bmo-receiver, so a
  restaurant or delivery search applies the asker's own rule without anyone writing it into the query,
  and a delivery search judges whether a place is open at the moment the question was asked.

### The result

- **Written for the agent to read and kept as a document people read,** in plain text with a fixed
  structure: a head (title, topic, keywords, and the filtering rule where one applied), the core
  answer, a three-line summary, the body, and what could not be confirmed. `docs/archive.md` sets out
  each part.
- **A question after one value** leaves no document: it is answered in the room, and its row records
  what was asked and how it ended.
- **Citations:** claims cite their pages by footnote marks, and each block closes with the pages it
  cites. A claim with no page behind it is written as unconfirmed.

### Whether a run finished

- **The runner decides,** never the run's own account of itself. The run's record names the background
  tasks still live when it produced its result; a result produced with any of them live is refused as
  "the run ended with its own work still running".
- **A second net:** a result is written in the structure the task defines, and a note about a search
  still under way is written in none of it.
- **Either way the room is told in one line** that the search failed and nothing more is coming. The
  room hears the same line when the run dies, or a usage limit or an outage ends it, so a search never
  ends in silence.

### The run row

- **Every search is recorded as a run** in bmo-agency's run ledger: the launch opens it naming the
  agent, the room, the unit, the rung and the search it carries, once the search's own row stands.
- **The runner closes it** with how it ended — done, unanswered, stopped, failed, or refused for leaving
  its own work running — and with the context the run ended carrying. The close is sent once; a run
  already closed keeps its first closing. A run that never came up is closed by its launcher as an
  error.
- **What the run cost** is recorded by bmo-agency, which walks the run's accounts and reads its reply.
  Nothing of the query or the result goes into the run; it is a record of a run, not of what was asked.

## The browse

- **What a search cannot answer because it has to be seen** runs through bmo-explorer's browse tool.
  That run, its launch and its runner are bmo-explorer's.
- **This repository keeps two things of it:** the `browse` research profile, read through the
  research-settings routes, and the branch verb, which a browse run calls with `--source browse`.

## Research profiles

This repository keeps one profile per research key, in its `research_profiles` table. The keys are the
work keys: `search:basic`, `search:thorough`, `search:exhaustive` and `browse`. Which keys there are,
and what is true of each, is the `[research]` settings table (see "What each key declares").

### What a profile sets

- **Branch role and count:** the co-worker role taken from the pool, and the most branches one run may
  open. A key that opens no branch holds 0. The role must be one the pool has enabled.
- **Wait limit:** the longest a run whose depth was named may wait for a pool place.
- **Fetch strategy:** one of the strategies its key accepts.
  - A search key: `direct-then-browser` (a direct request, a real browser only when blocked),
    `browser-first`, or `direct-only`.
  - The browse key: `visible` or `headless`.
- **Host interval and engine interval:** the least time between page requests to one host, and between
  queries to one search engine.
- **Total time and wrap-up time,** only on a key that holds a clock. The total time is the ceiling at
  which the run is cut; the wrap-up time is the stretch before it in which nothing new is opened and the
  result is written. A key without a clock holds neither: every read answers both as empty for it,
  whatever the row holds.
- **No model and no effort.** The tier is the assignment of the key's work key, kept by bmo-agency; a
  profile names neither, so one work's model is never decided in two places.

### What each key declares

Each key in the settings states its own:

- its **line**: `search` for the three search keys, `browse` for the browse key — the line its provider
  must carry;
- whether it holds a **clock**;
- whether it is **branchless**, which holds its branch count at 0;
- the **strategies** it accepts;
- its **seed**: the values a new profile, and a profile put back, starts from.

| Key | Line | Clock | Branches | Seed |
|---|---|---|---|---|
| `search:basic` | search | no | none | 0 branches, `direct-then-browser` |
| `search:thorough` | search | no | up to the limit | scout × 4, `direct-then-browser` |
| `search:exhaustive` | search | no | up to the limit | scout × 8, `direct-then-browser` |
| `browse` | browse | yes | up to the limit | 0 branches, `visible`, 30 minutes total, 3 minutes wrap-up |

Every seed waits at most 10 minutes for a place, and keeps a host interval of 1 second and an engine
interval of 2 seconds. A key added to the settings with these five answers needs no change in the code.

- **A key with no row yet reads as its seed,** set at no time, until a profile is saved for it. Putting
  a profile back writes its seed as the row.

### The limits

The settings also hold the bounds every check reads:

- the total time, from 30 seconds to 2 hours;
- the wrap-up time, at least 10 seconds;
- the research left after the wrap-up time, at least 10 seconds;
- the branch count, at most 16;
- the wait limit, at most 1 hour;
- each interval, at most 1 minute;
- a pool limit, at most 128 places.

### Editing and checks

- **Where:** the console's agents screen, through `POST /research-settings` (`docs/api.md`, "Research
  settings").
- **What is refused,** each with its field and a snake_case code, in the form's order:
  - a total time or a wrap-up time on a key without a clock, or either missing on a key with one;
  - a total time out of its range, a wrap-up time under its least, or a wrap-up time that leaves less
    research than its least;
  - a branch count out of its range, or any branch on a branchless key;
  - a role name that is not a role name, or a role the pool has not enabled where branches are allowed;
  - a wait limit or an interval out of its range;
  - a strategy the key does not accept.
- **A profile put back to its seed** passes the same checks.
- **The tier is not edited here.** It is the work key's assignment in bmo-agency, which checks it as it
  checks any assignment. A tier whose model the research provider cannot run is shown with its reason,
  and the run is refused when it starts.

### A run keeps the settings it started with

- **One copy for the run:** the launcher reads the profile and hands the runner the same copy, as a
  snapshot naming its key. A snapshot for another key, or one whose values fail the checks, is refused.
- **A change applies from the next run.**

### What the settings open

- **Three of a research run's tools are opened by its settings:** the picture viewer, the branches and
  the rulebooks, each by the capability it asks for. The settings tie each capability to the run
  variable that opens it; no launcher names one in its code.
- **A run holds a capability where its variable names something:** at least one comma-separated item
  that is neither empty nor `0`, so a switch set to `1` and a list of rulebooks open it alike.
- **A run carrying no research setting at all** holds none of them, whatever its role gives. What a run
  withholds is named in capability order.

## Model and reasoning level

- **Both come from the tiers,** which bmo-agency keeps. The runner reads the work key's tier and takes
  that tier's model and effort on the provider it launches on.
- **The provider** is the first enabled provider, in bmo-agency's order, whose program carries the
  run's line. With none, the run does not start and ends as a settings error.
- **The effort must be accepted** by the resolved provider and model, as bmo-agency's check answers. An
  effort it does not accept is never quietly lowered or raised: the run does not start.

## A research run set off on a registration

- **A registration that keeps its research takes its callers' research.** A search set off by a run on
  such a registration goes out on that same registration in place of the key's tier mapping.
- **Who set it off is read off the launching process:** one started under a program on the registry's
  road stands inside that registration's home. The launcher asks bmo-agency which registration owns
  that home and whether it keeps its research, and hands the runner the registration where it does.
  A launch from anywhere else names none.
- **It opens no branch at any depth.** The runner is handed its profile with no branch allowed, so
  nothing is reserved or waited for, and an `explicit` depth is never refused for a full pool. The
  depth is never lowered either: a deeper depth is the run's own wider search.

## A search has no clock

- **No ceiling, no research deadline,** and no depth raised by time. One question, one run, which goes
  on until it has written its answer.
- **A limit in time** closes regardless of whether the answer is good, and where it closes is the
  middle of writing the result, so the room gets nothing at all. A slow search is better than a cut one.

### The only three places a search stops

1. **The done condition.** Before its first query the run derives a done condition from the question;
   the moment it is met, research stops and the result is written. An unconfirmed item that does not
   change the answer is written as unconfirmed and left.
2. **The branch limit.** A branch request beyond the profile's count is refused, finally. When the
   search engine refuses twice in a row, that process's web search stops the same way.
3. **A person stopping it.** The owner presses Stop on the run in the console's live status screen, or
   stops its unit from a shell. The runner receives SIGTERM or SIGINT, stops its run apart through
   bmo-agency, giving the work under way up to 30 seconds to come to rest, keeps the last structured
   result the run wrote, returns it to the room, and closes the run as `stopped`. Where nothing was written, the room
   is told in one line that the search was stopped before it came to anything. A stop is not a failure.

### How a finished run is closed

- **The run judges itself** in the first line of its result: whether its answer met the done
  condition. One that met it closes as `done`; one that did not closes as `unanswered`, and the room
  hears that the answer did not go all the way.
- **Looking deeper** happens when the room asks again, never by the runner's own decision.

### A run that dies

- **A runner that dies writes no ending.** bmo-job's watcher closes a run whose unit is gone, and the
  search it carried with it, so no search stands as "researching" once its runner is gone.

## The browse clock

- **A browse run's clock starts** when its model process is launched:
  - the research deadline is the start plus the total time less the wrap-up time;
  - the ceiling is the start plus the total time.
- **After the research deadline** the web tools refuse new searches and pages, saying to write the
  result from what is at hand. A branch taken by a browse run is told the same deadline.
- **A search run's clock** has no deadline and no ceiling; its branches are told none.

## A named depth, and a full pool

### Whether the depth is required

- **The launch says whether the depth is required:**
  - `explicit`: the person named that depth, or chose it on the console;
  - `optional`: anything else — a depth the session chose, a rung raised after being asked to look
    further.

### Room in the pool

- **When a depth that opens branches starts,** the runner asks bmo-agency whether the pool has room at
  the branch role's tier: the limit less the places active, reserved and waited for.
- **No room** is said in a sentence: the role does not resolve, is not an enabled pooled role, has no
  limit on its provider and tier, or its pool is full with its counts.

### What each kind does

- **`optional` with no room,** or a limit of 0: the run starts at the shallowest depth, which records it,
  and the reason is logged.
- **`explicit`:** never lowered. The runner reserves the branch places and waits until the first is
  taken, polling twice a second. Past the wait limit it releases the reservation, and the room is told
  as a failure that the pool stayed full; a run stopped while waiting releases it the same way.

## Branches are taken from the pool

- **A branch is opened by the branch tool,** never by the run program's own subagents, which follow the
  parent's model and cannot take a place before they start.
- **The branch tool runs `bmo search branch`,** a process of its own the run's web server stops when the
  run ends. Its angle comes on standard input; the parent's research close, fetch settings, pool owner,
  role and branch number come as arguments.
- **Branch calls go out in parallel:** every web tool declares itself read-only, so branch calls made in
  one turn queue in the pool together and run at once as far as the pool has places.

### One call, step by step

1. **Refuse** where research has closed. The parent's tool refuses a branch beyond the count, and one
   asked from inside a branch, before the verb is run.
2. **Find the provider:** the first enabled provider carrying the search line. With none, the branch is
   refused, naming why.
3. **Queue a request** in the pool under the owner's key and the branch's number, with the profile's
   branch role. The model and effort resolve from that role's tier, whatever the parent's are. A place
   the run's reservation holds is taken over without queueing again.
4. **Wait** until the request is given a place, asking twice a second. Where the parent ends or research
   closes first, finish the request as cancelled and refuse.
5. **Bind the branch to its place,** and start it only then. Where the parent ended meanwhile, finish
   the request and refuse rather than start it.
6. **Run the branch apart:** the branch line on the place's model and effort, its web server told the
   parent's research close and fetch settings. A watch stops it the moment the verb is told to stop.
7. **Finish the request once** whatever the ending — completed, stopped at research close, or failed —
   and record the place's write where it answered.
8. **Answer the branch's notes,** headed and closed with the branch's number and marked as unchecked
   working notes, keeping only the pictures the branch's own record shows it looked at. A branch cut at
   research close answers what it had written, saying where it was stopped.

Every refusal and failure is said as a line telling the parent to write its result from what it has.

### What a branch is given

- **Tools:** only the web tools and the picture viewer — no branch tool, no shell, no files.
- **Clock:** a search branch has none, and reports once it has answered its angle. A browse branch takes
  its parent's research deadline.
- **Lifetime:** it is stopped at its parent's research close or when its parent ends, and hands back
  what it had written by then.
- **Results** come back as working notes the parent has not checked; the parent verifies what it
  carries into its result.

### Cleanup

- **When a run ends,** the runner finishes every request and releases every reservation it held.
- **A runner that died** leaves its requests and reservations behind. The next research run's start
  reclaims only those whose owner has ended: its run is closed in bmo-agency's ledger and its unit no
  longer runs, or the ledger holds no such run. An owner
  that names neither a search nor a browse run, or whose number is not a number, is never taken as
  ended.
- **Time alone reclaims nothing.**

### Reserving branch places

- **Before starting the model,** a run that opens branches reserves as many places of the branch role
  as its profile allows, and holds the reservation until it ends.
- **What that buys:** branch calls take over the held places without queueing, and places another run
  reserved are left out of the room an `optional` depth measures.
- **A branch place is a warm prefix,** not a checkout: every branch starts on the one branch line, so
  branches on the same provider, model and effort share a prefix whichever run they belong to. Each
  place a branch answered on is recorded under that line's provider, model and effort, which
  bmo-agency's pool keeps warm while the reservation stands.

## Fetch strategy

- **A search reads with a direct request first** by default, and hands over to a real browser only on a
  block or an empty script shell. `browser-first` is the value the owner picks where blocks are
  frequent; `direct-only` never starts a browser.
- **A browse is a browser run by nature;** how its browser is drawn is bmo-explorer's.
