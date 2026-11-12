import { describe, expect, test } from "bun:test";

// A button in a toolbar or a row of controls is the library's small one, and one that stands as its icon alone is the
// library's small square icon button with its words in its tooltip (`docs/artifact.md`, "Where a person reads it").
// The files are read rather than mounted, as the file that ships decides what stands where.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const templateOf = (source: string): string => source.slice(0, source.indexOf("<script"));
const between = (source: string, start: string, end: string): string => source.slice(source.indexOf(start), source.indexOf(end));
const list = templateOf(await read("../../app/pages/artifact/index.vue"));
const gallery = templateOf(await read("../../app/components/ArtifactGallery.vue"));

const buttons = (source: string): string[] => source.match(/<UButton\b[^>]*>/g) ?? [];

describe("the list's toolbar", () => {
  const toolbar = between(list, "<UDashboardToolbar", "</UDashboardToolbar>");

  test("holds the library's small buttons alone", () => {
    expect(buttons(toolbar).length).toBeGreaterThan(0);
    for (const button of buttons(toolbar)) expect(button).toContain('size="sm"');
  });

  test("stands the in-progress switch as its icon alone on a phone, its words in its tooltip there", () => {
    const tip = between(toolbar, '<HintTip class="flex" :text="$t(\'artifact.label.in_progress_only\')"', "</HintTip>");

    expect(tip).toContain(':disabled="!phone"');
    expect(tip).toContain(":label=\"phone ? undefined : $t('artifact.label.in_progress_only')\"");
    expect(tip).not.toContain("sr-only");
  });
});

describe("a finding's picture row", () => {
  test("steps through its pictures with the library's small buttons", () => {
    const row = gallery.slice(gallery.indexOf("</UCarousel>"));

    expect(buttons(row)).toHaveLength(2);
    for (const button of buttons(row)) expect(button).toContain('size="sm"');
  });
});
