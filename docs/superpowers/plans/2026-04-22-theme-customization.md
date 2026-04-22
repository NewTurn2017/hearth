# Theme Customization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user pick one of 10 presets (5 dark + 5 light) or a custom brand color, with instant preview, persistence across restarts, and sync between the main window and Quick Capture.

**Architecture:** Warm Paper stays in Tailwind's `@theme` block as the no-attribute default. Nine other presets live as `[data-theme="<id>"]` CSS blocks in `src/theme/theme.css`. Custom themes inject a runtime `<style id="hearth-custom-theme">` tag with 11 derived tokens. A pure `applyTheme(theme)` function mutates `<html>`. A React `ThemeContext` wraps the setter with persistence (localStorage cache + Tauri settings KV) and a `theme-changed` event so Quick Capture stays in sync.

**Tech Stack:** React 19 + Tailwind 4 CSS variables, Tauri 2 commands + events, SQLite settings KV, Vitest + @testing-library/react.

**Spec:** [docs/superpowers/specs/2026-04-22-theme-customization-design.md](../specs/2026-04-22-theme-customization-design.md)

---

## File Structure

**Create (9 files):**

| File | Responsibility |
|---|---|
| `src/theme/types.ts` | `PresetId`, `ThemeSetting`, `DEFAULT_THEME`, `ThemeTokens` interface |
| `src/theme/presets.ts` | Full 11-token table for the 10 presets; `PRESET_META` display data |
| `src/theme/derive.ts` | Pure: `deriveBrand(hex)` + `composeCustomTokens(theme)` |
| `src/theme/applyTheme.ts` | Pure DOM mutation: set `data-theme`, upsert/remove custom style tag |
| `src/theme/theme.css` | Nine `[data-theme="..."]` rule blocks (all presets except warm-paper) |
| `src/theme/ThemeContext.tsx` | React provider + `useTheme()` hook |
| `src/theme/__tests__/derive.test.ts` | Unit tests for HEX→HSL, lightness clamp, alpha, token composition |
| `src/theme/__tests__/applyTheme.test.ts` | DOM tests (jsdom) for attribute + style tag behavior |
| `src/components/SettingsThemeSection.tsx` | Settings UI tab content: 2 preset grids + custom picker |
| `src/components/__tests__/SettingsThemeSection.test.tsx` | Component tests: card click, custom input, debounce, revert |

**Modify (7 files):**

| File | Change |
|---|---|
| `src/App.css` | Add `@import "./theme/theme.css";` after the `@theme` block |
| `src/main.tsx` | Pre-paint from `localStorage` cache before React mounts |
| `src/App.tsx` | Wrap tree in `<ThemeProvider>` |
| `src/windows/QuickCapture.tsx` | Pre-paint + listen to `theme-changed` Tauri event |
| `src/api.ts` | Export `getTheme()` / `setTheme()` wrappers |
| `src/components/SettingsDialog.tsx` | Add `"theme"` tab (position 2, between 일반 and AI) |
| `src-tauri/src/cmd_settings.rs` | Add `K_THEME` key + `get_theme` / `set_theme` commands |
| `src-tauri/src/lib.rs` | Register the two new commands in the invoke handler |

---

## Task 1: Theme types + preset token tables

**Files:**
- Create: `src/theme/types.ts`
- Create: `src/theme/presets.ts`

No tests for this task — it's pure data that later tests consume.

- [ ] **Step 1: Create `src/theme/types.ts`**

```ts
// Theme types for the 10 presets + custom-brand accent mode.

export type PresetId =
  | "warm-paper"
  | "midnight"
  | "forest"
  | "plum"
  | "carbon"
  | "cream"
  | "linen"
  | "mint"
  | "blush"
  | "arctic";

export const DARK_PRESETS: PresetId[] = [
  "warm-paper",
  "midnight",
  "forest",
  "plum",
  "carbon",
];

export const LIGHT_PRESETS: PresetId[] = [
  "cream",
  "linen",
  "mint",
  "blush",
  "arctic",
];

export type BaseMode = "light" | "dark";

export type ThemeSetting =
  | { kind: "preset"; id: PresetId }
  | { kind: "custom"; baseMode: BaseMode; brandHex: string };

export const DEFAULT_THEME: ThemeSetting = { kind: "preset", id: "warm-paper" };

// The 11 CSS custom-property token names that the theme layer owns.
// Priority, category, semantic, radius, shadow, motion, typography tokens
// stay global in @theme (theme-independent).
export type ThemeTokens = {
  "--color-surface-0": string;
  "--color-surface-1": string;
  "--color-surface-2": string;
  "--color-surface-3": string;
  "--color-border": string;
  "--color-border-strong": string;
  "--color-text-hi": string;
  "--color-text": string;
  "--color-text-muted": string;
  "--color-text-dim": string;
  "--color-brand": string;
  "--color-brand-hi": string;
  "--color-brand-soft": string;
};
```

Note: `ThemeTokens` has 13 keys (11 visual + brand-hi + brand-soft). The spec's "11 tokens" groups brand/brand-hi/brand-soft as three, so this matches.

- [ ] **Step 2: Create `src/theme/presets.ts`**

```ts
// First-pass token tables for the 10 presets. Warm Paper mirrors the values in
// src/App.css @theme so the no-data-theme fallback is byte-identical. Light
// presets use dark text on light surfaces; dark presets use light text on dark
// surfaces. WCAG AA body-text contrast verified by hand during design.

import type { PresetId, ThemeTokens } from "./types";

export const PRESETS: Record<PresetId, ThemeTokens> = {
  "warm-paper": {
    "--color-surface-0": "#141312",
    "--color-surface-1": "#1a1917",
    "--color-surface-2": "#221f19",
    "--color-surface-3": "#2a2721",
    "--color-border": "#2e2a23",
    "--color-border-strong": "#3a362e",
    "--color-text-hi": "#f4efcf",
    "--color-text": "#ebeadf",
    "--color-text-muted": "#a7a496",
    "--color-text-dim": "#7a7668",
    "--color-brand": "#d97706",
    "--color-brand-hi": "#fbbf24",
    "--color-brand-soft": "rgba(217, 119, 6, 0.18)",
  },
  midnight: {
    "--color-surface-0": "#0f1420",
    "--color-surface-1": "#141b2c",
    "--color-surface-2": "#1b2439",
    "--color-surface-3": "#232d46",
    "--color-border": "#273352",
    "--color-border-strong": "#334063",
    "--color-text-hi": "#e6ecff",
    "--color-text": "#d8ddef",
    "--color-text-muted": "#8d97b5",
    "--color-text-dim": "#5e6885",
    "--color-brand": "#3b82f6",
    "--color-brand-hi": "#60a5fa",
    "--color-brand-soft": "rgba(59, 130, 246, 0.18)",
  },
  forest: {
    "--color-surface-0": "#0f1612",
    "--color-surface-1": "#141e18",
    "--color-surface-2": "#1b2a20",
    "--color-surface-3": "#223529",
    "--color-border": "#26402f",
    "--color-border-strong": "#30503b",
    "--color-text-hi": "#e5f3e8",
    "--color-text": "#d7e7dc",
    "--color-text-muted": "#8ea89a",
    "--color-text-dim": "#5f786a",
    "--color-brand": "#10b981",
    "--color-brand-hi": "#34d399",
    "--color-brand-soft": "rgba(16, 185, 129, 0.18)",
  },
  plum: {
    "--color-surface-0": "#1a1320",
    "--color-surface-1": "#211828",
    "--color-surface-2": "#2b1f35",
    "--color-surface-3": "#362843",
    "--color-border": "#3a2b49",
    "--color-border-strong": "#4a3860",
    "--color-text-hi": "#f0e9fa",
    "--color-text": "#e1d8ef",
    "--color-text-muted": "#a295b8",
    "--color-text-dim": "#766888",
    "--color-brand": "#a855f7",
    "--color-brand-hi": "#c084fc",
    "--color-brand-soft": "rgba(168, 85, 247, 0.18)",
  },
  carbon: {
    "--color-surface-0": "#111111",
    "--color-surface-1": "#181818",
    "--color-surface-2": "#1f1f1f",
    "--color-surface-3": "#272727",
    "--color-border": "#2c2c2c",
    "--color-border-strong": "#3a3a3a",
    "--color-text-hi": "#f5f5f5",
    "--color-text": "#e5e5e5",
    "--color-text-muted": "#a3a3a3",
    "--color-text-dim": "#737373",
    "--color-brand": "#f97316",
    "--color-brand-hi": "#fb923c",
    "--color-brand-soft": "rgba(249, 115, 22, 0.18)",
  },
  cream: {
    "--color-surface-0": "#fdf8ef",
    "--color-surface-1": "#f6efdf",
    "--color-surface-2": "#ede4cc",
    "--color-surface-3": "#e2d7b8",
    "--color-border": "#d5c79c",
    "--color-border-strong": "#b8a775",
    "--color-text-hi": "#2a2218",
    "--color-text": "#3d3325",
    "--color-text-muted": "#6a5c47",
    "--color-text-dim": "#94866f",
    "--color-brand": "#b45309",
    "--color-brand-hi": "#d97706",
    "--color-brand-soft": "rgba(180, 83, 9, 0.18)",
  },
  linen: {
    "--color-surface-0": "#fafaf7",
    "--color-surface-1": "#f3f3ee",
    "--color-surface-2": "#eaeae4",
    "--color-surface-3": "#dededa",
    "--color-border": "#d0d0cc",
    "--color-border-strong": "#b0b0ab",
    "--color-text-hi": "#1a1a1a",
    "--color-text": "#2e2e2e",
    "--color-text-muted": "#5f5f5c",
    "--color-text-dim": "#8b8b87",
    "--color-brand": "#1d4ed8",
    "--color-brand-hi": "#3b82f6",
    "--color-brand-soft": "rgba(29, 78, 216, 0.18)",
  },
  mint: {
    "--color-surface-0": "#f4faf6",
    "--color-surface-1": "#e9f3ec",
    "--color-surface-2": "#ddeae1",
    "--color-surface-3": "#ceddd3",
    "--color-border": "#b9ccbf",
    "--color-border-strong": "#9ab2a2",
    "--color-text-hi": "#152419",
    "--color-text": "#223024",
    "--color-text-muted": "#4e6455",
    "--color-text-dim": "#7d9283",
    "--color-brand": "#059669",
    "--color-brand-hi": "#10b981",
    "--color-brand-soft": "rgba(5, 150, 105, 0.18)",
  },
  blush: {
    "--color-surface-0": "#fdf5f6",
    "--color-surface-1": "#f8e9ec",
    "--color-surface-2": "#efdadd",
    "--color-surface-3": "#e3c7cd",
    "--color-border": "#d4b2ba",
    "--color-border-strong": "#b88f99",
    "--color-text-hi": "#2a1619",
    "--color-text": "#3c2429",
    "--color-text-muted": "#6a4a51",
    "--color-text-dim": "#947079",
    "--color-brand": "#be185d",
    "--color-brand-hi": "#ec4899",
    "--color-brand-soft": "rgba(190, 24, 93, 0.18)",
  },
  arctic: {
    "--color-surface-0": "#f4f7fb",
    "--color-surface-1": "#e8eef5",
    "--color-surface-2": "#dae4ef",
    "--color-surface-3": "#c8d6e5",
    "--color-border": "#b1c3d8",
    "--color-border-strong": "#8ea7c2",
    "--color-text-hi": "#0b1a2a",
    "--color-text": "#1a2838",
    "--color-text-muted": "#4b5f77",
    "--color-text-dim": "#7b8ea5",
    "--color-brand": "#0ea5e9",
    "--color-brand-hi": "#38bdf8",
    "--color-brand-soft": "rgba(14, 165, 233, 0.18)",
  },
};

export const PRESET_META: Record<PresetId, { label: string; mode: "light" | "dark" }> = {
  "warm-paper": { label: "Warm Paper", mode: "dark" },
  midnight:    { label: "Midnight",   mode: "dark" },
  forest:      { label: "Forest",     mode: "dark" },
  plum:        { label: "Plum",       mode: "dark" },
  carbon:      { label: "Carbon",     mode: "dark" },
  cream:       { label: "Cream",      mode: "light" },
  linen:       { label: "Linen",      mode: "light" },
  mint:        { label: "Mint",       mode: "light" },
  blush:       { label: "Blush",      mode: "light" },
  arctic:      { label: "Arctic",     mode: "light" },
};
```

- [ ] **Step 3: Commit**

```bash
git add src/theme/types.ts src/theme/presets.ts
git commit -m "feat(theme): add preset token tables and theme types"
```

---

## Task 2: Pure derivation helpers (HEX → brand-hi/brand-soft, custom token composition)

**Files:**
- Create: `src/theme/derive.ts`
- Test: `src/theme/__tests__/derive.test.ts`

- [ ] **Step 1: Write the failing tests**

```ts
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
npm test -- src/theme/__tests__/derive.test.ts
```

Expected: FAIL — `Cannot find module '../derive'`.

- [ ] **Step 3: Implement `src/theme/derive.ts`**

```ts
// Pure color math for the theme layer.
//
// - HEX ⇄ RGB ⇄ HSL conversions (integer channels, HSL in percent).
// - deriveBrand: from one brand HEX, produce brand / brand-hi / brand-soft.
// - composeCustomTokens: full 13-token set for a custom theme by copying
//   the neutral preset (Carbon for dark, Linen for light) and swapping in
//   the derived brand values.

import type { ThemeSetting, ThemeTokens } from "./types";
import { PRESETS } from "./presets";

export type Rgb = { r: number; g: number; b: number };
export type Hsl = { h: number; s: number; l: number };

const HEX6 = /^#?([0-9a-f]{6})$/i;
const HEX3 = /^#?([0-9a-f]{3})$/i;

export function hexToRgb(hex: string): Rgb {
  const m6 = HEX6.exec(hex);
  if (m6) {
    const n = parseInt(m6[1], 16);
    return { r: (n >> 16) & 0xff, g: (n >> 8) & 0xff, b: n & 0xff };
  }
  const m3 = HEX3.exec(hex);
  if (m3) {
    const s = m3[1];
    return {
      r: parseInt(s[0] + s[0], 16),
      g: parseInt(s[1] + s[1], 16),
      b: parseInt(s[2] + s[2], 16),
    };
  }
  throw new Error(`invalid hex color: ${hex}`);
}

export function rgbToHsl(r: number, g: number, b: number): Hsl {
  const R = r / 255,
    G = g / 255,
    B = b / 255;
  const max = Math.max(R, G, B);
  const min = Math.min(R, G, B);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case R:
        h = ((G - B) / d + (G < B ? 6 : 0)) * 60;
        break;
      case G:
        h = ((B - R) / d + 2) * 60;
        break;
      case B:
        h = ((R - G) / d + 4) * 60;
        break;
    }
  }
  return { h, s: s * 100, l: l * 100 };
}

function hue2rgb(p: number, q: number, t: number) {
  if (t < 0) t += 1;
  if (t > 1) t -= 1;
  if (t < 1 / 6) return p + (q - p) * 6 * t;
  if (t < 1 / 2) return q;
  if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
  return p;
}

export function hslToHex(h: number, s: number, l: number): string {
  const H = ((h % 360) + 360) % 360 / 360;
  const S = Math.max(0, Math.min(100, s)) / 100;
  const L = Math.max(0, Math.min(100, l)) / 100;
  let R: number, G: number, B: number;
  if (S === 0) {
    R = G = B = L;
  } else {
    const q = L < 0.5 ? L * (1 + S) : L + S - L * S;
    const p = 2 * L - q;
    R = hue2rgb(p, q, H + 1 / 3);
    G = hue2rgb(p, q, H);
    B = hue2rgb(p, q, H - 1 / 3);
  }
  const toHex = (v: number) => {
    const n = Math.round(v * 255);
    return n.toString(16).padStart(2, "0");
  };
  return `#${toHex(R)}${toHex(G)}${toHex(B)}`;
}

export type BrandTokens = Pick<
  ThemeTokens,
  "--color-brand" | "--color-brand-hi" | "--color-brand-soft"
>;

/**
 * From a brand HEX, produce brand / brand-hi / brand-soft.
 * - brand     = input (preserved verbatim, including 3-digit shorthand)
 * - brand-hi  = same hue/saturation, lightness + 10 clamped to [45, 75]
 * - brand-soft = rgba with alpha 0.18
 */
export function deriveBrand(hex: string): BrandTokens {
  const { r, g, b } = hexToRgb(hex);
  const { h, s, l } = rgbToHsl(r, g, b);
  const hiL = Math.max(45, Math.min(75, l + 10));
  return {
    "--color-brand": hex,
    "--color-brand-hi": hslToHex(h, s, hiL),
    "--color-brand-soft": `rgba(${r}, ${g}, ${b}, 0.18)`,
  };
}

/**
 * Build the complete 13-key token map for a custom theme. Copies the neutral
 * preset (Carbon for dark, Linen for light) for the 10 non-brand tokens and
 * overlays the derived brand triplet.
 */
export function composeCustomTokens(
  theme: Extract<ThemeSetting, { kind: "custom" }>,
): ThemeTokens {
  const base = theme.baseMode === "dark" ? PRESETS.carbon : PRESETS.linen;
  const brand = deriveBrand(theme.brandHex);
  return { ...base, ...brand };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npm test -- src/theme/__tests__/derive.test.ts
```

Expected: PASS — all 12 tests green.

- [ ] **Step 5: Commit**

```bash
git add src/theme/derive.ts src/theme/__tests__/derive.test.ts
git commit -m "feat(theme): add color math and custom-token derivation"
```

---

## Task 3: `applyTheme` DOM function

**Files:**
- Create: `src/theme/applyTheme.ts`
- Test: `src/theme/__tests__/applyTheme.test.ts`

- [ ] **Step 1: Write the failing tests**

```ts
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
npm test -- src/theme/__tests__/applyTheme.test.ts
```

Expected: FAIL — `Cannot find module '../applyTheme'`.

- [ ] **Step 3: Implement `src/theme/applyTheme.ts`**

```ts
// Pure DOM writer for the theme layer. Called on boot (pre-React) and from
// ThemeContext.setTheme. No React, no side effects beyond <html> attribute
// and an optional <style> tag in <head>.

import type { ThemeSetting } from "./types";
import { composeCustomTokens } from "./derive";

export const CUSTOM_STYLE_ID = "hearth-custom-theme";

export function applyTheme(theme: ThemeSetting): void {
  const html = document.documentElement;
  if (theme.kind === "preset") {
    html.setAttribute("data-theme", theme.id);
    document.getElementById(CUSTOM_STYLE_ID)?.remove();
    return;
  }
  const tokens = composeCustomTokens(theme);
  const body = Object.entries(tokens)
    .map(([k, v]) => `  ${k}: ${v};`)
    .join("\n");
  const css = `:root[data-theme="custom"] {\n${body}\n}`;
  let tag = document.getElementById(CUSTOM_STYLE_ID) as HTMLStyleElement | null;
  if (!tag) {
    tag = document.createElement("style");
    tag.id = CUSTOM_STYLE_ID;
    document.head.appendChild(tag);
  }
  tag.textContent = css;
  html.setAttribute("data-theme", "custom");
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npm test -- src/theme/__tests__/applyTheme.test.ts
```

Expected: PASS — 4 tests green.

- [ ] **Step 5: Commit**

```bash
git add src/theme/applyTheme.ts src/theme/__tests__/applyTheme.test.ts
git commit -m "feat(theme): applyTheme DOM writer with custom style injection"
```

---

## Task 4: `theme.css` — nine `[data-theme]` preset blocks

**Files:**
- Create: `src/theme/theme.css`
- Modify: `src/App.css`

This task has no unit test — visual correctness is verified in the manual-QA task. Commit and move on.

- [ ] **Step 1: Create `src/theme/theme.css`**

```css
/* Nine presets as [data-theme="..."] rule blocks. The tenth — "warm-paper" —
   lives in src/App.css @theme as the no-attribute default, so setting
   data-theme="warm-paper" falls through to that cascade (and we do set the
   attribute so the UI's "current theme" highlight works uniformly). */

:root[data-theme="midnight"] {
  --color-surface-0: #0f1420;
  --color-surface-1: #141b2c;
  --color-surface-2: #1b2439;
  --color-surface-3: #232d46;
  --color-border: #273352;
  --color-border-strong: #334063;
  --color-text-hi: #e6ecff;
  --color-text: #d8ddef;
  --color-text-muted: #8d97b5;
  --color-text-dim: #5e6885;
  --color-brand: #3b82f6;
  --color-brand-hi: #60a5fa;
  --color-brand-soft: rgba(59, 130, 246, 0.18);
}

:root[data-theme="forest"] {
  --color-surface-0: #0f1612;
  --color-surface-1: #141e18;
  --color-surface-2: #1b2a20;
  --color-surface-3: #223529;
  --color-border: #26402f;
  --color-border-strong: #30503b;
  --color-text-hi: #e5f3e8;
  --color-text: #d7e7dc;
  --color-text-muted: #8ea89a;
  --color-text-dim: #5f786a;
  --color-brand: #10b981;
  --color-brand-hi: #34d399;
  --color-brand-soft: rgba(16, 185, 129, 0.18);
}

:root[data-theme="plum"] {
  --color-surface-0: #1a1320;
  --color-surface-1: #211828;
  --color-surface-2: #2b1f35;
  --color-surface-3: #362843;
  --color-border: #3a2b49;
  --color-border-strong: #4a3860;
  --color-text-hi: #f0e9fa;
  --color-text: #e1d8ef;
  --color-text-muted: #a295b8;
  --color-text-dim: #766888;
  --color-brand: #a855f7;
  --color-brand-hi: #c084fc;
  --color-brand-soft: rgba(168, 85, 247, 0.18);
}

:root[data-theme="carbon"] {
  --color-surface-0: #111111;
  --color-surface-1: #181818;
  --color-surface-2: #1f1f1f;
  --color-surface-3: #272727;
  --color-border: #2c2c2c;
  --color-border-strong: #3a3a3a;
  --color-text-hi: #f5f5f5;
  --color-text: #e5e5e5;
  --color-text-muted: #a3a3a3;
  --color-text-dim: #737373;
  --color-brand: #f97316;
  --color-brand-hi: #fb923c;
  --color-brand-soft: rgba(249, 115, 22, 0.18);
}

:root[data-theme="cream"] {
  --color-surface-0: #fdf8ef;
  --color-surface-1: #f6efdf;
  --color-surface-2: #ede4cc;
  --color-surface-3: #e2d7b8;
  --color-border: #d5c79c;
  --color-border-strong: #b8a775;
  --color-text-hi: #2a2218;
  --color-text: #3d3325;
  --color-text-muted: #6a5c47;
  --color-text-dim: #94866f;
  --color-brand: #b45309;
  --color-brand-hi: #d97706;
  --color-brand-soft: rgba(180, 83, 9, 0.18);
}

:root[data-theme="linen"] {
  --color-surface-0: #fafaf7;
  --color-surface-1: #f3f3ee;
  --color-surface-2: #eaeae4;
  --color-surface-3: #dededa;
  --color-border: #d0d0cc;
  --color-border-strong: #b0b0ab;
  --color-text-hi: #1a1a1a;
  --color-text: #2e2e2e;
  --color-text-muted: #5f5f5c;
  --color-text-dim: #8b8b87;
  --color-brand: #1d4ed8;
  --color-brand-hi: #3b82f6;
  --color-brand-soft: rgba(29, 78, 216, 0.18);
}

:root[data-theme="mint"] {
  --color-surface-0: #f4faf6;
  --color-surface-1: #e9f3ec;
  --color-surface-2: #ddeae1;
  --color-surface-3: #ceddd3;
  --color-border: #b9ccbf;
  --color-border-strong: #9ab2a2;
  --color-text-hi: #152419;
  --color-text: #223024;
  --color-text-muted: #4e6455;
  --color-text-dim: #7d9283;
  --color-brand: #059669;
  --color-brand-hi: #10b981;
  --color-brand-soft: rgba(5, 150, 105, 0.18);
}

:root[data-theme="blush"] {
  --color-surface-0: #fdf5f6;
  --color-surface-1: #f8e9ec;
  --color-surface-2: #efdadd;
  --color-surface-3: #e3c7cd;
  --color-border: #d4b2ba;
  --color-border-strong: #b88f99;
  --color-text-hi: #2a1619;
  --color-text: #3c2429;
  --color-text-muted: #6a4a51;
  --color-text-dim: #947079;
  --color-brand: #be185d;
  --color-brand-hi: #ec4899;
  --color-brand-soft: rgba(190, 24, 93, 0.18);
}

:root[data-theme="arctic"] {
  --color-surface-0: #f4f7fb;
  --color-surface-1: #e8eef5;
  --color-surface-2: #dae4ef;
  --color-surface-3: #c8d6e5;
  --color-border: #b1c3d8;
  --color-border-strong: #8ea7c2;
  --color-text-hi: #0b1a2a;
  --color-text: #1a2838;
  --color-text-muted: #4b5f77;
  --color-text-dim: #7b8ea5;
  --color-brand: #0ea5e9;
  --color-brand-hi: #38bdf8;
  --color-brand-soft: rgba(14, 165, 233, 0.18);
}
```

- [ ] **Step 2: Add the `@import` to `src/App.css`**

Modify `src/App.css` — add line 2 (right after `@import "tailwindcss";`):

```css
@import "tailwindcss";
@import "./theme/theme.css";

@theme {
  /* Surface (warm paper dark) */
  --color-surface-0: #141312;
  ...
}
```

- [ ] **Step 3: Run the test suite to verify no regression**

```bash
npm test
```

Expected: all existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src/theme/theme.css src/App.css
git commit -m "feat(theme): add 9 [data-theme] preset CSS blocks"
```

---

## Task 5: Tauri backend `get_theme` / `set_theme`

**Files:**
- Modify: `src-tauri/src/cmd_settings.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the `K_THEME` key and commands to `cmd_settings.rs`**

At the top of `src-tauri/src/cmd_settings.rs`, after the existing `K_AUTOSTART` constant:

```rust
pub(crate) const K_THEME: &str = "ui.theme";
```

Append to the end of the file (after `set_ui_scale` but before `#[cfg(test)]`):

```rust
/// Theme is stored as an opaque JSON blob because its shape is a tagged union
/// — either `{"kind":"preset","id":"midnight"}` or
/// `{"kind":"custom","baseMode":"dark","brandHex":"#ff8000"}`. We don't parse
/// it on the Rust side; the frontend owns the schema and we just round-trip
/// the string. Empty → frontend falls back to DEFAULT_THEME.
#[tauri::command]
pub fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    read(&db, K_THEME)
}

#[tauri::command]
pub fn set_theme(state: State<'_, AppState>, theme: String) -> Result<(), String> {
    // Minimal validation: must be valid JSON so we don't persist garbage.
    serde_json::from_str::<serde_json::Value>(&theme)
        .map_err(|e| format!("invalid theme json: {e}"))?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    write(&db, K_THEME, &theme)
}
```

Verify `serde_json` is already a dependency by running:

```bash
grep -n 'serde_json' src-tauri/Cargo.toml
```

Expected: a line like `serde_json = ...`. If missing, add to `[dependencies]`:

```toml
serde_json = "1"
```

- [ ] **Step 2: Register the commands in `src-tauri/src/lib.rs`**

Find the `.invoke_handler(tauri::generate_handler![...])` block that currently lists `cmd_settings::set_ui_scale` and add two lines immediately after it:

```rust
cmd_settings::get_ui_scale,
cmd_settings::set_ui_scale,
cmd_settings::get_theme,
cmd_settings::set_theme,
```

- [ ] **Step 3: Build to verify Rust compiles**

```bash
cd src-tauri && cargo build --quiet
```

Expected: clean build with no warnings about unused imports. If `serde_json` is missing, the compile will fail — `cargo add serde_json` from inside `src-tauri/` and retry.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cmd_settings.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(theme): add get_theme/set_theme Tauri commands"
```

---

## Task 6: Frontend API wrappers

**Files:**
- Modify: `src/api.ts`

- [ ] **Step 1: Add `getTheme` / `setTheme` exports**

Append to the end of `src/api.ts`:

```ts
// Theme (persisted JSON blob — the frontend owns the schema; Rust just
// round-trips the string).
import type { ThemeSetting } from "./theme/types";

export const getTheme = async (): Promise<ThemeSetting | null> => {
  const raw = await invoke<string>("get_theme");
  if (!raw) return null;
  try {
    return JSON.parse(raw) as ThemeSetting;
  } catch {
    return null;
  }
};

export const setTheme = (theme: ThemeSetting): Promise<void> =>
  invoke<void>("set_theme", { theme: JSON.stringify(theme) });
```

- [ ] **Step 2: Run type check**

```bash
npx tsc --noEmit
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/api.ts
git commit -m "feat(theme): getTheme/setTheme api wrappers"
```

---

## Task 7: ThemeContext + ThemeProvider

**Files:**
- Create: `src/theme/ThemeContext.tsx`

No unit test for this task (wiring-only; covered indirectly by SettingsThemeSection tests in Task 10).

- [ ] **Step 1: Create `src/theme/ThemeContext.tsx`**

```tsx
// Theme state lifted into React context. Call `setTheme` to flip the
// appearance — it (1) mutates the DOM immediately, (2) updates localStorage
// for FOUC-free reloads, (3) persists to the Rust settings table, and
// (4) emits a Tauri event so the Quick Capture window reacts too.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { emit } from "@tauri-apps/api/event";
import type { ThemeSetting } from "./types";
import { DEFAULT_THEME } from "./types";
import { applyTheme } from "./applyTheme";
import * as api from "../api";

export const THEME_LS_KEY = "hearth.theme";
export const THEME_EVENT = "theme-changed";

type ThemeContextValue = {
  theme: ThemeSetting;
  setTheme: (next: ThemeSetting) => Promise<void>;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);

function readCache(): ThemeSetting | null {
  try {
    const raw = localStorage.getItem(THEME_LS_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as ThemeSetting;
  } catch {
    return null;
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemeSetting>(
    () => readCache() ?? DEFAULT_THEME,
  );

  // Reconcile with Rust-side settings on mount. If the backend disagrees with
  // the cache (e.g. user changed theme in another instance), re-apply and
  // update the cache.
  useEffect(() => {
    let alive = true;
    api
      .getTheme()
      .then((fresh) => {
        if (!alive || !fresh) return;
        if (JSON.stringify(fresh) !== JSON.stringify(theme)) {
          applyTheme(fresh);
          localStorage.setItem(THEME_LS_KEY, JSON.stringify(fresh));
          setThemeState(fresh);
        }
      })
      .catch(() => {
        /* backend unavailable; keep cached theme */
      });
    return () => {
      alive = false;
    };
    // run once on mount only — re-checking on every theme change would bounce
    // back to cache values.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setTheme = useCallback(async (next: ThemeSetting) => {
    applyTheme(next);
    localStorage.setItem(THEME_LS_KEY, JSON.stringify(next));
    setThemeState(next);
    try {
      await api.setTheme(next);
      await emit(THEME_EVENT, next);
    } catch (e) {
      // Persistence failed — leave the UI flipped, surface to console. The
      // next boot will fall back to the localStorage cache anyway.
      console.error("setTheme persist failed:", e);
    }
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme used outside ThemeProvider");
  return ctx;
}
```

- [ ] **Step 2: Run type check**

```bash
npx tsc --noEmit
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/theme/ThemeContext.tsx
git commit -m "feat(theme): ThemeProvider with persistence and event emit"
```

---

## Task 8: Pre-paint in `main.tsx` + wrap App

**Files:**
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Pre-paint from localStorage in `src/main.tsx`**

Replace the file contents with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { QuickCapture } from "./windows/QuickCapture";
import { applyTheme } from "./theme/applyTheme";
import { DEFAULT_THEME, type ThemeSetting } from "./theme/types";
import { THEME_LS_KEY } from "./theme/ThemeContext";

// Pre-paint before React mounts so the first frame lands in the correct
// theme. If the cache is missing/corrupt, fall back to DEFAULT_THEME. The
// Rust-backed reconciliation happens once React runs ThemeProvider.
(function paintThemeFromCache() {
  try {
    const raw = localStorage.getItem(THEME_LS_KEY);
    const theme: ThemeSetting = raw ? (JSON.parse(raw) as ThemeSetting) : DEFAULT_THEME;
    applyTheme(theme);
  } catch {
    applyTheme(DEFAULT_THEME);
  }
})();

const params = new URLSearchParams(window.location.search);
const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

if (params.get("window") === "quick-capture") {
  root.render(
    <React.StrictMode>
      <QuickCapture />
    </React.StrictMode>
  );
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
```

- [ ] **Step 2: Wrap `App.tsx` in `<ThemeProvider>`**

In `src/App.tsx`, add the import at the top:

```ts
import { ThemeProvider } from "./theme/ThemeContext";
```

Replace the `App` function body so `ThemeProvider` sits outside `ToastProvider`:

```tsx
function App() {
  useUiScale();
  return (
    <ThemeProvider>
      <ToastProvider>
        <Layout>
          {({ activeTab, priorities, category, openNewProject }) => (
            <>
              {activeTab === "projects" && (
                <ProjectsTab
                  priorities={priorities}
                  category={category}
                  onAdd={openNewProject}
                />
              )}
              {activeTab === "calendar" && <CalendarView />}
              {activeTab === "memos" && <MemoBoard />}
            </>
          )}
        </Layout>
      </ToastProvider>
    </ThemeProvider>
  );
}
```

- [ ] **Step 3: Run dev server and eyeball the app**

```bash
npm run dev
```

Expected: app loads with the current Warm Paper look (no regression — `data-theme="warm-paper"` falls through to the `@theme` defaults). Stop the server with Ctrl+C.

- [ ] **Step 4: Run the full test suite**

```bash
npm test
```

Expected: all tests pass (no regressions in existing tests).

- [ ] **Step 5: Commit**

```bash
git add src/main.tsx src/App.tsx
git commit -m "feat(theme): pre-paint from cache and wrap App in ThemeProvider"
```

---

## Task 9: `SettingsThemeSection` component

**Files:**
- Create: `src/components/SettingsThemeSection.tsx`
- Test: `src/components/__tests__/SettingsThemeSection.test.tsx`

- [ ] **Step 1: Write the failing tests**

```tsx
// src/components/__tests__/SettingsThemeSection.test.tsx
import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import type { ReactNode } from "react";
import { SettingsThemeSection } from "../SettingsThemeSection";
import { ThemeProvider } from "../../theme/ThemeContext";

vi.mock("../../api", () => ({
  getTheme: vi.fn().mockResolvedValue(null),
  setTheme: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import * as api from "../../api";

function wrap(ui: ReactNode) {
  return render(<ThemeProvider>{ui}</ThemeProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
});

describe("SettingsThemeSection", () => {
  it("renders all 10 preset cards grouped into dark + light", () => {
    wrap(<SettingsThemeSection />);
    expect(screen.getByRole("button", { name: /Warm Paper/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Midnight/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Forest/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Plum/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Carbon/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Cream/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Linen/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Mint/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Blush/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Arctic/ })).toBeInTheDocument();
  });

  it("clicking a preset card calls setTheme and sets data-theme", async () => {
    wrap(<SettingsThemeSection />);
    fireEvent.click(screen.getByRole("button", { name: /Midnight/ }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenCalledWith({ kind: "preset", id: "midnight" });
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("midnight");
  });

  it("marks the active preset card with aria-pressed", async () => {
    wrap(<SettingsThemeSection />);
    const carbon = screen.getByRole("button", { name: /Carbon/ });
    fireEvent.click(carbon);
    await waitFor(() => expect(carbon).toHaveAttribute("aria-pressed", "true"));
    expect(screen.getByRole("button", { name: /Midnight/ })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("debounces custom color input and previews live before save", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    const hexInput = screen.getByLabelText("강조색 HEX") as HTMLInputElement;
    fireEvent.change(hexInput, { target: { value: "#ff8000" } });
    // Before 300ms: still on previous theme
    expect(document.documentElement.getAttribute("data-theme")).not.toBe("custom");
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("custom");
    // Preview should NOT persist until "저장" click
    expect(api.setTheme).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("rejects invalid HEX with a visible error", async () => {
    wrap(<SettingsThemeSection />);
    const hexInput = screen.getByLabelText("강조색 HEX");
    fireEvent.change(hexInput, { target: { value: "nope" } });
    expect(await screen.findByText(/올바른 HEX/)).toBeInTheDocument();
  });

  it("save button persists the currently-previewed custom theme", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    fireEvent.change(screen.getByLabelText("강조색 HEX"), {
      target: { value: "#10b981" },
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    vi.useRealTimers();
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenCalledWith({
        kind: "custom",
        baseMode: "dark",
        brandHex: "#10b981",
      });
    });
  });

  it("revert link goes back to warm-paper when no prior preset", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    fireEvent.change(screen.getByLabelText("강조색 HEX"), {
      target: { value: "#ff8000" },
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    vi.useRealTimers();
    fireEvent.click(screen.getByRole("button", { name: /프리셋으로 되돌리기/ }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenLastCalledWith({
        kind: "preset",
        id: "warm-paper",
      });
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
npm test -- src/components/__tests__/SettingsThemeSection.test.tsx
```

Expected: FAIL — `Cannot find module '../SettingsThemeSection'`.

- [ ] **Step 3: Implement `src/components/SettingsThemeSection.tsx`**

```tsx
// Settings → 테마 tab content. Two preset grids (dark + light) plus a
// custom-brand picker with 300ms-debounced live preview and explicit save.

import { useEffect, useMemo, useRef, useState } from "react";
import { Check } from "lucide-react";
import { cn } from "../lib/cn";
import { Button } from "../ui/Button";
import { useTheme } from "../theme/ThemeContext";
import { DARK_PRESETS, LIGHT_PRESETS, type PresetId, type ThemeSetting } from "../theme/types";
import { PRESETS, PRESET_META } from "../theme/presets";

const HEX_RE = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

function PresetCard({
  id,
  active,
  onSelect,
}: {
  id: PresetId;
  active: boolean;
  onSelect: () => void;
}) {
  const p = PRESETS[id];
  const meta = PRESET_META[id];
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onSelect}
      className={cn(
        "relative flex flex-col gap-2 rounded-[var(--radius-lg)] p-3 text-left transition-colors",
        "border-2",
        active
          ? "border-[var(--color-brand)]"
          : "border-[var(--color-border)] hover:border-[var(--color-border-strong)]",
      )}
      style={{ background: p["--color-surface-1"] }}
    >
      <div className="flex gap-1">
        <span className="h-5 w-5 rounded" style={{ background: p["--color-surface-0"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-surface-2"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-brand"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-text-hi"] }} />
      </div>
      <div className="flex items-center justify-between">
        <span className="text-[13px] font-medium" style={{ color: p["--color-text-hi"] }}>
          {meta.label}
        </span>
        {active && (
          <Check size={14} style={{ color: p["--color-brand"] }} aria-hidden />
        )}
      </div>
    </button>
  );
}

export function SettingsThemeSection() {
  const { theme, setTheme } = useTheme();

  const activePresetId: PresetId | null =
    theme.kind === "preset" ? theme.id : null;

  // Remember the last preset the user was on so the "revert" link has
  // somewhere to go back to.
  const lastPresetRef = useRef<PresetId>(
    theme.kind === "preset" ? theme.id : "warm-paper",
  );
  useEffect(() => {
    if (theme.kind === "preset") lastPresetRef.current = theme.id;
  }, [theme]);

  // Custom picker state. Seeded from the current theme so switching to the
  // custom tab mid-session shows the current values.
  const [baseMode, setBaseMode] = useState<"light" | "dark">(
    theme.kind === "custom" ? theme.baseMode : "dark",
  );
  const [hexInput, setHexInput] = useState<string>(
    theme.kind === "custom" ? theme.brandHex : "#d97706",
  );

  const normalizedHex = useMemo(() => {
    const v = hexInput.trim();
    if (!HEX_RE.test(v)) return null;
    return v.startsWith("#") ? v : `#${v}`;
  }, [hexInput]);

  // Debounced preview: re-apply but do NOT persist until the user clicks 저장.
  useEffect(() => {
    if (!normalizedHex) return;
    const t = setTimeout(() => {
      const preview: ThemeSetting = { kind: "custom", baseMode, brandHex: normalizedHex };
      // Apply visually only — the context's setTheme would persist, which we
      // only want on explicit save. Call applyTheme directly from here.
      import("../theme/applyTheme").then((m) => m.applyTheme(preview));
    }, 300);
    return () => clearTimeout(t);
  }, [normalizedHex, baseMode]);

  const onSavePreset = async (id: PresetId) => {
    await setTheme({ kind: "preset", id });
  };

  const onSaveCustom = async () => {
    if (!normalizedHex) return;
    await setTheme({ kind: "custom", baseMode, brandHex: normalizedHex });
  };

  const onRevert = async () => {
    await setTheme({ kind: "preset", id: lastPresetRef.current });
  };

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">다크</h3>
        <div className="grid grid-cols-3 gap-2">
          {DARK_PRESETS.map((id) => (
            <PresetCard
              key={id}
              id={id}
              active={activePresetId === id}
              onSelect={() => void onSavePreset(id)}
            />
          ))}
        </div>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">라이트</h3>
        <div className="grid grid-cols-3 gap-2">
          {LIGHT_PRESETS.map((id) => (
            <PresetCard
              key={id}
              id={id}
              active={activePresetId === id}
              onSelect={() => void onSavePreset(id)}
            />
          ))}
        </div>
      </section>

      <section
        className={cn(
          "rounded-[var(--radius-lg)] border-2 p-3 flex flex-col gap-3",
          theme.kind === "custom"
            ? "border-[var(--color-brand)]"
            : "border-[var(--color-border)]",
        )}
      >
        <h3 className="text-[13px] text-[var(--color-text-hi)]">커스텀</h3>

        <div className="flex items-center gap-4 text-[13px]">
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name="theme-base-mode"
              checked={baseMode === "dark"}
              onChange={() => setBaseMode("dark")}
            />
            다크
          </label>
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name="theme-base-mode"
              checked={baseMode === "light"}
              onChange={() => setBaseMode("light")}
            />
            라이트
          </label>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="color"
            aria-label="강조색 색상 선택"
            value={normalizedHex ?? "#d97706"}
            onChange={(e) => setHexInput(e.target.value)}
            className="h-8 w-10 cursor-pointer rounded border border-[var(--color-border)]"
          />
          <input
            type="text"
            aria-label="강조색 HEX"
            value={hexInput}
            onChange={(e) => setHexInput(e.target.value)}
            placeholder="#rrggbb"
            className="font-mono rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-[12px]"
          />
          <Button size="sm" onClick={() => void onSaveCustom()} disabled={!normalizedHex}>
            저장
          </Button>
        </div>

        {!normalizedHex && (
          <p className="text-[11px] text-red-400">
            올바른 HEX 값을 입력하세요 (예: #ff8000)
          </p>
        )}

        <button
          type="button"
          onClick={() => void onRevert()}
          className="self-start text-[11px] text-[var(--color-text-muted)] hover:text-[var(--color-text)] underline"
        >
          프리셋으로 되돌리기
        </button>
      </section>
    </div>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npm test -- src/components/__tests__/SettingsThemeSection.test.tsx
```

Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsThemeSection.tsx src/components/__tests__/SettingsThemeSection.test.tsx
git commit -m "feat(theme): SettingsThemeSection with 10 preset cards and custom picker"
```

---

## Task 10: Wire `SettingsThemeSection` into `SettingsDialog`

**Files:**
- Modify: `src/components/SettingsDialog.tsx`

- [ ] **Step 1: Add the `"theme"` tab**

Open `src/components/SettingsDialog.tsx` and make three edits:

First, update the `TabKey` type on line 16:

```ts
type TabKey = "general" | "theme" | "ai" | "backup" | "categories";
```

Second, update the `TABS` array on lines 18–23 to include the new tab between 일반 and AI:

```ts
const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: "일반" },
  { key: "theme", label: "테마" },
  { key: "ai", label: "AI" },
  { key: "backup", label: "백업/가져오기" },
  { key: "categories", label: "카테고리" },
];
```

Third, import the section at the top of the file next to the other section imports:

```ts
import { SettingsThemeSection } from "./SettingsThemeSection";
```

Fourth, add the mounted section block between the General and AI blocks:

```tsx
<div className={tab === "general" ? "" : "hidden"}>
  <SettingsGeneralSection active={tab === "general"} />
</div>
<div className={tab === "theme" ? "" : "hidden"}>
  <SettingsThemeSection />
</div>
<div className={tab === "ai" ? "" : "hidden"}>
  <SettingsAiSection active={tab === "ai"} />
</div>
```

- [ ] **Step 2: Type check**

```bash
npx tsc --noEmit
```

Expected: clean.

- [ ] **Step 3: Run the full test suite**

```bash
npm test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsDialog.tsx
git commit -m "feat(theme): add 테마 tab to settings dialog"
```

---

## Task 11: Quick Capture window sync

**Files:**
- Modify: `src/windows/QuickCapture.tsx`

`main.tsx`'s pre-paint already runs before React mounts in both windows — no code needed there for the initial paint. We only need QuickCapture to reconcile with the backend on mount and subscribe to the `theme-changed` event.

- [ ] **Step 1: Add reconcile + listener to `QuickCapture.tsx`**

Add these imports at the top:

```ts
import { listen } from "@tauri-apps/api/event";
import { applyTheme } from "../theme/applyTheme";
import { DEFAULT_THEME, type ThemeSetting } from "../theme/types";
import { THEME_EVENT, THEME_LS_KEY } from "../theme/ThemeContext";
import { getTheme } from "../api";
```

Then, inside `QuickCapture()`, add a new `useEffect` near the top of the component body (after the existing focus-on-mount effect):

```tsx
// Reconcile theme with backend on mount, then listen for live changes from
// the main window. Pre-paint already happened in main.tsx from the cache.
useEffect(() => {
  let alive = true;
  let off: (() => void) | null = null;

  void getTheme().then((fresh: ThemeSetting | null) => {
    if (!alive) return;
    const applied = fresh ?? DEFAULT_THEME;
    applyTheme(applied);
    localStorage.setItem(THEME_LS_KEY, JSON.stringify(applied));
  });

  void listen<ThemeSetting>(THEME_EVENT, (e) => {
    applyTheme(e.payload);
    localStorage.setItem(THEME_LS_KEY, JSON.stringify(e.payload));
  }).then((unlisten) => {
    if (!alive) {
      unlisten();
      return;
    }
    off = unlisten;
  });

  return () => {
    alive = false;
    off?.();
  };
}, []);
```

- [ ] **Step 2: Type check**

```bash
npx tsc --noEmit
```

Expected: clean.

- [ ] **Step 3: Run the existing QuickCapture test**

```bash
npm test -- src/__tests__/QuickCapture.test.tsx
```

Expected: pass. (The test already mocks `@tauri-apps/api/event`; if it fails because `listen` or `getTheme` wasn't mocked, add them to the mock.)

If the mock needs extending, open `src/__tests__/QuickCapture.test.tsx` and add `listen: vi.fn().mockResolvedValue(() => {})` to the event mock and `getTheme: vi.fn().mockResolvedValue(null)` to the api mock — then re-run.

- [ ] **Step 4: Commit**

```bash
git add src/windows/QuickCapture.tsx src/__tests__/QuickCapture.test.tsx
git commit -m "feat(theme): Quick Capture subscribes to theme-changed events"
```

---

## Task 12: Manual verification + release notes

**Files:**
- None to create. This is a hands-on verification pass.

- [ ] **Step 1: Run the full automated suite**

```bash
npm test
cd src-tauri && cargo test --quiet && cd ..
npx tsc --noEmit
```

Expected: all green.

- [ ] **Step 2: Launch the app in dev mode**

```bash
npm run tauri dev
```

Wait for the window to open. Then, for EACH of the 10 presets:

1. Open Settings → 테마 → click the preset card.
2. Visit every view: 프로젝트 / 캘린더 / 메모 / 명령 팔레트 (Cmd+K) / Quick Capture (global shortcut).
3. Eyeball: text readable on surface-0 and surface-1, selected/hover states visible, calendar today-highlight + event pills readable.
4. Note any low-contrast tokens on a scratch list.

Then for custom:

1. Pick a "different" brand (e.g., `#e91e63`, `#14b8a6`, `#fbbf24`) in both dark and light base modes.
2. Eyeball the same views.

Then persistence:

1. Pick Midnight. Quit the app (Cmd+Q). Relaunch via `npm run tauri dev`.
2. Verify the app reopens in Midnight.

Then Quick Capture sync:

1. With the main window open, trigger Quick Capture (global shortcut) so its window is visible.
2. In the main window, switch theme → confirm the Quick Capture window updates within one frame (no reload).

- [ ] **Step 3: Bump version and update CHANGELOG**

Update `package.json` `version` from `0.6.0` to `0.7.0`. Add a CHANGELOG entry if a `CHANGELOG.md` exists — otherwise skip (no existing file verified).

```bash
grep -l '"version": "0.6.0"' package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
```

For each file the grep reports, bump `0.6.0` → `0.7.0` using Edit.

- [ ] **Step 4: Commit the verification + version bump**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: 0.7.0 — theme customization"
```

- [ ] **Step 5: Final sanity check**

```bash
npm run build
```

Expected: Vite build succeeds with no TypeScript errors.

---

## Self-review result

(Checklist run after the plan was written.)

- **Spec coverage:** every spec section maps to at least one task — types/presets (Task 1), derive (Task 2), applyTheme (Task 3), theme.css (Task 4), Tauri commands (Task 5), api wrappers (Task 6), ThemeContext (Task 7), pre-paint + App wrap (Task 8), SettingsThemeSection + debounce + revert (Task 9), SettingsDialog tab (Task 10), Quick Capture sync (Task 11), manual QA + version bump (Task 12).
- **Placeholder scan:** no TBDs, no "add appropriate handling", every code step shows the code.
- **Type consistency:** `ThemeSetting`, `PresetId`, `ThemeTokens`, `CUSTOM_STYLE_ID`, `THEME_LS_KEY`, `THEME_EVENT`, `applyTheme`, `deriveBrand`, `composeCustomTokens` — names are consistent across every task that references them.
- **Spec-to-plan deviation:** the spec calls the `<style>` tag id `hearth-custom-theme` — plan matches. Spec's debounce 300ms, alpha 0.18, lightness clamp 45–75% — all preserved in Task 2 + Task 9. Spec stores theme as a tagged union — Rust persists the JSON opaquely in Task 5, preserving the schema on the frontend.
