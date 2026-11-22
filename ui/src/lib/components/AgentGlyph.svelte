<script lang="ts">
	// The status mark of an agent: ✻ working, a dot when it needs you,
	// a check when it finished, a warning after an error.
	import type { Tone } from "$lib/state/activity";
	import type { Provider } from "$lib/types";
	import Spark from "./conv/Spark.svelte";
	import Check from "@lucide/svelte/icons/check";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	let { tone, provider, class: className = "" }: { tone: Tone; provider: Provider; class?: string } = $props();
	const color = $derived(provider === "codex" ? "var(--codex)" : "var(--claude)");
</script>

<span class="grid size-4 shrink-0 place-items-center {className}" aria-hidden="true">
	{#if tone === "working"}
		<Spark {color} class="text-[13px]" />
	{:else if tone === "waiting"}
		<span class="waiting bg-attention size-2 rounded-full"></span>
	{:else if tone === "done"}
		<Check class="text-primary size-3.5" />
	{:else if tone === "error"}
		<TriangleAlert class="text-destructive size-3.5" />
	{:else}
		<span class="size-1.5 rounded-full" style="background: {color}"></span>
	{/if}
</span>

<style>
	.waiting {
		box-shadow: 0 0 0 0 color-mix(in oklab, var(--attention) 45%, transparent);
		animation: ping 1.8s ease-out infinite;
	}
	@keyframes ping {
		70% {
			box-shadow: 0 0 0 5px transparent;
		}
		100% {
			box-shadow: 0 0 0 0 transparent;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.waiting {
			animation: none;
		}
	}
</style>
