import { describe, expect, test } from "bun:test";

// A button in a toolbar or a row of controls is the library's small one, and one that stands as its icon alone is the
// library's small square icon button with its words in its tooltip (`docs/artifact.md`, "Where a person reads it").
// The files are read rather than mounted, as the file that ships decides what stands where.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const templateOf = (source: string): string => source.slice(0, source.indexOf("<script"));
const between = (source: string, start: string, end: string): string => {
  const from = source.indexOf(start);

  return source.slice(from, source.indexOf(end, from));
};
const listSource = await read("../../app/pages/artifact/index.vue");
const list = templateOf(listSource);
const listScript = listSource.slice(listSource.indexOf("<script"));
const gallery = templateOf(await read("../../app/components/ArtifactGallery.vue"));

const buttons = (source: string): string[] => source.match(/<UButton\b[^>]*>/g) ?? [];

describe("the list's toolbar", () => {
  const toolbar = between(list, "<UDashboardToolbar", "</UDashboardToolbar>");

  test("holds the library's small buttons alone", () => {
    expect(buttons(toolbar).length).toBeGreaterThan(0);
    for (const button of buttons(toolbar)) expect(button).toContain('size="sm"');
  });

  test("stands the filters only in the list and where the bar has the room for them", () => {
    const standing = between(toolbar, '<template v-if="!front && !gathered">', "</template>");

    expect(standing).toContain('<ChoiceSelect\n              v-for="filter in filters"');
  });

  test("folds them into one filter popover at its end, opened by a small square icon button named in its tooltip, where the bar gathers", () => {
    const popover = between(toolbar, '<UPopover v-if="!front && gathered"', "</UPopover>");
    const [button] = buttons(popover);
    const content = between(popover, "<template #content>", "</template>");

    expect(toolbar.trimEnd().endsWith("</UPopover>\n        </template>")).toBe(true);
    expect(popover).toContain(":content=\"{ align: 'end' }\"");
    expect(popover).toContain("<UTooltip :text=\"$t('artifact.label.filters')\">");
    expect(button).toContain("square");
    expect(button).toContain(":aria-label=\"$t('artifact.label.filters')\"");
    expect(content).toContain('<UFormField v-for="filter in filters" :key="filter.key" :label="filter.name">');
  });

  test("puts the filter popover away on every choice made in it, before the address takes the choice", () => {
    const popover = between(toolbar, '<UPopover v-if="!front && gathered"', "</UPopover>");
    const content = between(popover, "<template #content>", "</template>");

    expect(popover).toContain('v-model:open="filtersOpen"');
    expect(content).toContain('@update:model-value="(value: string) => fromPopover(() => filter.hold(value))"');
    expect(listScript).toContain("filtersOpen.value = false;\n  choose();");
  });

  test("decides whether to gather by the bar's own room, as every bar does", () => {
    expect(listScript).toContain('import { useBarGathers } from "bmo-ui/app/domain/actionbar";');
    expect(listScript).toContain("const gathered = useBarGathers(() => bar.value?.$el);");
    expect(toolbar).toContain('<UDashboardToolbar ref="bar"');
  });
});

describe("a finding's picture row", () => {
  test("steps through its pictures with the library's small buttons", () => {
    const row = gallery.slice(gallery.indexOf("</UCarousel>"));

    expect(buttons(row)).toHaveLength(2);
    for (const button of buttons(row)) expect(button).toContain('size="sm"');
  });
});
