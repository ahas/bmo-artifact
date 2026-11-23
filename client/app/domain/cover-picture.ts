// A document's cover drawn small (`docs/artifact.md`, "A picture drawn small"): a cover fills a box of its own
// shape, cropped to it, so it is asked at the width that covers that box on the screen's pixels: the box's width,
// or its height times the picture's own shape where the picture is wider than the box. Before the picture has
// loaded its shape is unknown, so it is asked to cover the box's longer side; once loaded, a picture wider than
// that is asked again at the width that covers it.

import { DRAWN_STEPS, drawnAddress } from "./drawn-pictures";

/** The box a cover fills, in CSS pixels. */
export interface CoverBox {
  width: number;
  height: number;
}

/** The ladder's first step at or above `width`, and its widest step past the last. */
export function coverStep(width: number): number {
  return DRAWN_STEPS.find((step) => step >= width) ?? DRAWN_STEPS[DRAWN_STEPS.length - 1]!;
}

/**
 * The width a cover is asked at to fill `box` on a screen of `density`: where its shape (width over height) is
 * known, the width that covers the box at that shape, else the box's longer side.
 */
export function coverWidth(box: CoverBox, density: number, shape: number | null): number {
  const across = shape === null ? Math.max(box.width, box.height) : Math.max(box.width, box.height * shape);

  return coverStep(Math.ceil(across * density));
}

/** A cover's address asked at the width that fills `box`. */
export function coverAddress(address: string, box: CoverBox, density: number, shape: number | null): string {
  return drawnAddress(address, coverWidth(box, density, shape));
}

/**
 * The address a cover `<img>` loads, kept to the box it fills: none until the box is measured, so no whole
 * card is fetched first, then the copy filling it, asked again whenever the box grows or the picture's shape shows it needs a wider one.
 */
export function useCoverPicture(address: () => string, image: Ref<HTMLImageElement | null>) {
  const width = ref<number | null>(null);

  function box(): CoverBox | null {
    // The picture's own element stands at the box's size, set by its classes before anything loads.
    const rect = image.value?.getBoundingClientRect();

    return rect === undefined || rect.width === 0 ? null : { width: rect.width, height: rect.height };
  }

  // The picture's shape once it has loaded, none before.
  function shape(): number | null {
    const shown = image.value;

    return shown === null || shown.naturalHeight === 0 ? null : shown.naturalWidth / shown.naturalHeight;
  }

  // Asks the copy the box now needs where it needs a wider one than the copy asked; a box that shrinks keeps the
  // copy it holds rather than fetching a narrower one.
  function measure(): void {
    const measured = box();

    if (measured === null) return;

    const wanted = coverWidth(measured, window.devicePixelRatio || 1, shape());
    const shown = image.value;

    // A copy is never wider than the card, so a loaded picture narrower than the copy asked has no wider one.
    if (width.value !== null && shown !== null && shown.naturalWidth > 0 && shown.naturalWidth < width.value) return;
    if (width.value === null || wanted > width.value) width.value = wanted;
  }

  // The box grows as the page settles (a column laid out, a window widened), so it is measured whenever it moves.
  let watching: ResizeObserver | null = null;

  onMounted(() => {
    measure();

    if (typeof ResizeObserver !== "undefined" && image.value !== null) {
      watching = new ResizeObserver(measure);
      watching.observe(image.value);
    }
  });
  onBeforeUnmount(() => watching?.disconnect());
  watch(address, () => {
    width.value = null;
    nextTick(measure);
  });

  const src = computed(() => (width.value === null ? undefined : drawnAddress(address(), width.value)));

  return { src, loaded: measure };
}
