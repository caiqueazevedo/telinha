import { describe, expect, it } from "vitest";
import { isUsable, normalizeDrag } from "./geometry";

describe("normalizeDrag", () => {
  it("keeps a top-left to bottom-right drag as is", () => {
    expect(normalizeDrag({ x: 10, y: 20 }, { x: 110, y: 70 })).toEqual({ x: 10, y: 20, width: 100, height: 50 });
  });

  it("normalizes a drag towards the top-left", () => {
    expect(normalizeDrag({ x: 110, y: 70 }, { x: 10, y: 20 })).toEqual({ x: 10, y: 20, width: 100, height: 50 });
  });

  it("normalizes a drag across the diagonal", () => {
    expect(normalizeDrag({ x: 110, y: 20 }, { x: 10, y: 70 })).toEqual({ x: 10, y: 20, width: 100, height: 50 });
  });

  it("scales CSS pixels to physical pixels and rounds", () => {
    expect(normalizeDrag({ x: 10.2, y: 20 }, { x: 110, y: 70.4 }, 1.5)).toEqual({ x: 15, y: 30, width: 150, height: 76 });
  });
});

describe("isUsable", () => {
  it("accepts a region with both sides at the minimum", () => {
    expect(isUsable({ x: 0, y: 0, width: 16, height: 16 })).toBe(true);
  });

  it("rejects a click or a thin strip", () => {
    expect(isUsable({ x: 0, y: 0, width: 0, height: 0 })).toBe(false);
    expect(isUsable({ x: 0, y: 0, width: 400, height: 15 })).toBe(false);
  });
});
