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
