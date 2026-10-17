import { describe, expect, test } from "bun:test";

import { MOST_CAPTION_CHARS, MOST_PICTURES, PICTURES_MESSAGE, receivedPictures } from "../../shared/document-pictures";

// What a page in the console's frame may make the console's picture viewer open: a run of pictures in
// the one shape it is asked for, and nothing else.
describe("a page's message for the picture viewer", () => {
  const picture = { src: "https://example.com/a.png", caption: " a figure " };

  test("opens the run it names on the pressed picture, its captions trimmed", () => {
    expect(receivedPictures({ type: PICTURES_MESSAGE, at: 1, pictures: [picture, { src: "data:image/png;base64,AA==" }] })).toEqual({
      at: 1,
      pictures: [
        { src: "https://example.com/a.png", caption: "a figure", address: "https://example.com/a.png" },
        { src: "data:image/png;base64,AA==", caption: "", address: null }
      ]
    });
    expect(receivedPictures({ type: PICTURES_MESSAGE, at: 0, pictures: [{ src: picture.src, caption: "x".repeat(MOST_CAPTION_CHARS + 5) }] })?.pictures[0]?.caption).toHaveLength(MOST_CAPTION_CHARS);
  });

  test("is nothing in any other shape", () => {
    for (const data of [
      null,
      "bmo-pictures",
      { type: "other", at: 0, pictures: [picture] },
      { type: PICTURES_MESSAGE, at: 1, pictures: [picture] },
      { type: PICTURES_MESSAGE, at: 0.5, pictures: [picture, picture] },
      { type: PICTURES_MESSAGE, at: 0, pictures: [] },
      { type: PICTURES_MESSAGE, at: 0, pictures: Array.from({ length: MOST_PICTURES + 1 }, () => picture) },
      { type: PICTURES_MESSAGE, at: 0, pictures: [{ caption: "no address" }] }
    ]) {
      expect(receivedPictures(data)).toBeNull();
    }
  });

  test("names no picture the viewer may not load", () => {
    for (const src of ["javascript:alert(1)", "blob:null/1", "data:text/html,<b>x</b>", "/api/archive/1"]) {
      expect(receivedPictures({ type: PICTURES_MESSAGE, at: 0, pictures: [{ src }] })).toBeNull();
    }
  });
});
