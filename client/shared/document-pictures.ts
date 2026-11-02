import design from "../design/document.json";

// Which pictures of a document open the console's picture viewer, and the one message a page standing
// in the console's frame asks for it by (`docs/artifact.md`, "Pictures open whole"). A markdown document
// is read by the console itself; a page is read by the bridge the server writes ahead of it, since its
// frame gives it an origin of its own that the console cannot reach into. Both read the rule from the
// design file, so a picture opens the viewer or keeps to the document on either.

// A picture opens the viewer unless the document gives its press a meaning of its own.
export const VIEWER_PICTURES: string = design.viewer.pictures;

// A picture the viewer can show is one at an address the console itself may load: over the web, or
// written into the document as a picture's data. A page's own `blob:` addresses are its origin's alone.
export const VIEWER_ADDRESS = new RegExp(design.viewer.address, design.viewer.address_flags);

// What a page sends its frame, and the most pictures one message may carry.
export const PICTURES_MESSAGE: string = design.viewer.message;
export const MOST_PICTURES: number = design.viewer.most_pictures;
export const MOST_CAPTION_CHARS: number = design.viewer.most_caption_chars;

export interface DocumentPicture {
  src: string;
  caption: string;
  // Where the picture is from, and null for one written into the document as data.
  address: string | null;
}

// A picture's own address, where it has one: a picture written as data has none.
export function pictureAddress(src: string): string | null {
  return src.startsWith("data:") ? null : src;
}

// The pictures a document hands the viewer, and the one pressed among them.
export interface PictureRun {
  pictures: DocumentPicture[];
  at: number;
}

// What a page posted, as the console takes it: a message of the shape above, every picture at an
// address the viewer may show, and the pressed one among them. Anything else is nothing, since the
// page is not trusted to have sent what it should.
export function receivedPictures(data: unknown): PictureRun | null {
  if (typeof data !== "object" || data === null) return null;

  const { type, at, pictures } = data as { type?: unknown; at?: unknown; pictures?: unknown };

  if (type !== PICTURES_MESSAGE || !Array.isArray(pictures) || pictures.length === 0 || pictures.length > MOST_PICTURES) return null;
  if (typeof at !== "number" || !Number.isInteger(at) || at < 0 || at >= pictures.length) return null;

  const read: DocumentPicture[] = [];

  for (const picture of pictures as unknown[]) {
    const { src, caption } = (typeof picture === "object" && picture !== null ? picture : {}) as { src?: unknown; caption?: unknown };

    if (typeof src !== "string" || !VIEWER_ADDRESS.test(src)) return null;

    read.push({ src, caption: typeof caption === "string" ? caption.trim().slice(0, MOST_CAPTION_CHARS) : "", address: pictureAddress(src) });
  }

  return { pictures: read, at };
}
