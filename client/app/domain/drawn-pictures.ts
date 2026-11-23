// A kept picture the body draws smaller than it is kept is drawn from a copy the service makes near that size
// (`docs/artifact.md`, "A picture drawn small"). The picture is marked as one of the document's kept files where the
// body is written; once it has loaded and its own width is known, a picture standing more than a quarter smaller than
// its file on the screen's pixels names the copies below its own width as the widths it can be drawn from, and the
// width it stands at, for the browser to choose the first covering it.

/** The attribute marking a picture the body draws from one of the document's kept files. */
export const DRAWN_COPIES = "data-drawn-copies";

// The narrowest and the widest copy, and how much wider each step may be than the one before: the service's ladder.
const FIRST_STEP = 16;
const LAST_STEP = 2048;
const STEP_GROWTH = 1.25;

/** The widths the service copies a picture at, narrowest first. */
export const DRAWN_STEPS: readonly number[] = (() => {
  const steps: number[] = [];

  for (let width = FIRST_STEP; width < LAST_STEP; width = Math.floor((width * 5) / 4)) steps.push(width);

  return [...steps, LAST_STEP];
})();

/** A kept picture's address asking for its copy at one width. */
export function drawnAddress(address: string, width: number): string {
  return `${address}${address.includes("?") ? "&" : "?"}width=${width}`;
}

/**
 * The candidate list and the width a picture of `own` pixels across is drawn from, standing `standing` CSS pixels
 * wide on a screen of `density`: none where its own file is within a step of what it covers.
 */
export function drawnChoice(address: string, own: number, standing: number, density: number): { srcset: string; sizes: string } | null {
  if (standing <= 0 || own <= standing * density * STEP_GROWTH) return null;

  // Its own file is no candidate: a browser keeps drawing a candidate it already holds at a higher density than it
  // needs. One of the steps below its own width covers what it stands on, since it is more than a step wider.
  const below = DRAWN_STEPS.filter((step) => step < own).map((step) => `${drawnAddress(address, step)} ${step}w`);

  return { srcset: below.join(", "), sizes: `${standing}px` };
}

// One picture, once it has loaded at its own width.
function drawFromCopies(image: HTMLImageElement): void {
  if (image.srcset !== "" || image.naturalWidth === 0) return;

  const choice = drawnChoice(image.getAttribute("src") ?? "", image.naturalWidth, image.getBoundingClientRect().width, window.devicePixelRatio || 1);

  if (choice === null) return;

  image.sizes = choice.sizes;
  image.srcset = choice.srcset;
}

/** Every kept picture under `root` drawn from its copies once it has loaded. */
export function drawKeptPictures(root: ParentNode): void {
  for (const image of root.querySelectorAll<HTMLImageElement>(`img[${DRAWN_COPIES}]`)) {
    if (image.complete && image.naturalWidth > 0) drawFromCopies(image);
    else image.addEventListener("load", () => drawFromCopies(image), { once: true });
  }
}
