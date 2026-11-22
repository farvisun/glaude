<script lang="ts">
	// Text with a highlight sweeping across it, as Claude shows while it works.
	import type { Snippet } from "svelte";

	let {
		color = "currentColor",
		class: className = "",
		children,
	}: { color?: string; class?: string; children: Snippet } = $props();
</script>

<span class="shimmer {className}" style:--shimmer-color={color}>{@render children()}</span>

<style>
	.shimmer {
		--shimmer-hi: color-mix(in oklab, var(--shimmer-color), white var(--shimmer-lift, 30%));
		background: linear-gradient(
			90deg,
			var(--shimmer-color) 0%,
			var(--shimmer-color) 44%,
			var(--shimmer-hi) 50%,
			var(--shimmer-color) 56%,
			var(--shimmer-color) 100%
		);
		background-size: 300% 100%;
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
		animation: shimmer 2.4s linear infinite;
	}
	@keyframes shimmer {
		from {
			background-position: 100% 0;
		}
		to {
			background-position: 0% 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.shimmer {
			animation: none;
			background: none;
			color: var(--shimmer-color);
		}
	}
</style>
