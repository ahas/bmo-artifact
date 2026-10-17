import { describe, expect, test } from "bun:test";

// The one share link dialog the Archive stands its links in: which of its two states stands when,
// what each offers, and the rules a rendered screen would show as nothing at all — that it is the
// shared centred dialog rather than a sheet, that every icon button says what it is on hover, that the
// address it hands over is one somebody outside can open, and that the body is not emptied on the way
// out. The screen hands it the road its own links go by.

const read = async (path: string): Promise<string> => Bun.file(new URL(path, import.meta.url)).text();
const source = await read("../../app/components/ShareLinkDialog.vue");
const archiveRoad = await read("../../app/components/ArchiveShareDialog.vue");
const template = source.slice(0, source.indexOf("<script"));

// The one element a mark stands in, with its attributes and nothing of what it holds.
function tagOf(mark: string): string {
  const at = template.indexOf(mark);

  expect(at).toBeGreaterThan(-1);

  const opened = template.lastIndexOf("<", at);

  return template.slice(opened, template.indexOf(">", opened) + 1);
}

describe("there is a template to read, so nothing below passes on an empty string", () => {
  test("the file carries one", () => {
    expect(template.length).toBeGreaterThan(500);
  });
});

describe("where the dialog stands", () => {
  test("it is the shared dialog, which is centred at every width and answers the back gesture", () => {
    expect(template).toContain("<SettingsDialog");
    expect(template).not.toContain("UDrawer");
    expect(template).not.toContain("USlideover");
  });

  // The line under the title is the name of what is shared, where the screen behind does not say it,
  // never words about the dialog.
  test("it is titled by what it is, with no sentence explaining the screen", () => {
    expect(tagOf("<SettingsDialog")).toContain(`:title="$t('common.title.share_link')"`);
    expect(tagOf("<SettingsDialog")).toContain(`:description="subject"`);
    expect(tagOf("<SettingsDialog")).not.toContain(":description=\"$t(");
  });
});

describe("while the link is being read", () => {
  test("the body is a skeleton rather than an empty box", () => {
    expect(tagOf("<ContentSkeleton")).toContain('v-if="loading"');
  });
});

describe("with no link", () => {
  test("the expiry is a dropdown over the one list of spans, and nothing is typed", () => {
    expect(source).toContain("SHARE_SPANS.map");
    expect(tagOf("<USelect")).toContain('v-model="span"');
    expect(tagOf(`<UFormField v-else :label="$t('common.label.expiry')">`)).toContain("v-else");
  });

  // The two faces stand on the link alone, so the one made is the one shown where the form was, in the
  // dialog that is still standing. Nothing here closes the dialog.
  test("making one leaves the dialog standing and puts the link where the form was", () => {
    expect(tagOf('v-else-if="share !== null"')).toContain('v-else-if="share !== null"');
    expect(source).not.toContain("open.value = false");
    expect(source).not.toMatch(/open\.value\s*=/);
  });

  test("making one stands in the dialog's footer, where a dialog's action belongs", () => {
    expect(template).toContain('<template v-if="!loading" #footer>');
    expect(tagOf(`:label="$t('common.action.create_link')"`)).toContain('v-if="share === null"');
  });
});

describe("with a link", () => {
  test("the address is shown as a field nobody edits, in the face this console gives machine text", () => {
    expect(tagOf("<UInput")).toContain("readonly");
    expect(tagOf("<UInput")).toContain(':model-value="address"');
    expect(tagOf("<UInput")).toContain("font-mono");
  });

  test("the address is absolute, so it opens from outside this console", () => {
    expect(source).toContain("window.location.origin");
    expect(source).toContain("appUrl(props.road.page(share.value.token))");
    expect(archiveRoad).toContain("page: (token) => `/share/${token}`");
  });

  // The archive's road is the share API's, under the document's own number.
  test("the archive's road reads, makes and takes away a link through the share API", () => {
    expect(archiveRoad).toContain("const at = `/api/archive/${props.searchId}/share`;");
    expect(archiveRoad).toContain('{ method: "POST", body: { span } }');
    expect(archiveRoad).toContain('{ method: "DELETE" }');
  });

  // Copying is the field's own affordance and stands on it; taking the link away acts on the link
  // itself, so it stands in the footer as the dialog's action, in the colour this console gives an
  // action that removes something.
  test("copying is an icon button named on hover, and there is no menu anywhere", () => {
    expect(template).toContain(`<UTooltip :text="$t('common.action.copy')">`);
    expect(template).not.toContain("UDropdownMenu");
    expect(template).not.toContain("more-vert");
  });

  test("taking the link away is the footer's action, and says it removes something", () => {
    expect(tagOf(`:label="$t('common.action.unshare')"`)).toContain('color="error"');
    expect(tagOf(`:label="$t('common.action.unshare')"`)).toContain("v-else");
  });

  test("the time left carries tabular figures, since it ticks while the dialog stands", () => {
    expect(tagOf(':label="left"')).toContain("tabular-nums");
  });

  test("a copy says whether it worked, over the dialog rather than inside it", () => {
    expect(source).toContain("island.say");
    expect(source).toContain("common.message.link_copied");
    expect(source).toContain("common.error.link_copy_failed");
  });

  test("what is left of the link is a chip in words, and the exact moment stands beside it", () => {
    expect(tagOf("<UBadge")).toContain(':label="left"');
    expect(source).toContain("shareLeftText(t, share.value.expiresAt, now.value)");
    expect(template).toContain("formatStamp(share.expiresAt)");
  });

  // A link with no end has no moment to count down to and none to name, so both places say what it is
  // instead — the chip through the shared reading, the line beside it in as many words.
  test("a link that never ends says so in both places rather than standing them empty", () => {
    expect(source).toContain("shareLeftText");
    expect(template).toContain('share.expiresAt === null ? $t("common.label.no_expiry")');
  });

  test("an ended link is only one that had a time to end at", () => {
    expect(source).toContain("share.value.expiresAt !== null && share.value.expiresAt <= now.value");
  });

  test("the countdown moves while the dialog stands, so the line does not go stale", () => {
    expect(source).toContain("useNowSeconds()");
    // The figure is put forward the moment a link is made, so the first one drawn is its whole span
    // rather than one up to half a minute short of it.
    expect(source).toContain("sync();");
  });
});

describe("what it tells the screen it stands on", () => {
  // The dialog is the one place a link is made and taken away, so the document's own screen is told
  // from here rather than asking the server again once this has closed.
  test("every one of the three says what the document is now shared by", () => {
    expect(source).toContain('emit("shared"');
    expect(source.match(/emit\("shared"/g)?.length).toBe(3);
    expect(source).toContain('emit("shared", null)');
  });
});

describe("on the way out", () => {
  test("what stands is read afresh on opening and left standing as it closes", () => {
    expect(source).toContain("void read();");
    // Nothing clears the link as the dialog closes; a body emptied on the way out blinks.
    expect(source).not.toContain("share.value = null;\n    return;");
  });
});
