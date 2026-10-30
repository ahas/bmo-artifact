import { describe, expect, test } from "bun:test";

// Where a document's sounds stand (`docs/archive.md`, "How a document is drawn" and "An HTML page"): a markdown
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
