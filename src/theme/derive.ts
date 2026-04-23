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
  const H = (((h % 360) + 360) % 360) / 360;
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
