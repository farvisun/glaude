# App icon

`icon-art.png` is the full-bleed artwork (1254×1254). Codex drew it with its
built-in image tool. `src-tauri/icons/icon-source.png` is that art fitted to
the macOS icon grid: an 824 px rounded square (radius 185) on a transparent
1024 canvas, with a soft shadow. Every platform size is generated from it.

Concept: a speech bubble drawn as a neural circuit, with a terminal prompt
chevron inside. It stands for chat, AI and a developer tool. Colours: royal
blue `#1f3cff`→`#0b1fa8`, white, and orange `#ff8a1f`.

## Rebuild

```sh
# 1. Fit the art to the macOS grid
in=docs/brand/icon-art.png; out=src-tauri/icons/icon-source.png; t=$(mktemp -d)
magick "$in" -resize 824x824^ -gravity center -extent 824x824 "$t/art.png"
magick -size 824x824 xc:none -fill white -draw "roundrectangle 0,0,823,823,185,185" "$t/mask.png"
magick "$t/art.png" "$t/mask.png" -alpha off -compose CopyOpacity -composite "$t/rounded.png"
magick "$t/rounded.png" -background black -shadow 30x12+0+10 "$t/shadow.png"
magick -size 1024x1024 xc:none \( "$t/shadow.png" \) -gravity center -geometry +0+8 -composite \
  "$t/rounded.png" -gravity center -composite "$out"; rm -rf "$t"

# 2. Generate all sizes (.icns, .ico, PNGs), then drop the mobile/Store extras
cd src-tauri && bunx @tauri-apps/cli@2 icon icons/icon-source.png
rm -rf icons/android icons/ios icons/Square*Logo.png icons/StoreLogo.png
```

## Prompt

Run with `codex exec` (Codex's image tool), square output:

> Subject: a speech bubble outline drawn as a glowing neural circuit (traces
> and nodes forming the bubble shape), with a terminal prompt chevron in its
> center and a small orb of light at one node. Palette: bold royal blue
> background (#1f3cff to #0b1fa8), circuit bubble in crisp white, chevron and
> orb in bright orange (#ff8a1f). Style: premium enterprise software app icon
> for an AI developer tool, polished modern macOS-style illustration with real
> depth: layered shapes, smooth gradients, soft studio lighting, subtle glass
> and glow, crisp clean edges. Composition: one dominant bold shape plus two
> or three supporting elements with a clear visual hierarchy, rich but not
> cluttered, still readable at 32 pixels. Canvas: Square 1024x1024,
> FULL-BLEED: the background fills the entire square edge to edge, no rounded
> corners, no frame, no border, no outer drop shadow, no device mockup. Keep
> the whole composition inside the central 62% of the canvas. Absolutely NO
> text, letters, numbers, logos of other companies, UI writing or watermarks
> anywhere.
