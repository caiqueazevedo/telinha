import { describe, expect, it } from "vitest";
import { formatElapsed } from "./format";

describe("formatElapsed", () => {
  it("formats minutes and seconds with padding", () => {
    expect(formatElapsed(65_400)).toBe("01:05");
  });

  it("adds hours past the first hour", () => {
    expect(formatElapsed(3_723_000)).toBe("1:02:03");
  });

  it("clamps negative values from clock skew to zero", () => {
    expect(formatElapsed(-500)).toBe("00:00");
  });
});
