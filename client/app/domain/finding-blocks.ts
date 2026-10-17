import type { GallerySlide } from "./finding-markdown";

// The places the finding reading leaves in a text for the blocks markdown has no form for (see
// finding-markdown): a fold, a gallery and a map. The reading writes what each needs onto its place,
// and this reads it back off the page, where the component that fills the place is chosen.

export type FindingBlock =
  | { kind: "fold"; label: string; html: string }
  | { kind: "gallery"; slides: GallerySlide[] }
  | { kind: "map"; query: string };

export const FINDING_BLOCK_PLACES = ".finding-fold, .finding-gallery, .finding-map";

function slidesOf(value: string | undefined): GallerySlide[] {
  try {
    const slides: unknown = JSON.parse(value ?? "[]");

    return Array.isArray(slides)
      ? slides.filter((slide): slide is GallerySlide => typeof slide === "object" && slide !== null && typeof slide.href === "string" && typeof slide.caption === "string" && (slide.src === null || typeof slide.src === "string"))
      : [];
  } catch {
    return [];
  }
}

// What one place asks for, or null where it asks for nothing that can be drawn: a gallery with no
// slide, a map with no place.
export function findingBlock(place: HTMLElement): FindingBlock | null {
  if (place.classList.contains("finding-fold")) {
    return { kind: "fold", label: place.dataset.foldLabel ?? "", html: place.querySelector(".finding-fold-body")?.innerHTML ?? "" };
  }

  if (place.classList.contains("finding-gallery")) {
    const slides = slidesOf(place.dataset.slides);

    return slides.length === 0 ? null : { kind: "gallery", slides };
  }

  const query = place.dataset.mapQuery?.trim() ?? "";

  return query === "" ? null : { kind: "map", query };
}
