# App Preview Placeholders

Placeholder images for `composition.html`. Replace each with a real capture from
the seed-screenshots run (`scripts/seed-screenshots.ts` per
`docs/superpowers/specs/2026-04-26-app-store-submission-design.md` §5.1).

All captures should be 1920x1080 (or wider, cropped to 1440x720 for the inline
frame slots). Dark mode only. No real personal data — use only the seeded demo
fixtures.

| File | Replaces / shows | Notes |
|------|------------------|-------|
| `cut-2.png` | Hearth **Projects** tab with seeded demo data (5 projects, amber accents visible) | Cut 2, 2.0–6.0s. Hero shot of the unified workspace. |
| `cut-3.png` | Hearth **Memos** tab — markdown rendering with headings + checklist + code block | Cut 3, 6.0–10.0s. Use a memo whose body shows clean typography. |
| `cut-4.png` | Hearth **Schedule / Calendar** view, week or month with ~8 demo events | Cut 4, 10.0–14.0s. Make sure today's column is visible with multiple events. |
| `cut-5-hearth.png` | Hearth Projects tab right after a `hearth-cli project add` write — the new project sits as the top row | Cut 5 (hero), 14.0–24.0s. The composition's amber `new-row-glow` overlay sweeps across the row at ~38% from the top of the frame, so frame the screenshot with the new project row in roughly that vertical band. |

## Optional swap-ins

If you want the hero terminal pane (left side of cut 5) to be a real screenshot
of Claude Code instead of the hand-rendered HTML terminal, drop a file at
`placeholders/cut-5-cc.png` and replace the `<div class="pane terminal">` block
in `composition.html` with an `<img>` tag pointing at it. The hand-rendered
version is preferred because it lets us animate the typed prompt + tool-call
cascade deterministically.

## Capture recipe (suggested)

1. Boot a clean dev build with the seeded fixture DB.
2. Resize the Hearth window to 1920x1200 with the title bar visible.
3. Cmd+Shift+4 + Spacebar to capture the window with alpha preserved.
4. Save into this directory using the filenames above.
5. Re-run `npx hyperframes preview` from `assets/app-preview/` to verify.
