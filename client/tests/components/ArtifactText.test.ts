import { describe, expect, test } from "bun:test";

// What a finding's text stands as before and after the reading is in the browser. The component is
// read rather than mounted: which element holds the text and which the HTML is what the file that
// ships decides.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const source = await read("../../app/components/ArtifactText.vue");
const template = source.slice(0, source.indexOf("<script"));
const script = source.slice(source.indexOf("<script"));
const loader = await read("../../app/domain/finding-load.ts");
const markSource = await read("../../app/components/ArtifactMarkSource.vue");
const templateOf = (file: string): string => file.slice(0, file.indexOf("<script"));
const config = await read("../../nuxt.config.ts");

describe("a finding's text", () => {
  test("stands as the markdown it was answered as until there is HTML to stand instead", () => {
    expect(template).toContain('<div v-if="html === null" v-bind="$attrs" class="break-words whitespace-pre-wrap">{{ props.markdown }}</div>');
    expect(template).toMatch(/<div\s+v-else\s+ref="shown"\s+v-bind="\$attrs"\s+class="markdown markdown--finding"[^>]*\sv-html="html"\s*\/>/);
  });

  // A fenced block's copy button is drawn with the text, so the press on it is taken where the text
  // is put on the screen.
  test("takes a press on a fenced block's copy button", () => {
    expect(script).toContain('import { copyFromPress } from "bmo-ui/app/domain/code-copy";');
    expect(script).toContain('import { copyText } from "../domain/copy";');
    expect(script).toContain("copyFromPress(t, event, copyText);");
    expect(template).toContain('@click="onPress"');
  });

  test("says when its formatted form is on the page, with the element that holds it", () => {
    expect(script).toContain("defineEmits<{ rendered: [element: HTMLElement] }>()");
    expect(script).toContain('emit("rendered", shown.value)');
  });

  test("draws the diagrams in its text once it is on the page, and again when the theme switches", () => {
    expect(script).toContain('import { drawDiagrams } from "bmo-ui/app/domain/diagram";');
    expect(script).toContain("void drawDiagrams(shown.value, dark.value);");
    expect(script).toContain("void drawDiagrams(shown.value, inDark);");
  });

  test("colours its code blocks when it is mounted with its HTML already drawn, and lets them go as the text is replaced or leaves", () => {
    expect(script).toContain("colourCode(shown.value, dark.value);");
    expect(script).toContain('{ flush: "post", immediate: true }');
    expect(script.match(/forgetCode\(shown\.value\)/g)).toHaveLength(2);
  });

  test("fills the places the reading left with the bundle's own fold, gallery and map", () => {
    expect(template).toContain('<Teleport v-for="block in blocks" :key="block.key" :to="block.place">');
    expect(template).toContain("<ArtifactFold ");
    expect(template).toContain("<ArtifactGallery ");
    expect(template).toContain("<ArtifactMap ");
  });

  // A footnote mark brings its source forward as a wiki's footnotes do: over the mark in the library's popover, above
  // it, for a pointer, and in a centred dialog for a finger; no press follows its link (docs/archive.md, "How a
  // document is drawn").
  test("brings a footnote mark's source forward in a popover above the mark, or in a dialog for a tap", () => {
    const shown = templateOf(markSource);

    expect(template).toContain('<UPopover v-model:open="sourceOpen" :reference="sourceMark ?? undefined" :content="SOURCE_PLACE">');
    expect(script).toContain('side: "top"');
    expect(template).toContain('@pointerenter="holdSource" @pointerleave="leaveSource"');
    expect(template).toContain('<UModal v-model:open="dialogOpen"');
    expect(template.match(/<ArtifactMarkSource :source="source" \/>/g)).toHaveLength(2);
    expect(script).toContain('(event as PointerEvent).pointerType === "touch"');
    expect(script).toContain("event.preventDefault();");
    expect(shown).toContain('target="_blank"');
    expect(template.match(/v-html/g)).toHaveLength(1);
  });

  test("draws the plain text while the page is hydrated, so the browser's first pass agrees with the server's", () => {
    expect(script).toContain("useNuxtApp().isHydrating ? null : findingReading()");
  });

  test("gets its HTML from the reading alone, which the browser fetches and the server never carries", () => {
    expect(script).toContain("reading.value.renderFinding(t, props.markdown, { images: props.images, files: props.files, slugs: props.slugs })");
    expect(script).toContain("await loadFindingReading()");
    expect(script).not.toMatch(/^import (?!type )[^\n]*domain\/finding-markdown"/m);
    expect(loader).toContain("if (!import.meta.client)");
    expect(loader).toContain('import("./finding-markdown")');
  });

  // The finding's design is the bundle's, so a host extending it carries the sheet with it.
  test("is drawn by the bundle's own stylesheet", () => {
    expect(config).toContain('new URL("./app/assets/css/finding.css", import.meta.url)');
  });
});
