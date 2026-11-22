# Design tokens

Since 2026-11 Glaude follows a flat style close to openrouter.ai. The
tokens live in `ui/src/app.css`; components use them through Tailwind.

## Surfaces and colour

- Palette **Sade** is the default: near-white or near-black grounds, cards a
  step lighter, and greys that are tints of the text colour (`--border` is
  the text colour at 10%, `--input` at 14–15%, hovers at 6–7.5%).
- The primary is monochrome (black on light, white on dark). Links use
  `--link` (blue). Status colours stay separate: `--attention` (amber),
  `--ok` (green), `--destructive` (red), and the tool colours `--claude` and
  `--codex`.
- **Lajevard** and **Dood** remain as palettes with their own colours on the
  same structure. The first run with the new design moves Lajevard users to
  Sade once; picking Lajevard again sticks.
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
