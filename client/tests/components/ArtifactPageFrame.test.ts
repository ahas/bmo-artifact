import { describe, expect, test } from "bun:test";

// Where a document's sounds stand (`docs/artifact.md`, "How a document is drawn" and "An HTML page"): a markdown
// document's in the reader's listen band under its masthead, a page's where the page writes them, inside its
// sandboxed frame, with no band of the reader's over it. The components are read rather than mounted, as the
// file that ships decides what stands where.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const templateOf = (source: string): string => source.slice(0, source.indexOf("<script"));
const frame = await read("../../app/components/ArtifactPageFrame.vue");
const body = await read("../../app/components/ArtifactBody.vue");
const listen = await read("../../app/components/ArtifactListen.vue");
const screen = await read("../../app/pages/artifact/[id].vue");
const shared = await read("../../app/pages/artifact/share/[token].vue");
const framing = await read("../../app/domain/hosted-page.ts");

describe("a page's sounds", () => {
  test("have no band of the reader's over the page's frame", () => {
    expect(templateOf(frame)).not.toContain("<ArtifactListen");
    expect(templateOf(frame)).not.toContain("<AudioPlayer");
    expect(frame).not.toContain("headSounds");
  });

  test("are asked for in the scheme and the language the console stands in", () => {
    expect(framing).toContain('page()({ scheme: schemeOf(dark.value ? "dark" : "light"), language: locale.value })');
    expect(frame).toContain("useFramedPage(() => props.page)");
    expect(screen).toContain("const page: PageServed = (asked) => contentAddress(documentId.value, asked);");
    expect(templateOf(screen)).toContain('<ArtifactPageFrame v-else-if="hosted" :title="held.document.title" :labels="held.document.labels" :page="page" />');
    expect(templateOf(shared)).toContain(':page="(asked) => sharedContentAddress(token, asked)"');
  });
});

describe("a markdown document's listen band", () => {
  test("stands under its masthead", () => {
    expect(templateOf(body)).toContain('<ArtifactListen v-if="files.head.length > 0" :sounds="files.head" :address="props.address" />');
    expect(templateOf(body)).not.toContain("<AudioPlayer");
  });

  test("lists each sound in the design's player, labelled with its language where its name says one", () => {
    const template = templateOf(listen);

    expect(template).toContain('v-for="sound in props.sounds"');
    expect(template).toContain('<p v-if="languageOf(sound) !== null"');
    expect(template).toContain('<AudioPlayer :src="props.address(sound)" :download="props.address(sound, true)"');
    expect(template).not.toContain("<audio");
  });
});

// A document's instruction stands among the actions of the screen's own row for every kind, never under its title
// (`docs/artifact.md`, "Where a person reads it").
describe("a document's instruction", () => {
  test("opens from the screen's own row, the first of the actions on the document, and is drawn under no title", () => {
    expect(screen).toContain('...(instruction.value === null ? [] : [{ id: "instruction", label: t("artifact.action.instruction"), icon: "i-material-symbols-assignment-outline", onSelect: openInstruction }])');
    expect(templateOf(screen)).toContain('<ArtifactInstruction v-if="instruction !== null" v-model:open="instructionOpen" :text="instruction" />');
    expect(screen).toContain("labelsBesideInstruction(held.value.document.labels)");
    expect(templateOf(body)).not.toContain("description");
  });
});

// A title stands on the type scale and is never cut mid-word with nothing to read it whole by (`docs/artifact.md`,
// "How a document is drawn" and "Where a person reads it").
describe("a document's title", () => {
  test("in a page's row wraps between words onto two lines at most and leaves its tooltip to the cut-line layer", () => {
    const template = templateOf(frame);

    expect(template).toContain('<h1 class="line-clamp-2 text-lg font-semibold break-keep text-highlighted [overflow-wrap:anywhere] sm:text-xl" data-page-title>');
    expect(template).not.toContain("truncate");
    expect(template.match(/<h1 [^>]*>/g)).toHaveLength(1);
    expect(template.match(/<h1 [^>]*>/)![0]).not.toContain(":title");
  });

  test("in a markdown document's masthead stands at the section title step at every width", async () => {
    const sheet = await read("../../app/assets/css/document.css");
    const title = sheet.slice(sheet.indexOf(".artifact-title {"));

    expect(title.slice(0, title.indexOf("}"))).toContain("font-size: 1.5rem;");
    expect(sheet.match(/\.artifact-title\b/g)).toHaveLength(1);
  });

  test("in a page's own masthead heads a section at 20px and a part at 18px, at every width", async () => {
    const design = JSON.parse(await read("../../design/document.json")) as { element_rules: [string, string][]; narrow_element_rules: [string, string][] };
    const size = (selector: string): string | undefined => design.element_rules.find(([one]) => one === selector)?.[1].match(/font-size:(\d+px)/)?.[1];

    expect([size("h1"), size("h2"), size("h3"), size("h4,h5,h6")]).toEqual(["24px", "20px", "18px", "16px"]);
    expect(design.narrow_element_rules.filter(([selector]) => /^h\d/.test(selector))).toEqual([]);
  });
});

// Nothing a screen holds stands in a title bar: a document's way back stands at the left end of the screen's own row
// under the bar, and the actions on it at its right end, under the console's bar and the screen's own alike
// (`docs/layout.md`, "The console bundle"; `docs/artifact.md`, "Where a person reads it").
describe("a document's screen row", () => {
  const template = templateOf(screen);
  const row = template.slice(template.indexOf("<UDashboardToolbar"), template.indexOf("</UDashboardToolbar>"));

  test("lays nothing into the console's title bar, and its own bar holds its name and mark alone", () => {
    expect(screen).not.toContain("screenActions");
    expect(screen).not.toContain("screenLead");
    expect(template).not.toContain("<Teleport");
    expect(template).toContain(`<UDashboardNavbar v-if="!barOffered" :title="$t('artifact.title.screen')" icon="i-material-symbols-archive-outline" />`);
    expect(template.match(/<UDashboardToolbar[ >]/g)).toHaveLength(1);
  });

  test("stands the way back at its left end and all that acts on the document at its right end, the toggle first", () => {
    const left = row.slice(row.indexOf("#left"), row.indexOf("#right"));
    const right = row.slice(row.indexOf("#right"));

    expect(left.indexOf('@click="goBack"')).toBeGreaterThan(-1);
    expect(left.indexOf('@click="goBack"')).toBeLessThan(left.indexOf("data-document-number"));
    expect(right).toContain('<ActionGroup v-if="translation !== null" :actions="[translation]" :narrow-fold="false" />');
    expect(right.indexOf(':actions="[translation]"')).toBeLessThan(right.indexOf(':actions="actions"'));
    expect(right).toContain(':properties="held === null ? null : properties"');
    expect(right.match(/:narrow-fold="false"/g)).toHaveLength(2);
    expect(templateOf(frame)).not.toContain("<ActionGroup");
  });

  test("offers each kind's own actions between the instruction and the sharing", () => {
    expect(screen).toContain("const ownActions = computed(() => (hosted.value ? pageActions.value : writtenActions.value));");
    expect(screen).toContain("references: () => (narrow.value ? openSide : null)");
    expect(screen.indexOf('id: "instruction"')).toBeLessThan(screen.indexOf("...ownActions.value"));
    expect(screen.indexOf("...ownActions.value")).toBeLessThan(screen.indexOf('id: "share"'));
  });
});

// A markdown document's masthead holds no action of the console's screen: they all stand in its row, and only a share
// page, with no such row, offers its referred pages under the title (`docs/artifact.md`, "Where a person reads it").
describe("a markdown document's masthead", () => {
  const masthead = templateOf(body).slice(templateOf(body).indexOf("<header"), templateOf(body).indexOf("</header>"));

  test("offers its referred pages alone, and only where its screen hands them", () => {
    expect(masthead.match(/<ActionGroup /g)).toHaveLength(1);
    expect(masthead).not.toContain("<UTabs");
    expect(masthead).not.toContain(":properties=");
    expect(body.match(/id: "/g)).toHaveLength(1);
    expect(body).toContain('id: "references"');
    expect(templateOf(screen)).not.toContain(":references=");
    expect(templateOf(shared)).toContain(':references="() => { sideOpen = true; }"');
  });

  test("still says what the translation says of itself under the title", () => {
    expect(masthead).toContain('data-reading-state="asking"');
    expect(masthead).toContain('@click="askAgain"');
  });
});
