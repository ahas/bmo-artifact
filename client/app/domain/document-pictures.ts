import { countText } from "bmo-ui/app/domain/numbers";

import { MOST_PICTURES, VIEWER_ADDRESS, VIEWER_PICTURES, pictureAddress } from "../../shared/document-pictures";

import type { BarProperty } from "bmo-ui/app/domain/tabs";
import type { Size } from "bmo-ui/app/domain/zoom";
import type { Translate } from "bmo-ui/shared/wording";
import type { DocumentPicture, PictureRun } from "../../shared/document-pictures";

// A document's pictures as the viewer walks them (`docs/archive.md`, "Pictures open whole"): every
// picture the document draws on the screen that the shared rule lets open, in the order they stand.

// One picture as the viewer shows it, or null where it is not drawn or stands at an address the viewer
// cannot load. Its caption is its own alternative text, else the caption of the figure it stands in;
// its address is where it was found, where the document says so, else where it is shown from.
export function viewerPicture(image: HTMLImageElement): DocumentPicture | null {
  const src = image.currentSrc || image.src;

  if (!VIEWER_ADDRESS.test(src) || image.getClientRects().length === 0) return null;

  const caption = image.alt.trim() || (image.closest("figure")?.querySelector("figcaption")?.textContent ?? "").trim();

  return { src, caption, address: image.dataset.pictureAddress || pictureAddress(src) };
}

// The run the viewer opens on when `target` is pressed inside `root`, or null where the press was not
// on a picture that opens it. A document with more pictures than one run carries is cut to the stretch
// around the pressed one.
export function pressedPictures(root: ParentNode, target: EventTarget | null): PictureRun | null {
  if (!(target instanceof HTMLImageElement) || !target.matches(VIEWER_PICTURES)) return null;

  const pictures: DocumentPicture[] = [];
  let at = -1;

  for (const image of root.querySelectorAll<HTMLImageElement>(VIEWER_PICTURES)) {
    const picture = viewerPicture(image);

    if (picture === null) continue;
    if (image === target) at = pictures.length;

    pictures.push(picture);
  }

  if (at < 0) return null;

  const from = Math.max(0, Math.min(at - MOST_PICTURES / 2, pictures.length - MOST_PICTURES));

  return { pictures: pictures.slice(from, from + MOST_PICTURES), at: at - from };
}

// Whether a press is the plain one that opens a picture: the main button with no key held, which the
// document has not already taken for itself.
export function plainPress(event: MouseEvent): boolean {
  return !event.defaultPrevented && event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
}

// The picture's details as the properties list stands them: its caption, the size the browser read off
// it once it has loaded, and its address where it has one rather than being the picture's own data.
export function documentPictureProperties(t: Translate, picture: DocumentPicture, drawn: Size | null): BarProperty[] {
  return [
    ...(picture.caption === "" ? [] : [{ label: t("common.label.picture_caption"), value: picture.caption }]),
    ...(drawn === null || drawn.width === 0
      ? []
      : [{ label: t("common.label.picture_resolution"), value: t("common.label.picture_resolution_value", { width: countText(drawn.width), height: countText(drawn.height) }) }]),
    ...(picture.address === null ? [] : [{ label: t("archive.label.picture_address"), value: picture.address, mono: true }])
  ];
}
