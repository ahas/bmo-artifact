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
const screen = await read("../../app/pages/archive/[id].vue");
const shared = await read("../../app/pages/archive/share/[token].vue");

describe("a page's sounds", () => {
  test("have no band of the reader's over the page's frame", () => {
    expect(templateOf(frame)).not.toContain("<ArtifactListen");
    expect(templateOf(frame)).not.toContain("<AudioPlayer");
    expect(frame).not.toContain("headSounds");
  });

  test("are asked for in the scheme and the language the console stands in", () => {
    expect(frame).toContain('props.page({ scheme: schemeOf(dark.value ? "dark" : "light"), language: locale.value })');
    expect(templateOf(screen)).toContain(':page="(asked) => contentAddress(documentId, asked)"');
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

// A document's instruction stands behind a button of the screen's bar for every kind, never under its title
// (`docs/artifact.md`, "Where a person reads it").
describe("a document's instruction", () => {
  test("opens from the screen's bar, in the console's bar and in the screen's own alike, and is drawn under no title", () => {
    const template = templateOf(screen);

    expect(template.match(/data-instruction/g)).toHaveLength(2);
    expect(template.match(/@click="openInstruction"/g)).toHaveLength(2);
    expect(template).toContain('<ArtifactInstruction v-if="instruction !== null" v-model:open="instructionOpen" :text="instruction" />');
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

// A document in progress keeps the screen's bar to one row on a phone (`docs/artifact.md`, "Where a person reads it").
describe("a document's in-progress badge", () => {
  test("stands as its mark alone with its words in its tooltip in a phone's bar, in both bars", async () => {
    const badge = templateOf(await read("../../app/components/ArtifactPendingBadge.vue"));

    expect(badge).toContain('<HintTip v-if="lacks !== null && props.compact" class="flex" :text="said">');
    expect(badge).toContain(':aria-label="said" data-pending />');
    expect(templateOf(screen).match(/<ArtifactPendingBadge [^>]*:compact="phone" \/>/g)).toHaveLength(2);
    // A finger reaches every tooltip of the bar through the shared one: by a tap on the badge, by resting on a button.
    expect(templateOf(screen)).not.toContain("<UTooltip");
    expect(templateOf(screen).match(/<HintTip [^>]* press[ >]/g)).toHaveLength(4);
  });
});
