import { describe, expect, test } from "bun:test";
import { Glob } from "bun";
import { parse } from "vue/compiler-sfc";

// A value is chosen from a list in bmo-ui's one select (`docs/artifact.md`, "Where a person reads it"; bmo-ui's
// `docs/components.md`, "Choosing a value"): no screen of the bundle stands the component library's select, select
// menu or input menu bare, nor a dropdown menu of checks standing in for a select, and none decides whether its list
// searches, since the layer gives the search once a list holds more than seven choices. That is read here off the
// files that ship, by the template compiler that reads them for the build; how the select itself searches is the
// layer's own tests'.

const app = new URL("../../app/", import.meta.url).pathname;

interface Prop {
  type: number;
  name: string;
  arg?: { content?: string };
}

interface Node {
  type: number;
  tag?: string;
  props?: Prop[];
  children?: readonly (Node | string | symbol)[];
  branches?: readonly Node[];
}

interface Found {
  path: string;
  node: Node;
}

const ELEMENT = 1;
const ATTRIBUTE = 6;
const BARE = new Set(["USelect", "USelectMenu", "UInputMenu", "u-select", "u-select-menu", "u-input-menu"]);
const CHOICE = new Set(["ChoiceSelect", "choice-select"]);
// What the layer settles for every list, so a screen hands none of it.
const SETTLED = new Set(["search-input", "searchInput", "searchable", "ignore-filter", "ignoreFilter", "search-term", "searchTerm"]);

function every(node: Node, out: Node[]): Node[] {
  if (node.type === ELEMENT) out.push(node);
  for (const child of [...(node.children ?? []), ...(node.branches ?? [])]) {
    if (typeof child === "object") every(child, out);
  }

  return out;
}

// The name a prop is handed by, written plainly, bound or through a model.
function propNames(node: Node): string[] {
  return (node.props ?? []).map((one) => (one.type === ATTRIBUTE ? one.name : (one.arg?.content ?? one.name)));
}

const sources = new Map<string, string>();

for await (const path of new Glob("{pages,components,layouts}/**/*.vue").scan({ cwd: app })) {
  sources.set(path, await Bun.file(`${app}${path}`).text());
}

function elements(): Found[] {
  return [...sources].flatMap(([path, text]) => {
    const { descriptor } = parse(text, { filename: path });

    return descriptor.template?.ast ? every(descriptor.template.ast as unknown as Node, []).map((node) => ({ path, node })) : [];
  });
}

describe("choosing a value", () => {
  const all = elements();
  const choices = all.filter((one) => CHOICE.has(one.node.tag ?? ""));

  test("every list a value is chosen from stands in the one select", () => {
    expect(choices.map((one) => one.path).sort()).toEqual(["components/ArtifactUnitShareDialog.vue", "pages/artifact/index.vue", "pages/artifact/index.vue"]);
    expect(all.filter((one) => BARE.has(one.node.tag ?? "")).map((one) => `${one.path}: ${one.node.tag}`)).toEqual([]);
  });

  test("no dropdown menu of checks stands in for a select", () => {
    expect([...sources].filter(([, text]) => /type:\s*["']checkbox["']/.test(text)).map(([path]) => path)).toEqual([]);
  });

  test("no screen decides whether its list searches", () => {
    const deciding = choices.flatMap((one) => propNames(one.node).filter((name) => SETTLED.has(name)).map((name) => `${one.path}: ${name}`));

    expect(deciding).toEqual([]);
  });
});
