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

// The 13 CSS custom-property token names that the theme layer owns
// (4 surface + 2 border + 4 text + 3 brand). Priority, category, semantic,
// radius, shadow, motion, typography tokens stay global in @theme.
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
