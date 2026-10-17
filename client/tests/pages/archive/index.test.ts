import { describe, expect, test } from "bun:test";

// The Archive list screen, read off the file that ships: what a row says about being shared and what
// kind it is, and how the list's filters stand and are remembered. What the rows look like is checked
// in a browser.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const page = await read("../../../app/pages/archive/index.vue");
const detail = await read("../../../app/pages/archive/[id].vue");
const shared = await read("../../../app/pages/share/[token].vue");
const body = await read("../../../app/components/ArchiveBody.vue");

describe("there is a screen to read, so nothing below passes on an empty string", () => {
  test("each file carries one", () => {
    for (const source of [page, detail, shared, body]) {
      expect(source.length).toBeGreaterThan(2000);
    }
  });
});

describe("a row that is out on a link", () => {
  test("says so among its own chips, with what is left of the link", () => {
    expect(page).toContain('v-if="item.shared !== null"');
    expect(page).toContain("$t('archive.label.sharing', { left: shareLeftText($t, item.shared.expiresAt, now) })");
  });

  // A link with no end has no figure to count down, and the shared reading is what puts the word for
  // that in the figure's place — here and in the dialog alike.
  test("reads its remaining life through the one reading that knows a link may have no end", () => {
    expect(page).toContain("shareLeftText");
    expect(page).not.toContain("remainingText(");
  });

  test("keeps the figure moving, so a list left standing does not count down to a lie", () => {
    expect(page).toContain("useNowSeconds()");
  });
});

describe("one document read whole", () => {
  test("says the same thing beside its state", () => {
    expect(detail).toContain('t("archive.label.sharing", { left: shareLeftText(t, mark.expiresAt, now.value) })');
    expect(body).toContain('v-if="props.shared !== null"');
  });

  // The dialog is the one place a link is made and taken away, so the chip follows what it says rather
  // than the screen asking the server again once the dialog has closed.
  test("follows the dialog rather than reading the link for itself", () => {
    expect(detail).toContain('@shared="markShared"');
    expect(detail).toContain("detail.value.search.shared = mark");
  });

  test("offers no such chip to somebody holding a link, who is never told about it", () => {
    expect(body).toContain("shared?: string | null");
    expect(body).toContain("shared: null");
  });

  // The kind and its contents are the read API's to say, so neither page outlines a document itself.
  test("is outlined as the read API answers, on the console and under a link alike", () => {
    expect(detail).toContain("detail.value?.outline.entries ?? []");
    expect(detail).toContain("detail.value?.kind ?? null");
    expect(shared).toContain("detail.value?.outline.entries ?? []");
    expect(shared).toContain("detail.value?.kindEntry ?? null");
    expect(`${detail}${shared}`).not.toContain(".outline(");
  });
});

describe("holding the list to what is shared", () => {
  test("is one pressed toggle in the screen's own row, named the same on the button and on hover", () => {
    expect(page).toContain('<UTooltip :text="sharedLabel">');
    expect(page).toContain(':aria-label="sharedLabel"');
    expect(page).toContain(':aria-pressed="sharedOnly"');
    expect(page).toContain("sharedOnly ? 'primary' : 'neutral'");
    expect(page).toContain("sharedOnly ? 'solid' : 'ghost'");
  });

  // A toggle is named by the list a press would leave the reader with, and it spins while the read it
  // asked for is running — the reread button beside it does not, or a press on one would look like a
  // press on both.
  test("is named by the list it would leave you with, and carries its own spinner", () => {
    expect(page).toContain('sharedOnly.value ? t("archive.label.show_all") : t("archive.label.shared_only")');
    expect(page).toContain(`:loading="filtering === 'shared'"`);
    expect(page).toContain(':loading="loading && filtering === null"');
  });

  test("stands in the address, so a link opens the list held the same way", () => {
    expect(page).toContain('route.query.shared === "1"');
    expect(page).toContain('router.replace({ query: only ? { ...route.query, shared: "1" }');
  });

  test("is the server's to do, so a held page is a page of shared documents", () => {
    expect(page).toContain('shared: sharedOnly.value ? "1" : undefined');
  });

  test("reads the list again from its head when it moves, rather than filtering what is in hand", () => {
    expect(page).toContain("watch([sharedOnly, kindChoice], () => {");
    expect(page).toContain("items.value = [];");
  });

  test("says which kind of nothing an empty list is", () => {
    expect(page).toContain('sharedOnly.value ? t("archive.status.no_shared") : t("archive.status.no_documents")');
  });
});

// In a list a state stands as a compact icon; the chip of words is the document page's own.
describe("the state of each row", () => {
  test("is its icon, named in a tooltip and to a reader that cannot see it", () => {
    expect(page).toContain('<UTooltip :text="statusView($t, item.status).label">');
    expect(page).toContain('<span role="img" :aria-label="statusView($t, item.status).label"');
  });

  test("stands as no chip of words among the row's own", () => {
    expect(page).not.toContain(':label="statusView($t, item.status).label"');
  });
});

// Every document has a kind, and the list both names it on each row and can be held to one. The kinds
// come out of the catalogue the read API answers, so the screen keeps no list of its own and branches
// on none.
describe("the kind of each document", () => {
  test("stands on every row as the kind's own label", () => {
    expect(page).toContain(':label="$t(kindOf(item)!.labelKey)"');
  });

  test("is chosen from a dropdown built out of the catalogue, with every kind at once first", () => {
    expect(page).toContain("<USelect");
    expect(page).toContain("const kinds = useArchiveKinds();");
    expect(page).toContain('[{ label: t("common.label.all"), value: ALL_KINDS }, ...kinds.value.map((kind) => ({ label: t(kind.labelKey), value: kind.id }))]');
    expect(page).not.toMatch(/["'](search|import|learning|record)["']/);
  });

  test("stands in the address, read against the catalogue, and is the server's to do", () => {
    expect(page).toContain("kinds.value.find((kind) => kind.id === askedKind.value)?.id ?? ALL_KINDS");
    expect(page).toContain("kind: value === ALL_KINDS ? undefined : value");
    expect(page).toContain("kind: kindChoice.value === ALL_KINDS ? undefined : kindChoice.value");
  });

  test("the choice carries the spinner while the read it asked for runs", () => {
    expect(page).toContain(`:loading="filtering === 'kind'"`);
  });
});

// Looking for documents by a phrase. The search is the server's, over the whole archive, so the screen
// only carries the phrase there and marks what came back.
describe("searching the list", () => {
  test("is a field in the screen's own row, taking the width the filters leave", () => {
    expect(page).toContain(`<UDashboardToolbar :ui="{ left: 'min-w-0 flex-1', right: 'shrink-0' }">`);
    expect(page).toContain('icon="i-material-symbols-search"');
    expect(page).toContain(':maxlength="SEARCHES_PHRASE_LIMIT"');
    expect(page).toContain(`:aria-label="$t('common.action.clear_search')"`);
  });

  test("stands in the address, reached once typing pauses, and follows the address back", () => {
    expect(page).toContain('typeof route.query.q === "string" ? route.query.q : ""');
    expect(page).toContain("settling = setTimeout(holdToPhrase, SEARCH_SETTLE_MS);");
    expect(page).toContain('q: asked === "" ? undefined : asked');
    expect(page).toContain("if (searchPhrase(typed.value) !== held) typed.value = held;");
  });

  test("is the server's to do, beside the kind and the shared toggle", () => {
    expect(page).toMatch(/kind: kindChoice\.value === ALL_KINDS \? undefined : kindChoice\.value,\s+q: asked === "" \? undefined : asked/);
  });

  // The rows found by the last phrase stay until the new ones land, and the field is what spins.
  test("reads from the head without dropping the rows in hand, the field carrying the spinner", () => {
    expect(page).toMatch(/watch\(phrase, \(\) => \{\s+hasMore\.value = false;\s+void load\(null\);/);
    expect(page).toContain(`:loading="filtering === 'phrase'"`);
  });

  test("marks each row's hit through the console's list search, by the phrase the rows were found with", () => {
    expect(page).toContain(':text="searchTitle(item)" :query="searched"');
    expect(page).toContain('<SearchHit :match="rowMatch(item)" :query="searched" />');
    expect(page).toContain("matchFields(archiveSearchFields(t, item, roomName(item)), searched.value)");
  });

  test("says so, naming the phrase, when nothing is found", () => {
    expect(page).toContain('t("archive.status.no_match", { phrase: searched.value })');
  });
});

// The bundle is a layer a host extends, and the host binds the aliases of a project's own folders to
// its own; so the screen reaches its own files by relative path and its base library by package.
describe("what the screen imports", () => {
  test("names none of its own files through an alias", () => {
    for (const source of [page, detail, shared, body]) {
      expect(source).not.toMatch(/from "(~|@|#shared)\//);
    }
  });
});
