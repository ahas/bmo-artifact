import { describe, expect, test } from "bun:test";

import { DRAWN_STEPS, drawnChoice } from "../../app/domain/drawn-pictures";

describe("the copies a kept picture is drawn from", () => {
  test("follow the service's ladder, from 16 to 2048, each step at most a quarter wider", () => {
    expect([DRAWN_STEPS[0], DRAWN_STEPS.at(-1)]).toEqual([16, 2048]);
    expect(DRAWN_STEPS.slice(1).every((step, at) => step > DRAWN_STEPS[at]! && step * 4 <= DRAWN_STEPS[at]! * 5)).toBe(true);
  });

  test("are the steps below its own width, at the width it stands at", () => {
    const choice = drawnChoice("/kept?v=1", 1000, 320, 1.25)!;

    expect(choice.sizes).toBe("320px");
    expect(choice.srcset.split(", ").at(-1)).toBe(`/kept?v=1&width=${DRAWN_STEPS.filter((step) => step < 1000).at(-1)} ${DRAWN_STEPS.filter((step) => step < 1000).at(-1)}w`);
    expect(choice.srcset).not.toContain("1000w");
  });

  test("are none where its own file is within a step of the pixels it stands on", () => {
    expect(drawnChoice("/kept", 480, 320, 1.25)).toBeNull();
    expect(drawnChoice("/kept", 1000, 0, 1)).toBeNull();
  });
});
