import { describe, expect, test } from "bun:test";

// What a document of the Archive keeps off the screen when the reader is not the owner. The state
// chip, the properties and the document's own file are each a caller's to offer, and a share page
// offers none of the three; what must not happen then is a row or a box standing empty where they
// would have been. All of it is which element stands under which condition, which a rendered screen
// would show as nothing at all, so the rule is read off the files that ship.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const body = await read("../../app/components/ArchiveBody.vue");
const frame = await read("../../app/components/ArchivePageFrame.vue");
const sheet = await read("../../app/assets/css/document.css");
// The group of actions is bmo-ui's, read from the package the bundle stands on.
const group = await Bun.file(Bun.resolveSync("bmo-ui/app/components/ActionGroup.vue", import.meta.dir)).text();

function templateOf(source: string): string {
  return source.slice(0, source.indexOf("<script"));
}

// The one element a mark stands in, with its attributes and nothing of what it holds.
function tagOf(template: string, mark: string): string {
  const at = template.indexOf(mark);

  expect(at).toBeGreaterThan(-1);

  const opened = template.lastIndexOf("<", at);

  return template.slice(opened, template.indexOf(">", opened) + 1);
}

const bodyTemplate = templateOf(body);
const groupTemplate = templateOf(group);

describe("there is a template to read, so nothing below passes on an empty string", () => {
  test("both files carry one", () => {
    expect(bodyTemplate.length).toBeGreaterThan(500);
    expect(groupTemplate.length).toBeGreaterThan(500);
  });
});

describe("a document read by somebody who is not the owner", () => {
  test("the state chip stands only where a state was offered", () => {
    expect(tagOf(bodyTemplate, ":label=\"props.status.label\"")).toContain('v-if="props.status !== null"');
  });

  test("the row holding the chips and the actions stands only where one of them does", () => {
    expect(bodyTemplate).toContain('v-if="props.status !== null || props.shared !== null || actions.length > 0 || props.properties !== null"');
  });

  // The kind is the document's own and a share page shows it too, over the title; it stands only where
  // the caller named one.
  test("the kind over the title stands only where a kind was given", () => {
    expect(tagOf(bodyTemplate, "{{ props.kind }}")).toContain('v-if="props.kind !== null"');
  });

  // The files a document keeps are part of the document, on the console and on a share page alike,
  // each through its own road; with none, or no road to them, no section stands. A picture the body
  // already draws by its name is not one of them.
  test("the attachment section stands only where there are attachments and a road to them", () => {
    expect(bodyTemplate).toContain('<ArchiveAttachments v-if="split.others.length > 0 && props.attachmentBase !== null" :attachments="split.others"');
    expect(body).toContain("undrawnAttachments(props.attachments, props.findings.map((finding) => finding.text))");
  });

  // A sound the document keeps is its listening band under the masthead, on a share page as well, and
  // it is not among the files after the body.
  test("the sounds stand under the masthead only where there are some and a road to them", () => {
    expect(bodyTemplate).toContain('<section v-if="split.sounds.length > 0 && props.attachmentBase !== null"');
    expect(bodyTemplate.indexOf("split.sounds")).toBeGreaterThan(bodyTemplate.indexOf("</header>"));
    expect(bodyTemplate.indexOf("split.sounds")).toBeLessThan(bodyTemplate.indexOf('v-for="finding in props.findings"'));
    expect(body).toContain("soundsFirst(standing.value)");
  });

  // Being out on a link is the owner's own business. Somebody holding the link is told nothing about
  // it — the caller offers none, which is also what the share page's reading carries.
  test("the chip saying the document is shared is the caller's to offer too", () => {
    expect(bodyTemplate).toContain('v-if="props.shared !== null"');
  });

  test("the three the share page drops are each the caller's to offer, and default to none", () => {
    for (const none of ["status: null", "shared: null", "properties: null", "documentUrl: null", "sources: null", "share: null"]) {
      expect(body).toMatch(new RegExp(`\\n  ${none},?\\n`));
    }
  });

  // A search that answered one value keeps no document, and the screen says that rather than showing
  // it as a search that found nothing.
  test("what stands where there are no findings is the caller's to word", () => {
    expect(bodyTemplate).toContain('<p v-if="props.findings.length === 0" class="text-sm text-muted">{{ props.empty ?? $t("archive.status.empty") }}</p>');
  });

  test("the document's own file is an action only where one was offered", () => {
    expect(body).toContain("props.documentUrl === null ? [] :");
  });

  // Making a link is the owner's, so it is an action the screen offers rather than a part of the
  // document: a share page hands none and the control is simply not there.
  test("making a share link is an action only where one was offered", () => {
    expect(body).toContain("props.share === null ? [] :");
    expect(body).toContain('label: t("common.action.share")');
  });

  test("the title and the findings come as themselves, not as a whole document read off the console", () => {
    expect(bodyTemplate).toContain("{{ props.title }}");
    expect(bodyTemplate).toContain('v-for="finding in props.findings"');
    expect(bodyTemplate).not.toContain("props.detail");
    expect(body).not.toContain("SearchDetail");
  });

  // What a finding is called, where it folds and the markdown it stands as are the read API's to say,
  // and the bundle draws it with its own text component.
  test("each finding is named, folded and drawn as the read API answers it", () => {
    expect(bodyTemplate).toContain(':aria-label="finding.title"');
    expect(bodyTemplate).toContain('<FindingText :markdown="finding.markdown"');
    expect(bodyTemplate).toContain('v-if="finding.foldedMarkdown !== null"');
    expect(bodyTemplate).toContain('<FindingText :markdown="finding.foldedMarkdown ?? \'\'"');
    expect(bodyTemplate).not.toContain("MarkdownText");
  });
});

// The masthead's title as the stylesheet draws it. Where a line breaks is the browser's, so what is
// read here is the rule each title is drawn by.
describe("the masthead's title", () => {
  // The block a selector opens, from its brace to the brace that closes it.
  function ruleOf(selector: string): string {
    const at = sheet.indexOf(`\n${selector} {`);

    expect(at).toBeGreaterThan(-1);

    return sheet.slice(at, sheet.indexOf("}", at) + 1);
  }

  // A word in a script written without spaces between its syllables is never split across two lines;
  // one long word with no break in it, an address or an identifier, still breaks inside itself rather
  // than running past a phone's width. The lines run the whole reading width, with no word left alone
  // on the last.
  test("breaks only between words and fills the reading width", () => {
    const title = ruleOf(".document-title");

    expect(title).toContain("word-break: keep-all;");
    expect(title).toContain("overflow-wrap: anywhere;");
    expect(title).toContain("text-wrap: pretty;");
    expect(title).not.toContain("balance");
  });

  // A page document is drawn here only while its page is still being written; once the page is there
  // its title stands in the page frame's head, so until then it stands at that same step.
  test("a page still being written takes the step of the page frame's title", () => {
    expect(tagOf(bodyTemplate, "{{ props.title }}")).toContain(":class=\"{ 'document-title--page': props.hosted }\"");
    expect(body).toMatch(/\n {2}hosted: false,?\n/);
    expect(tagOf(templateOf(frame), "{{ props.title }}")).toContain("text-xl");
    expect(ruleOf(".document-title--page")).toContain("font-size: 1.25rem;");
    expect(sheet.indexOf("\n.document-title--page {")).toBeGreaterThan(sheet.indexOf("\n  .document-title {"));
  });
});

describe("a group of actions with nothing in it", () => {
  test("stands nowhere at all, rather than as an empty box holding a gap open", () => {
    // The condition holds a `>` of its own, so this one is read as the whole opening rather than
    // through the tag reader, which would cut it at that character.
    expect(groupTemplate).toContain('<div v-if="count > 0" ref="root"');
    expect(group).toContain("const count = computed(() => props.actions.length + (props.properties === null ? 0 : 1));");
  });

  test("the properties button says what it is on hover, as every icon button here does", () => {
    const at = groupTemplate.indexOf('<UTooltip v-if="props.properties !== null"');

    expect(at).toBeGreaterThan(-1);
    expect(groupTemplate.slice(at, groupTemplate.indexOf(">", at) + 1)).toContain(`:text="$t('common.label.properties')"`);
  });
});
