import { onBeforeUnmount, onMounted } from "vue";

import type { Ref } from "vue";

// The room a column held in view has (docs/artifact.md, 「The front page」): the visible height of the region that
// scrolls it, less that region's padding above and below, since a sticky column holds inside that padding. It is set
// on the column as `--side-room` and read again whenever the region changes its size, so a column sticky in it never
// stands taller than what the reader sees.

/** The nearest ancestor of an element that scrolls up and down; null where only the window does. */
export function scrollingAncestor(element: HTMLElement): HTMLElement | null {
  for (let at = element.parentElement; at !== null; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;

    if (overflow === "auto" || overflow === "scroll" || overflow === "overlay") return at;
  }

  return null;
}

/** The height a column held in a scrolling region may take, in pixels; the window's where no region scrolls it. */
export function sideRoom(scroller: HTMLElement | null): number {
  if (scroller === null) return window.innerHeight;

  const style = getComputedStyle(scroller);
  const top = parseFloat(style.paddingTop) || 0;
  const bottom = parseFloat(style.paddingBottom) || 0;

  return Math.max(0, scroller.clientHeight - top - bottom);
}

/** Keeps `--side-room` on the column as its scrolling region's room, for as long as it stands. */
export function useSideRoom(column: Readonly<Ref<HTMLElement | null>>): void {
  let observer: ResizeObserver | null = null;
  let scroller: HTMLElement | null = null;

  function measure(): void {
    const element = column.value;

    if (element === null) return;

    element.style.setProperty("--side-room", `${sideRoom(scroller)}px`);
  }

  function onWindowResize(): void {
    measure();
  }

  onMounted(() => {
    const element = column.value;

    if (element === null) return;

    scroller = scrollingAncestor(element);
    measure();
    if (scroller !== null) {
      observer = new ResizeObserver(measure);
      observer.observe(scroller);
    } else {
      window.addEventListener("resize", onWindowResize);
    }
  });

  onBeforeUnmount(() => {
    observer?.disconnect();
    window.removeEventListener("resize", onWindowResize);
  });
}
