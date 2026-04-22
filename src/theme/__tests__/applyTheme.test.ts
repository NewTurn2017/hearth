// src/theme/__tests__/applyTheme.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { applyTheme, CUSTOM_STYLE_ID } from "../applyTheme";

beforeEach(() => {
  document.documentElement.removeAttribute("data-theme");
  document.getElementById(CUSTOM_STYLE_ID)?.remove();
});

describe("applyTheme", () => {
  it("sets data-theme for a preset", () => {
    applyTheme({ kind: "preset", id: "midnight" });
    expect(document.documentElement.getAttribute("data-theme")).toBe("midnight");
  });

  it("removes the custom style tag when switching preset → preset", () => {
    applyTheme({ kind: "custom", baseMode: "dark", brandHex: "#ff8000" });
    expect(document.getElementById(CUSTOM_STYLE_ID)).toBeTruthy();
    applyTheme({ kind: "preset", id: "forest" });
    expect(document.getElementById(CUSTOM_STYLE_ID)).toBeNull();
  });

  it("injects a custom style tag with the 13 token variables", () => {
    applyTheme({ kind: "custom", baseMode: "light", brandHex: "#1d4ed8" });
    expect(document.documentElement.getAttribute("data-theme")).toBe("custom");
    const tag = document.getElementById(CUSTOM_STYLE_ID);
    expect(tag).toBeTruthy();
    const css = tag!.textContent ?? "";
    expect(css).toContain('[data-theme="custom"]');
    expect(css).toContain("--color-brand: #1d4ed8");
    expect(css).toContain("--color-surface-0: #fafaf7"); // Linen neutral
    expect(css).toContain("--color-brand-soft: rgba(29, 78, 216, 0.18)");
  });

  it("upserts (not appends duplicates) when re-applying custom", () => {
    applyTheme({ kind: "custom", baseMode: "dark", brandHex: "#ff8000" });
    applyTheme({ kind: "custom", baseMode: "dark", brandHex: "#10b981" });
    const tags = document.querySelectorAll(`#${CUSTOM_STYLE_ID}`);
    expect(tags.length).toBe(1);
    expect(tags[0].textContent).toContain("--color-brand: #10b981");
  });
});
