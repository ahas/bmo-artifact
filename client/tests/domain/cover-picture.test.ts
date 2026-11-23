import { describe, expect, test } from "bun:test";

import { coverStep, coverWidth } from "../../app/domain/cover-picture";
import { DRAWN_STEPS } from "../../app/domain/drawn-pictures";

describe("a cover asked at its box", () => {
  test("asks the ladder's first step at or above what it covers, and the widest past the last", () => {
    expect(coverStep(80)).toBe(DRAWN_STEPS.find((step) => step >= 80)!);
    expect(coverStep(99999)).toBe(2048);
  });

  test("covers the box's longer side before its shape is known", () => {
    expect(coverWidth({ width: 80, height: 80 }, 2, null)).toBe(coverStep(160));
    expect(coverWidth({ width: 320, height: 180 }, 1, null)).toBe(coverStep(320));
  });

  test("covers the box's height times its shape where the picture is wider than the box", () => {
    expect(coverWidth({ width: 80, height: 80 }, 2, 2)).toBe(coverStep(320));
    expect(coverWidth({ width: 320, height: 180 }, 1, 0.5)).toBe(coverStep(320));
  });
});
