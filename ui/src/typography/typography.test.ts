import { describe, expect, it } from "vitest";
import {
  clampZoom,
  easeZoom,
  fitFontSize,
  formatZoom,
  readerFontSize,
  resolveTextSizing,
  resolveZoom,
  stepZoom,
  wheelZoom,
} from "./typography";

const fit = { availableWidth: 660, measure: 66, chPerPx: 0.5, minFontSize: 14, maxFontSize: 32 };

describe("zoom", () => {
  it("clamps and survives bad input", () => {
    expect(clampZoom(10)).toBe(3);
    expect(clampZoom(0.1)).toBe(0.5);
    expect(clampZoom(Number.NaN)).toBe(1);
  });

  it("steps like a browser", () => {
    expect(stepZoom(1, 1)).toBe(1.1);
    expect(stepZoom(1, -1)).toBe(0.9);
    expect(stepZoom(1.05, 1)).toBe(1.1);
    expect(stepZoom(1.05, -1)).toBe(1);
    expect(stepZoom(3, 1)).toBe(3);
    expect(stepZoom(0.5, -1)).toBe(0.5);
  });

  it("wheel zooms in on scroll up, proportionally", () => {
    expect(wheelZoom(1, -100)).toBeGreaterThan(1.1);
    expect(wheelZoom(1, -100)).toBeLessThan(1.2);
    expect(wheelZoom(1, 100)).toBeLessThan(1);
    expect(wheelZoom(1, -3, 1)).toBeCloseTo(wheelZoom(1, -120), 5);
    expect(wheelZoom(2.9, -1000)).toBe(3);
  });

  it("resolves document, then default, then 100%", () => {
    expect(resolveZoom(1.5, 2)).toBe(1.5);
    expect(resolveZoom(null, 2)).toBe(2);
    expect(resolveZoom(null, null)).toBe(1);
  });

  it("formats and animates", () => {
    expect(formatZoom(1.1)).toBe("110%");
    expect(easeZoom(1, 2, 0)).toBe(1);
    expect(easeZoom(1, 2, 1)).toBe(2);
    expect(easeZoom(1, 2, 0.5)).toBeGreaterThan(1.5);
  });
});

describe("text sizing", () => {
  it("resolves document, then user default, then theme", () => {
    expect(resolveTextSizing("fit", "fixed", "fixed")).toBe("fit");
    expect(resolveTextSizing(null, "fit", "fixed")).toBe("fit");
    expect(resolveTextSizing(null, null, "fit")).toBe("fit");
  });

  it("fit fills the measure and clamps", () => {
    // 66 characters at 0.5ch per px in 660px: 20px.
    expect(fitFontSize(fit)).toBe(20);
    expect(fitFontSize({ ...fit, availableWidth: 3000 })).toBe(32);
    expect(fitFontSize({ ...fit, availableWidth: 200 })).toBe(14);
    expect(fitFontSize({ ...fit, availableWidth: 0 })).toBe(14);
  });

  it("zoom multiplies either mode", () => {
    expect(readerFontSize("fixed", 18, fit, 1.5)).toBe(27);
    expect(readerFontSize("fit", 18, fit, 1.5)).toBe(30);
  });
});
