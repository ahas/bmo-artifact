import { viewerPicture } from "../../app/domain/document-pictures";
import { drawKeptPictures } from "../../app/domain/drawn-pictures";

// The page of `drawn-pictures.test.ts`: its pictures drawn from their copies as a markdown body's are, and the
// address the viewer opens one at, for the test to read.

drawKeptPictures(document);

(window as unknown as { viewerOf: (image: HTMLImageElement) => string | null }).viewerOf = (image) => viewerPicture(image)?.src ?? null;
