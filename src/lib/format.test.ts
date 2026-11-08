import { describe, expect, it } from "vitest";
import { formatBytes, formatElapsed } from "./format";

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

describe("formatBytes", () => {
  it("uses KB below a megabyte, never showing zero", () => {
    expect(formatBytes(0)).toBe("1 KB");
    expect(formatBytes(300 * 1024)).toBe("300 KB");
  });

  it("uses MB and GB above that", () => {
    expect(formatBytes(15.25 * 1024 ** 2)).toBe("15.3 MB");
    expect(formatBytes(2.5 * 1024 ** 3)).toBe("2.50 GB");
  });
});
