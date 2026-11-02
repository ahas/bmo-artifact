import { describe, expect, test } from "bun:test";

import { findingHtml } from "../../app/domain/finding-markdown";

import type { Translate } from "bmo-ui/shared/wording";

// What a finding's markdown becomes. Each case starts from markdown in the shape the read API answers
// it, its structure lines already turned into words by the server's reading, whose own tests hold that
// turning. The reading is checked here directly, since marked needs no document; the cleaning is
// DOMPurify's, which needs a real one, so it is checked in a real browser
// (`tests/browser/finding-markdown.test.ts`). Where a case is about how Hangul is read, each Hangul
// syllable is named by its code point, so no source line holds one.

// The words of the language pack are the host's, so the reading is checked by the keys it asks for.
const t: Translate = (key) => key;

// The particles and units the cases write straight after a figure, an address or an emphasis.
const [IN, RO, JO, EOK, E, GWA, MAN] = [0xc778, 0xb85c, 0xc870, 0xc5b5, 0xc5d0, 0xacfc, 0xb9cc].map((point) => String.fromCodePoint(point));

const pictures = new Map([
  ["https://cdn.example.com/p.jpg", "/base/api/archive/4/refs/i1.1/image"],
  ["https://cdn.example.com/back.png", "/base/api/archive/4/refs/i1.2/image"]
]);
const images = (href: string): string | null => pictures.get(href) ?? null;

describe("a finding, as the read API answers it", () => {
  test("shows the HTML typed into it as the characters it was typed as", () => {
    const html = findingHtml(t, '<script>alert(1)</script>\n<img src=x onerror="alert(1)"> Array<string>', { images });

    expect(html).not.toContain("<script");
    expect(html).not.toContain("<img src=x");
    expect(html).toContain("&lt;script&gt;alert(1)&lt;/script&gt;");
    expect(html).toContain("Array&lt;string&gt;");
  });

  test("breaks a line where a line was broken", () => {
    expect(findingHtml(t, "first line\nsecond line")).toContain("first line<br>second line");
  });

  test("keeps its tildes as the ranges they are, and never strikes", () => {
    const html = findingHtml(t, "the main range (630~720) is 1.31~1.49 times, ~~two~~ as letters");

    expect(html).not.toContain("<del");
    expect(html).toContain("(630~720) is 1.31~1.49 times");
    expect(html).toContain("~~two~~");
    expect(findingHtml(t, "~~two~~ 3~5")).not.toContain("<del");
  });

  test("colours a rise red and a fall blue, the arrow with its figure and nothing past them", () => {
    const html = findingHtml(t, "| Stock | Move |\n| --- | --- |\n| Alpha | ▲ 1,200 (+3.2%)[1](#mark-1) up |\n| Beta | ▼-4.90% |");

    expect(html).toContain('<span class="search-rise">▲ 1,200 (+3.2%)</span><sup class="search-mark">');
    expect(html).toContain('<span class="search-fall">▼-4.90%</span>');
    expect(html).toContain("</sup> up");
  });

  test("ends a rise or a fall at its figure, before the particle or the comma the sentence goes on with", () => {
    const html = findingHtml(t, `index ▼2.20%${IN} day, ▲0.29%${RO}, net ▲1${JO}3,450${EOK}(11:36), outlook ▲ holds, rate ▲0.118%p (intraday)`);

    expect(html).toContain(`<span class="search-fall">▼2.20%</span>${IN} day`);
    expect(html).toContain(`<span class="search-rise">▲0.29%</span>${RO},`);
    expect(html).toContain(`<span class="search-rise">▲1${JO}3,450${EOK}</span>(11:36)`);
    expect(html).toContain('outlook <span class="search-rise">▲</span> holds');
    expect(html).toContain('<span class="search-rise">▲0.118%p</span> (intraday)');
  });

  test("stands the headings, the divider, the labels and the links the server wrote", () => {
    const html = findingHtml(t, "\n### Choosing a mini PC\n\nOne line of body.\n\n***\n\n\n### GEM12\n\n[Map](<https://map.example.com/p/search/GEM12>)\n\n**Summary** Buy it at the store.");

    expect(html).toContain("<h3>Choosing a mini PC</h3>");
    expect(html).toContain("<h3>GEM12</h3>");
    expect(html).toContain("<hr>");
    expect(html).toContain('<a href="https://map.example.com/p/search/GEM12">Map</a>');
    expect(html).toContain("<strong>Summary</strong> Buy it at the store.");
  });

  test("stands a picture as the console's copy of it, with its caption", () => {
    const html = findingHtml(t, "before\n\n![the unit \\[front\\] photo](<https://cdn.example.com/p.jpg>)\n\nafter", { images });

    expect(html).toContain(
      '<span class="markdown-figure"><img src="/base/api/archive/4/refs/i1.1/image" alt="the unit [front] photo" loading="lazy" data-picture-address="https://cdn.example.com/p.jpg">' +
        '<span class="markdown-caption">the unit [front] photo</span></span>'
    );
    // The address it was found at is named for the picture viewer, and nothing fetches it.
    expect(html).not.toMatch(/(?:src|href)="https:\/\/cdn\.example\.com/);
  });

  test("stands a gallery as one place, naming the copy and the address of each slide", () => {
    const html = findingHtml(
      t,
      "\n### The shop\n\n\n::: gallery\n![soup 9,000](<https://cdn.example.com/p.jpg>)\n![the front](<https://cdn.example.com/gone.jpg>)\n:::\n\n\n> **4** tasty [review](<https://example.com/r>)\n",
      { images }
    );
    const slides = /data-slides="([^"]*)"/.exec(html)?.[1] ?? "";

    expect(html.match(/class="finding-gallery"/g)).toHaveLength(1);
    expect(JSON.parse(slides.replaceAll("&quot;", '"'))).toEqual([
      { src: "/base/api/archive/4/refs/i1.1/image", href: "https://cdn.example.com/p.jpg", caption: "soup 9,000" },
      { src: null, href: "https://cdn.example.com/gone.jpg", caption: "the front" }
    ]);
    expect(html).toContain("<blockquote>");
  });

  test("stands a picture alone as a picture, not a gallery", () => {
    expect(findingHtml(t, "\n![one](<https://cdn.example.com/p.jpg>)\n\nbody", { images })).not.toContain("finding-gallery");
  });

  test("folds a fold's body under its label", () => {
    const html = findingHtml(
      t,
      "\n### The shop\n\n\n::: fold Menu\nmenu photo [2](#mark-2)\n\n| Dish | Price | Notes |\n| --- | --- | --- |\n| **Meals** | | |\n| Soup | 9,000 | |\n| Rice | 11,000 | lunch only |\n:::\n"
    );
    const fold = /<div class="finding-fold" data-fold-label="Menu"><div class="finding-fold-body">([\s\S]*?)<\/div><\/div>/.exec(html)?.[1] ?? "";

    expect(fold).toContain("menu photo");
    expect(fold).toContain('href="#mark-2"');
    expect(fold).toMatch(/<th[^>]*>Dish<\/th>\s*<th[^>]*>Price<\/th>\s*<th[^>]*>Notes<\/th>/);
    expect(fold).toContain("<strong>Meals</strong>");
    expect(fold).toMatch(/<td[^>]*>Soup<\/td>\s*<td[^>]*>9,000<\/td>/);
    expect(fold).toMatch(/<td[^>]*>lunch only<\/td>/);
  });

  test("leaves a map's place for the map, above its link", () => {
    const html = findingHtml(t, "\n### The shop\n\n**Address** 1 Main Street [3](#mark-3)\n\n::: map 1 Main Street\n\n[Map](<https://map.example.com/p/entry/place/1>)");

    expect(html).toContain('<div class="finding-map" data-map-query="1 Main Street"></div>');
    expect(html).toContain('<a href="https://map.example.com/p/entry/place/1">Map</a>');
  });

  // The server escapes a line the search itself wrote in a block's form, so it opens no block.
  test("keeps an escaped block line as its own words", () => {
    const html = findingHtml(t, "\\::: map somewhere\n\\::: fold fake\nbody\n\\:::");

    expect(html).not.toContain("finding-map");
    expect(html).not.toContain("finding-fold");
    expect(html).toContain("::: map somewhere");
  });

  test("shows a markdown image the record holds, and links one it does not", () => {
    const html = findingHtml(t, "![rear](https://cdn.example.com/back.png) ![outside](https://tracker.example.net/pixel.gif)", { images });

    expect(html).toContain('<img src="/base/api/archive/4/refs/i1.2/image" alt="rear" loading="lazy" data-picture-address="https://cdn.example.com/back.png">');
    expect(html).toContain('<a href="https://tracker.example.net/pixel.gif">outside</a>');
    expect(html).not.toContain('src="https://tracker');
  });

  // A picture or a link may name one of the document's own files by its name; it is drawn from, or
  // goes to, where that file is served, and a name the document does not keep stays as written.
  test("draws a picture and links a page the document keeps by the name it is kept under", () => {
    const files = (href: string): string | null => ({ "palette-01.webp": "/base/api/archive/4/attachments/8", "gallery.html": "/base/api/archive/4/attachments/7" })[href] ?? null;
    const html = findingHtml(t, "![Pearl](palette-01.webp)\n\n[**the page** opens](gallery.html) [the other file](other.html)", { images, files });

    expect(html).toContain('<img src="/base/api/archive/4/attachments/8" alt="Pearl" loading="lazy">');
    expect(html).toContain('<a href="/base/api/archive/4/attachments/7"><strong>the page</strong> opens</a>');
    expect(html).toContain('<a href="other.html">the other file</a>');
  });

  test("raises its footnote marks as links naming their mark, a snippet's apart", () => {
    const html = findingHtml(t, "1. 63 dollars[1](#mark-1), 11,890[2†](#mark-2%E2%80%A0)\n[3](https://x.example) as it is");

    expect(html).toContain('<sup class="search-mark"><a href="#mark-1" data-mark="1"><span class="search-mark-number">1</span></a></sup>');
    expect(html).toContain('class="search-mark search-mark-snippet"');
    expect(html).toContain('data-mark="2†"');
    expect(html).toContain('title="artifact.label.snippet_only"');
    expect(html).toContain('<a href="https://x.example">3</a>');
  });

  // A research's body cites its pages with bare marks and closes with a list of them, an entry a mark and a
  // link; the body is drawn as filed, so the marks are read where it is drawn.
  test("raises a bare mark its body carries a source entry for, and leads each entry's mark back to it", () => {
    const html = findingHtml(
      t,
      [
        "## Answer",
        "",
        "1. a weight[1] and two pages[5][6], a snippet's price[10†], an index a[7] as written",
        "",
        "| Part | Source |",
        "| --- | --- |",
        "| frame | [5][6] |",
        "",
        "## Sources",
        "",
        "- [1] [The **first** page](https://one.example/a)",
        "- [5] [Fifth](https://five.example)",
        "- [6] [Sixth](https://six.example)",
        "- [10†] [A snippet](https://ten.example)",
        "- [8] a note with no page"
      ].join("\n")
    );

    expect(html).toContain('a weight<sup class="search-mark"><a href="#mark-1" data-mark="1" id="cite-1"><span class="search-mark-number">[1]</span></a></sup> and');
    expect(html).toContain('<a href="#mark-5" data-mark="5" id="cite-5"><span class="search-mark-number">[5]</span></a></sup><sup class="search-mark"><a href="#mark-6" data-mark="6" id="cite-6"><span class="search-mark-number">[6]</span></a>');
    expect(html).toContain('<sup class="search-mark search-mark-snippet"><a href="#mark-10%E2%80%A0" data-mark="10†" id="cite-10†"><span class="search-mark-number">[10†]</span></a></sup>');
    // Only a mark's first place is where its entry leads back to.
    expect(html).toContain('<td><sup class="search-mark"><a href="#mark-5" data-mark="5"><span class="search-mark-number">[5]</span></a></sup>');
    expect(html.match(/id="cite-5"/g)).toHaveLength(1);
    // A snippet's mark with an entry says so in its popover, not in a title of its own.
    expect(html).not.toContain("title=");
    // A bracketed number the body carries no entry for stands as written, and so does an item with no page.
    expect(html).toContain("an index a[7] as written");
    expect(html).toContain("<li>[8] a note with no page</li>");
    expect(html).toContain(
      '<li id="mark-1" data-source="1"><a href="#cite-1" class="search-mark-back" data-mark-back="1"><span class="search-mark-number">[1]</span></a> <a href="https://one.example/a">The <strong>first</strong> page</a></li>'
    );
    expect(html).toContain('<li id="mark-10†" data-source="10†"><a href="#cite-10%E2%80%A0" class="search-mark-back" data-mark-back="10†"><span class="search-mark-number">[10†]</span></a>');
  });

  test("leaves bracketed numbers as written in a body with no source entries", () => {
    const html = findingHtml(t, "a list[1] and a range [2][3]\n\n- [4] just a note");

    expect(html).not.toContain("search-mark");
    expect(html).toContain("a list[1] and a range [2][3]");
    expect(html).toContain("<li>[4] just a note</li>");
  });

  // The owner asked for a table of contents that lands on the section it names, so a heading takes the
  // mark the contents steer by. The marks are the caller's, in the order the headings stand.
  test("gives its headings the marks the contents were built with", () => {
    const html = findingHtml(t, "\n### Rinkle\n\n\n#### Summary\n\n1. value\n\n#### Unconfirmed\n\n1. none", {
      slugs: ["3-rinkle", "3-summary", "3-unconfirmed"]
    });

    expect(html).toContain('<h3 id="3-rinkle" data-heading="3-rinkle">Rinkle</h3>');
    expect(html).toContain('<h4 id="3-summary" data-heading="3-summary">Summary</h4>');
    expect(html).toContain('<h4 id="3-unconfirmed" data-heading="3-unconfirmed">Unconfirmed</h4>');
  });

  // A finding drawn with no marks is drawn the same, only with nothing to steer to.
  test("carries no marks where it is given none", () => {
    const html = findingHtml(t, "\n#### Summary\n\n1. value");

    expect(html).toContain("<h4>Summary</h4>");
    expect(html).not.toContain("data-heading");
  });

  test("leaves what a fence holds as it was written", () => {
    expect(findingHtml(t, ["```", "# a comment", "SUMMARY: a value", "```"].join("\n"))).toContain("# a comment\nSUMMARY: a value");
  });

  test("links every picture when no source is given", () => {
    const html = findingHtml(t, "\n![a photo](<https://cdn.example.com/p.jpg>)\n");

    expect(html).not.toContain("<img");
    expect(html).toContain('<a href="https://cdn.example.com/p.jpg">a photo</a>');
  });

  // A bare address written straight into a Korean sentence ends where the sentence's letters begin.
  test("ends a bare address where the sentence takes over", () => {
    const html = findingHtml(t, `see https://example.com/page${E} there, and (https://example.com/a).`);

    expect(html).toContain(`<a href="https://example.com/page">https://example.com/page</a>${E} there`);
    expect(html).toContain('(<a href="https://example.com/a">https://example.com/a</a>).');
  });
});

describe("strong words in Korean", () => {
  test("close on a bracket that a particle follows", () => {
    const html = findingHtml(t, `only **「one picture」**${GWA} **「the math box」**${MAN} are read.`);

    expect(html).toContain(`<strong>「one picture」</strong>${GWA} <strong>「the math box」</strong>${MAN}`);
    expect(html).not.toContain("**");
  });

  test("leave a lone pair of stars and the ordinary forms as they were", () => {
    expect(findingHtml(t, "2 ** 3 is 8")).toContain("2 ** 3 is 8");
    expect(findingHtml(t, "**bold** and *italic*")).toContain("<strong>bold</strong> and <em>italic</em>");
  });
});

// A fenced block is what gets carried somewhere else — a command, a configuration — so it stands with
// a copy button beside it; a line of inline code is part of a sentence and carries none.
describe("a fenced block", () => {
  test("stands in a box of its own with a copy button after it", () => {
    const box = /<div[^>]*class="markdown-code"[^>]*>(<div class="markdown-code-head">.*?<\/div>)?<pre><code class="language-sh">bun install\n?<\/code><\/pre><button type="button" class="markdown-copy" data-copy aria-label="common\.action\.copy" data-tooltip="common\.action\.copy">/;

    expect(findingHtml(t, "a line before\n\n```sh\nbun install\n```\n")).toMatch(box);
  });

  test("names the language its fence gives at its head, and a fence with none carries no head", () => {
    const named = findingHtml(t, "```shell title=x\ncurl -s https://example.com\n```\n");
    const bare = findingHtml(t, "```\ncurl -s https://example.com\n```\n");

    expect(named).toMatch(/class="markdown-code" data-language="shell"><div class="markdown-code-head"><span class="markdown-code-language">shell<\/span><\/div><pre>/);
    expect(bare).not.toContain("data-language");
    expect(bare).not.toContain("markdown-code-head");
  });

  test("gives inline code no button", () => {
    expect(findingHtml(t, "`bun install` to run")).not.toContain("markdown-copy");
  });
});

describe("a table", () => {
  test("stands in a box of its own that knows how many columns the table has", () => {
    expect(findingHtml(t, "| Shop | Where | Price |\n|---|---|---|\n| one | 16 Road | 8,000 |")).toMatch(/^<div class="markdown-table" style="--columns: 3"><table>[\s\S]*<\/table>\s*<\/div>/);
  });
});

// marked hands a renderer a picture's words already escaped and every other token's words as they
// were typed, so each is escaped exactly once on its way out.
describe("the characters HTML reserves", () => {
  test("stand as they were typed in a heading, a picture, a caption and a link", () => {
    const typed = '### 01 · Pearl & Cyan "<b>"\n\n![Pearl & Cyan "<b>"](palette-01.webp)\n\n[Pearl & Cyan](https://example.com/) & <i>';
    const once = "Pearl &amp; Cyan &quot;&lt;b&gt;&quot;";
    const files = (href: string): string | null => (href === "palette-01.webp" ? "/base/api/archive/58/attachments/2" : null);
    const html = findingHtml(t, typed, { files, slugs: ["one"] });

    expect(html).toContain(">01 · Pearl &amp; Cyan &quot;&lt;b&gt;&quot;</h3>");
    expect(html).toContain(`alt="${once}"`);
    expect(html).toContain(`<span class="markdown-caption">${once}</span>`);
    expect(html).toContain(">Pearl &amp; Cyan</a> &amp; &lt;i&gt;");
    expect(html).not.toContain("&amp;amp;");
    expect(html).not.toContain("&amp;quot;");
  });
});

describe("a diagram block", () => {
  test("stands as a diagram's place holding its source, with no copy button", () => {
    const html = findingHtml(t, ["```mermaid", "flowchart TD", "  a[<b>asked</b>] --> b", "```"].join("\n"));

    expect(html).toContain('class="markdown-diagram" data-diagram><pre><code>flowchart TD\n  a[&lt;b&gt;asked&lt;/b&gt;] --&gt; b</code></pre></div>');
    expect(html).not.toContain("data-copy");
  });

  test("is named by the first word of the fence's language, in any case", () => {
    expect(findingHtml(t, "```Mermaid title\nflowchart TD\n```")).toContain("data-diagram");
    expect(findingHtml(t, "```mermaidish\nflowchart TD\n```")).not.toContain("data-diagram");
  });
});
