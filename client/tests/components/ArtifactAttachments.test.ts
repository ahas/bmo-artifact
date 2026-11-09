import { describe, expect, test } from "bun:test";

// How the files after a document's body stand (`docs/artifact.md`, "The files after the body"). The
// components are read rather than mounted: what a row draws is what the file that ships decides.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const source = await read("../../app/components/ArtifactAttachments.vue");
const template = source.slice(0, source.indexOf("<script"));
const script = source.slice(source.indexOf("<script"));
const body = await read("../../app/components/ArtifactBody.vue");

describe("the files after a document's body", () => {
  test("draw a picture as its row's thumbnail at the console's avatar size, read as it comes into view, and never large", () => {
    expect(template).toContain('<UAvatar :src="props.address(attachment)" :alt="attachment.name" :icon="KIND_ICONS.image" loading="lazy" class="shrink-0 rounded-md" />');
    expect(template).not.toMatch(/<UAvatar[^>]*\ssize=/);
    expect(template).not.toMatch(/<img\b/);
  });

  test("open a picture whole from one button holding the thumbnail and the name, named in the language pack's words", () => {
    expect(template).toContain('type="button"');
    expect(template).toContain(":aria-label=\"$t('artifact.action.view_picture', { name: attachment.name })\"");
    expect(template).toContain('@click="openPicture($event, attachment)"');
    expect(script).toContain("plainPress(event) ? keptPictures(props.attachments, attachment, (one) => props.address(one), window.location.href) : null");
    expect(script).toContain('emit("pictures", run)');
  });

  test("hand the run to the document's one picture viewer, which also opens the body's pictures", () => {
    expect(body).toContain('@pictures="showPictures"');
    expect(body.match(/<ArtifactPictureViewer /g)).toHaveLength(1);
    expect(body).toContain("const files = computed(() => filesAround(props.attachments));");
  });

  test("keep every other kind's row: a sound in the player, a film and a text under their rows, a page opened in a tab", () => {
    expect(template).toContain("<AudioPlayer");
    expect(template).toContain("<LoadingVideo v-if=\"attachment.kind === 'video'\"");
    expect(template).toContain("<ArtifactTextPreview v-else-if=\"attachment.kind === 'text'\"");
    expect(template).toContain("$t('artifact.action.open_page')");
  });
});
