// Small colour helpers for themes: parse, mix, contrast. sRGB only.

export type RGB = [number, number, number];

/** "#abc", "#aabbcc", "#aabbccdd", "aabbcc", "rgb(1 2 3)" → [r, g, b] (0–255), or null. */
export function parse(input: string | undefined | null): RGB | null {
	if (!input) return null;
	const s = input.trim().toLowerCase();
	let m = /^#?([0-9a-f]{3,4})$/.exec(s);
	if (m) {
		const h = m[1];
		return [0, 1, 2].map((i) => parseInt(h[i] + h[i], 16)) as RGB;
	}
	m = /^#?([0-9a-f]{6})([0-9a-f]{2})?$/.exec(s);
	if (m) {
		const h = m[1];
		return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) as RGB;
	}
	m = /^rgba?\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)/.exec(s);
	if (m) return [Number(m[1]), Number(m[2]), Number(m[3])];
	return null;
}

export const isColor = (s: string | undefined | null) => parse(s) !== null;

const clamp = (n: number) => Math.max(0, Math.min(255, Math.round(n)));

export function hex(c: RGB): string {
	return "#" + c.map((n) => clamp(n).toString(16).padStart(2, "0")).join("");
}

/** Normalise any accepted colour to "#rrggbb" (or the fallback). */
export function norm(input: string | undefined | null, fallback = "#000000"): string {
	const c = parse(input);
	return c ? hex(c) : fallback;
}

/** `a` moved towards `b` by `t` (0 = a, 1 = b). */
export function mix(a: string, b: string, t: number): string {
	const x = parse(a) ?? [0, 0, 0];
	const y = parse(b) ?? [0, 0, 0];
	return hex([0, 1, 2].map((i) => x[i] + (y[i] - x[i]) * t) as RGB);
}

/** "rgb(r g b / a)" for tints that sit on any surface. */
export function alpha(c: string, a: number): string {
	const [r, g, b] = parse(c) ?? [0, 0, 0];
	return `rgb(${r} ${g} ${b} / ${a})`;
}

export function luminance(c: string): number {
	const [r, g, b] = (parse(c) ?? [0, 0, 0]).map((v) => {
		const s = v / 255;
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
	});
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
	const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
	return (x + 0.05) / (y + 0.05);
}

export const isDark = (c: string) => luminance(c) < 0.18;

/** Text colour for a filled surface: whichever of the two reads better. */
export function textOn(bg: string, light = "#ffffff", dark = "#0b0c0f"): string {
	return contrast(bg, light) >= contrast(bg, dark) ? light : dark;
}

/** Moves `c` towards `towards` until it reaches `min` contrast on `bg`. */
export function legible(c: string, bg: string, min: number, towards: string): string {
	let out = norm(c);
	for (let i = 1; i <= 10 && contrast(out, bg) < min; i++) out = mix(c, towards, i / 10);
	return out;
}
