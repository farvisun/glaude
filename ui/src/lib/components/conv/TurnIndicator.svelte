<script lang="ts">
	// The working line under the conversation, Claude Code style:
	//   ✻ Pondering… (12s · esc to interrupt)
	import type { Conversation } from "$lib/state/conversations.svelte";
	import { ticker } from "$lib/state/ticker.svelte";
	import { fmtDuration } from "$lib/format";
	import Spark from "./Spark.svelte";
	import Shimmer from "./Shimmer.svelte";

	let { conv }: { conv: Conversation } = $props();

	const VERBS = [
		"Thinking", "Pondering", "Musing", "Mulling", "Brewing", "Simmering", "Percolating",
		"Noodling", "Tinkering", "Crafting", "Weaving", "Untangling", "Distilling", "Conjuring",
		"Synthesizing", "Deliberating", "Ruminating", "Cogitating", "Puzzling", "Sketching",
		"Composing", "Wrangling", "Calibrating", "Reckoning", "Marinating", "Polishing",
	];

	// One verb per turn, picked from when the turn started.
	const verb = $derived(VERBS[Math.abs(Math.floor((conv.turnStartedAt ?? 0) / 7)) % VERBS.length]);

	const label = $derived.by(() => {
		switch (conv.turn) {
			case "starting":
				return conv.turnDetail ?? "Starting";
			case "compacting":
				return "Compacting conversation";
			case "retrying":
				return conv.turnDetail ?? "Retrying";
			default:
				return verb;
		}
	});

	$effect(() => ticker.subscribe());
	const elapsed = $derived.by(() => {
		void ticker.frame;
		return conv.turnStartedAt ? Math.max(0, Date.now() - conv.turnStartedAt) : null;
	});

	const meta = $derived([elapsed != null ? fmtDuration(elapsed) : null, "esc to interrupt"].filter(Boolean).join(" · "));

	const color = $derived(conv.provider === "codex" ? "var(--codex)" : "var(--claude)");
</script>

<div class="flex items-center gap-1.5 text-[13px]" role="status" aria-live="polite">
	<Spark {color} class="text-[15px]" />
	<Shimmer {color} class="font-medium">{label}…</Shimmer>
	<span class="text-muted-foreground">({meta})</span>
</div>
