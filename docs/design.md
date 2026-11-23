# Design tokens

Since 2026-11 Glaude follows a flat style close to openrouter.ai. The
tokens are CSS variables that components use through Tailwind
(`ui/src/app.css`); their values come from the active theme
(`ui/src/lib/themes`, see *Themes* below).

## Surfaces and colour

- Palette **Sade** is the default: near-white or near-black grounds, cards a
  step lighter, and greys that are tints of the text colour (`--border` is
  the text colour at 10%, `--input` at 14–15%, hovers at 6–7.5%).
- The primary is monochrome (black on light, white on dark). Links use
  `--link` (blue). Status colours stay separate: `--attention` (amber),
  `--ok` (green), `--destructive` (red), and the tool colours `--claude` and
  `--codex`.
- **Lajevard** and **Dood** keep their own exact colours on the same
  structure (light and dark each).
- Surfaces are flat with a 1px hairline. Shadows only on floating layers:
  `shadow-float` (menus, popovers, selects) and `shadow-dialog` (dialogs).
  No gradients, glows or blur.

## Shape and size

- Radius: `sm` 4px, `md` 6px, `lg` 8px, `xl` 12px.
- Control heights: `sm` 32px, default 36px, `lg` 40px for buttons, inputs and
  selects. Icon buttons in bars (tab strip, sidebar footer, statusline) are
  24–28px.
- Bottom bars share `--bar-h` (32px) so the sidebar footer and the statusline
  line up; tab rows use `--tab-h`.

## Type

- Inter (bundled, variable) for Latin, Estedad for Persian; the
  Arabic-script ranges always come from Estedad. Code uses the code font from
  Settings.
- No negative letter spacing on Persian text: it breaks letter joining.

## Menus

- Items in dropdowns, selects and context menus are one line (32px). Long
  text ends in "…"; extra detail sits on the same line in muted text or in a
  tooltip, never on a second line.

## Motion

- Durations `--dur-1` 120ms, `--dur-2` 180ms, `--dur-3` 240ms, easing
  `--motion-ease` (ease-out). Small distances (4–6px).
- What moves: the sidebar when shown or hidden with a button or shortcut, the
  terminal panel, the progress panel, project groups opening, new tabs and a
  tab's content when it is shown, plus the menus and dialogs of the kit.
- Settings → Appearance → Animations turns it off (`data-motion="off"` on
  the root), and the system's reduced-motion setting does too. Loading
  indicators keep moving. Svelte transitions go through `ui/src/lib/motion.ts`,
  which returns duration 0 when motion is off.

## Themes

- A theme (`ui/src/lib/themes/types.ts`) gives a ground, text, accent, four
  status colours (red, green, yellow, blue), optional surfaces (cards, menus,
  sidebar, code and terminal grounds, secondary text), 10 code colours and the
  16 terminal colours. `themeVars()` turns it into every token: greys are
  tints of the text colour, missing surfaces are derived, and status colours,
  links and secondary text are nudged until they read on the ground.
  Sade, Lajevard and Dood carry their exact values in `vars`.
- Presets (`presets.ts`): Sade, Lajevard, Dood, GitHub, One, Solarized,
  Dracula, Tokyo Night (and Storm), Nord, Catppuccin (Latte, Mocha), Rosé
  Pine (main, Moon, Dawn), Gruvbox and Everforest. A test checks every preset
  for readable text, secondary text, links and button labels.
- Settings → Appearance: a light and a dark theme (Mode picks one, System
  follows macOS), an accent on top of any theme, and separate chat and
  terminal themes ("Same as the app", the app's light or dark theme, or any
  theme). A chat or terminal theme that differs from the window's is an
  island: its root gets the theme's variables inline and `.dark`/`.light`, and
  the `dark:` variant follows the nearest of those.
- Code colours come from `--syn-*` (syntect scope classes), terminal and
  statusline-script colours from `--ansi-0`…`--ansi-15`.
- Your themes: *Customize…* on any theme or *New theme* starts the editor
  from it; *Import…* reads a VS Code theme (`.json`, comments allowed), an
  iTerm2 scheme (`.itermcolors`), a Ghostty or kitty theme, a Windows
  Terminal scheme (or its `settings.json`), or a theme copied with *Copy as
  JSON*. Terminal-only formats get the rest from their 16 colours. Saved in
  localStorage (`glaude.themes.v1`).
- The window's variables are cached (`glaude.theme.boot`) and applied in
  `main.ts` before the first paint, so a start never flashes Sade.
