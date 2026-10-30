import { describe, expect, test } from "bun:test";

// Where the listen band stands (`docs/archive.md`, "How a document is drawn"): the reader's own, drawn by
// the screen around a document of either kind and never inside a page's sandboxed frame. The components
// are read rather than mounted, as the file that ships decides what stands where.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const templateOf = (source: string): string => source.slice(0, source.indexOf("<script"));
const frame = await read("../../app/components/ArtifactPageFrame.vue");
const body = await read("../../app/components/ArtifactBody.vue");
const listen = await read("../../app/components/ArtifactListen.vue");
const screen = await read("../../app/pages/archive/[id].vue");
const shared = await read("../../app/pages/archive/share/[token].vue");

describe("a page's listen band", () => {
  test("stands above the page's frame, outside it, where the page keeps a head sound, and not at all where it keeps none", () => {
    const template = templateOf(frame);
    const band = template.indexOf('<div v-if="sounds.length > 0"');

    expect(band).toBeGreaterThan(-1);
    expect(template.indexOf("<ArtifactListen ", band)).toBeGreaterThan(band);
    expect(template.indexOf("<ArtifactHostedFrame ")).toBeGreaterThan(template.indexOf("<ArtifactListen "));
    expect(frame).toContain("const sounds = computed(() => headSounds(props.attachments));");
  });

  test("is the same band a markdown document draws under its masthead", () => {
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

  test("plays through the attachment route the screen reads the document on", () => {
    expect(templateOf(screen)).toContain(':attachments="held.attachments"');
    expect(templateOf(screen)).toContain(':address="fileAddress"');
    expect(templateOf(shared)).toContain(':attachments="held.attachments"');
    expect(templateOf(shared)).toContain(':address="bodyProps.address"');
  });
});
