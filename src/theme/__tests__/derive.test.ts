// src/theme/__tests__/derive.test.ts
import { describe, it, expect } from "vitest";
import {
  hexToRgb,
  rgbToHsl,
  hslToHex,
  deriveBrand,
  composeCustomTokens,
} from "../derive";

describe("hexToRgb", () => {
  it("parses 6-digit hex with #", () => {
    expect(hexToRgb("#ff8000")).toEqual({ r: 255, g: 128, b: 0 });
  });
  it("parses 6-digit hex without #", () => {
    expect(hexToRgb("ff8000")).toEqual({ r: 255, g: 128, b: 0 });
  });
  it("parses 3-digit shorthand", () => {
    expect(hexToRgb("#f80")).toEqual({ r: 255, g: 136, b: 0 });
  });
  it("is case-insensitive", () => {
    expect(hexToRgb("#AbCdEf")).toEqual({ r: 171, g: 205, b: 239 });
  });
  it("throws on invalid hex", () => {
    expect(() => hexToRgb("#zzz")).toThrow();
    expect(() => hexToRgb("not-a-color")).toThrow();
    expect(() => hexToRgb("#12345")).toThrow();
  });
});

describe("rgbToHsl / hslToHex round-trip", () => {
  it("round-trips pure red", () => {
    const hsl = rgbToHsl(255, 0, 0);
    expect(hsl.h).toBeCloseTo(0, 0);
    expect(hsl.s).toBeCloseTo(100, 0);
    expect(hsl.l).toBeCloseTo(50, 0);
    expect(hslToHex(hsl.h, hsl.s, hsl.l).toLowerCase()).toBe("#ff0000");
  });
  it("round-trips black", () => {
    const hsl = rgbToHsl(0, 0, 0);
    expect(hsl.l).toBe(0);
    expect(hslToHex(hsl.h, hsl.s, hsl.l).toLowerCase()).toBe("#000000");
  });
  it("round-trips white", () => {
    const hsl = rgbToHsl(255, 255, 255);
    expect(hsl.l).toBe(100);
    expect(hslToHex(hsl.h, hsl.s, hsl.l).toLowerCase()).toBe("#ffffff");
  });
});

describe("deriveBrand", () => {
  it("brandHi lifts lightness by 10", () => {
    // #d97706 has L ~= 44% — +10 → 54%, inside [45, 75], stays at 54
    const out = deriveBrand("#d97706");
    expect(out["--color-brand"]).toBe("#d97706");
    const hsl = rgbToHsl(...Object.values(hexToRgb(out["--color-brand-hi"])) as [number, number, number]);
    expect(hsl.l).toBeGreaterThanOrEqual(45);
    expect(hsl.l).toBeLessThanOrEqual(75);
  });
  it("brandHi clamps lightness minimum at 45% for very dark input", () => {
    const out = deriveBrand("#000000");
    const hsl = rgbToHsl(...Object.values(hexToRgb(out["--color-brand-hi"])) as [number, number, number]);
    expect(hsl.l).toBe(45);
  });
  it("brandHi clamps lightness maximum at 75% for very light input", () => {
    const out = deriveBrand("#ffffff");
    const hsl = rgbToHsl(...Object.values(hexToRgb(out["--color-brand-hi"])) as [number, number, number]);
    expect(hsl.l).toBe(75);
  });
  it("brandSoft is rgba with alpha 0.18", () => {
    expect(deriveBrand("#ff8000")["--color-brand-soft"]).toBe("rgba(255, 128, 0, 0.18)");
  });
  it("accepts 3-digit shorthand", () => {
    expect(deriveBrand("#f80")["--color-brand"]).toBe("#f80");
    expect(deriveBrand("#f80")["--color-brand-soft"]).toBe("rgba(255, 136, 0, 0.18)");
  });
  it("throws on invalid hex", () => {
    expect(() => deriveBrand("#zzz")).toThrow();
  });
});

describe("composeCustomTokens", () => {
  it("dark base copies Carbon neutrals and swaps brand", () => {
    const tokens = composeCustomTokens({
      kind: "custom",
      baseMode: "dark",
      brandHex: "#ff8000",
    });
    expect(tokens["--color-surface-0"]).toBe("#111111"); // Carbon
    expect(tokens["--color-text-hi"]).toBe("#f5f5f5"); // Carbon
    expect(tokens["--color-brand"]).toBe("#ff8000");
    expect(tokens["--color-brand-soft"]).toBe("rgba(255, 128, 0, 0.18)");
  });
  it("light base copies Linen neutrals and swaps brand", () => {
    const tokens = composeCustomTokens({
      kind: "custom",
      baseMode: "light",
      brandHex: "#1d4ed8",
    });
    expect(tokens["--color-surface-0"]).toBe("#fafaf7"); // Linen
    expect(tokens["--color-text-hi"]).toBe("#1a1a1a"); // Linen
    expect(tokens["--color-brand"]).toBe("#1d4ed8");
  });
});
