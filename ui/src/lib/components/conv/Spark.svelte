<script lang="ts">
	// The spinner Claude Code uses: an asterisk that grows and shrinks
	// (· ✢ ✳ ✶ ✻ ✽ and back).
	import { ticker, reducedMotion } from "$lib/state/ticker.svelte";

	let {
		color = "var(--claude)",
		class: className = "",
		still = false,
	}: { color?: string; class?: string; still?: boolean } = $props();

	// U+FE0E keeps ✳ from turning into a coloured emoji.
	const FRAMES = ["·", "✢", "✳︎", "✶", "✻", "✽"];
	const SEQUENCE = [...FRAMES, ...FRAMES.slice(1, -1).reverse()];

	const animate = $derived(!still && !reducedMotion());
	$effect(() => {
		if (animate) return ticker.subscribe();
	});
	const glyph = $derived(animate ? SEQUENCE[ticker.frame % SEQUENCE.length] : "✻");
</script>

<span class="spark {className}" style:color aria-hidden="true">{glyph}</span>

<style>
	.spark {
		display: inline-grid;
		place-items: center;
		width: 1.15em;
		height: 1em;
		flex-shrink: 0;
		font-family: ui-monospace, "SF Mono", Menlo, "Apple Symbols", "Segoe UI Symbol", "DejaVu Sans Mono", monospace;
		font-variant-emoji: text;
		line-height: 1;
	}
</style>
