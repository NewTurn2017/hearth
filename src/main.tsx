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
