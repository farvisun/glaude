// Svelte transitions that follow Settings → Appearance → Animations and the
// system's reduced-motion setting (duration 0 when either says no).
import { cubicOut } from "svelte/easing";
import { fade as fadeT, fly as flyT, slide as slideT, type FadeParams, type FlyParams, type SlideParams } from "svelte/transition";
import { prefs } from "$lib/state/prefs.svelte";

const reduced = () => typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

export const motionOn = () => prefs.motion && !reduced();

/** `ms`, or 0 when motion is off. */
export const dur = (ms: number) => (motionOn() ? ms : 0);

export function slide(node: Element, p: SlideParams = {}) {
	return slideT(node, { easing: cubicOut, ...p, duration: dur(p.duration ?? 200) });
}

export function fade(node: Element, p: FadeParams = {}) {
	return fadeT(node, { ...p, duration: dur(p.duration ?? 150) });
}

export function fly(node: Element, p: FlyParams = {}) {
	return flyT(node, { easing: cubicOut, y: 6, ...p, duration: dur(p.duration ?? 200) });
}
