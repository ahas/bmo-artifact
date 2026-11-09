import { describe, expect, test } from "bun:test";

import { MOST_PICTURES } from "../../shared/document-pictures";
import { keptPictures } from "../../app/domain/document-pictures";

import type { ArtifactAttachment } from "../../shared/types";

// The run the picture viewer walks from a picture pressed among a document's files (`docs/artifact.md`,
// "Pictures open whole" and "The files after the body").

function attachment(id: number, name: string, kind: string): ArtifactAttachment {
  return { id, documentId: 1, name, mediaType: "application/octet-stream", kind, byteCount: 1, role: null, createdAt: 0 };
}

const address = (one: ArtifactAttachment): string => `/api/artifact/documents/1/attachments/${one.id}`;
const BASE = "https://console.test/artifact/1";

describe("a picture pressed among a document's files", () => {
  test("opens on itself among every picture the files hold, in the order they were kept, each named and found where it is served", () => {
    const files = [attachment(1, "plan.png", "image"), attachment(2, "notes.txt", "text"), attachment(3, "site.avif", "image")];
    const run = keptPictures(files, files[2]!, address, BASE);

    expect(run).toEqual({
      at: 1,
      pictures: [
        { src: "https://console.test/api/artifact/documents/1/attachments/1", caption: "plan.png", address: "https://console.test/api/artifact/documents/1/attachments/1" },
        { src: "https://console.test/api/artifact/documents/1/attachments/3", caption: "site.avif", address: "https://console.test/api/artifact/documents/1/attachments/3" }
      ]
    });
  });

  test("opens nothing for a file that is no picture among them", () => {
    const files = [attachment(1, "plan.png", "image"), attachment(2, "notes.txt", "text")];

    expect(keptPictures(files, files[1]!, address, BASE)).toBeNull();
    expect(keptPictures(files, attachment(9, "elsewhere.png", "image"), address, BASE)).toBeNull();
  });

  test("is cut to the stretch around the pressed picture where the files hold more than one run carries", () => {
    const files = Array.from({ length: MOST_PICTURES + 10 }, (_, at) => attachment(at + 1, `picture-${at + 1}.png`, "image"));
    const run = keptPictures(files, files.at(-1)!, address, BASE);

    expect(run?.pictures).toHaveLength(MOST_PICTURES);
    expect(run?.pictures[run.at]?.caption).toBe(`picture-${MOST_PICTURES + 10}.png`);
  });
});
