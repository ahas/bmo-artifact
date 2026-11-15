import { closeLayer, listenForBack, openLayer } from "bmo-ui/app/domain/layers";
import { scrollWithin } from "bmo-ui/app/domain/scroll-within";

// The jumps a press on a footnote mark makes in one text (`docs/artifact.md`, "How a document is drawn"): a click on a
// mark scrolls the text to its entry, and a press on an entry's mark scrolls it back to where the mark first stands.
// Each lands the place in the middle of the text, marks it there for a moment, and is a step the back gesture takes
// back: the text returns to where it stood before the jump, the address unchanged. A jump not taken back stays in the
// history as a wiki's does.

// How long a place a jump landed on stays marked.
const FOUND_MS = 2000;

/** The jumps of one text. */
export interface FootnoteJumps {
  /** Scrolls the text to `place` and marks it there for a moment, as a step back to where the text stood around `from`. */
  jump(place: HTMLElement, from: HTMLElement): void;
  /** Gives up every step still standing, as the text leaves; one a navigation left behind is stepped over, not taken back. */
  giveUp(): void;
}

// The box the text scrolls in: the nearest one around it that scrolls, or the page's own.
function scrollerAround(element: HTMLElement): HTMLElement | null {
  for (let at = element.parentElement; at !== null; at = at.parentElement) {
    const { overflowY } = getComputedStyle(at);

    if ((overflowY === "auto" || overflowY === "scroll") && at.scrollHeight > at.clientHeight) return at;
  }

  return document.scrollingElement as HTMLElement | null;
}

/** The jumps of a text, none standing yet. */
export function footnoteJumps(): FootnoteJumps {
  const steps: number[] = [];

  function remember(from: HTMLElement): void {
    const scroller = scrollerAround(from);

    if (scroller === null) return;

    const top = scroller.scrollTop;

    listenForBack();
    steps.push(
      openLayer(() => {
        steps.pop();
        scroller.scrollTo({ top });
      })
    );
  }

  return {
    jump(place, from) {
      remember(from);
      scrollWithin(place, "center");
      place.dataset["markFound"] = "";
      setTimeout(() => delete place.dataset["markFound"], FOUND_MS);
    },
    giveUp() {
      for (const id of steps.splice(0)) closeLayer(id);
    }
  };
}
