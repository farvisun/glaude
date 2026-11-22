<script lang="ts">
	// A folded run of tool calls: one line with what was done, the files
	// edited, and the live step while it runs. Click to see every call.
	import type { Snippet } from "svelte";
	import type { Item } from "$lib/types";
	import type { Conversation } from "$lib/state/conversations.svelte";
	import { runSummary, editedFiles, failures } from "$lib/state/runs";
	import { toolStep, fmtElapsed } from "$lib/state/activity";
	import { clock } from "$lib/state/clock.svelte";
	import { basename } from "$lib/format";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Check from "@lucide/svelte/icons/check";
	import XIcon from "@lucide/svelte/icons/x";
	import Layers from "@lucide/svelte/icons/layers";

	let {
		items,
		conv,
		open = $bindable(),
		row,
	}: { items: Item[]; conv: Conversation; open?: boolean; row: Snippet<[Item]> } = $props();
	const expanded = $derived(!!open);

	const live = $derived(
		[...items]
			.reverse()
			.find((i) => (i.kind === "tool" || i.kind === "command" || i.kind === "fileChange") && (i.status === "running" || i.status === "pending")),
	);
	const summary = $derived(runSummary(items));
	const files = $derived(editedFiles(items));
	const failed = $derived(failures(items));
	$effect(() => {
		if (live) return clock.subscribe();
	});
	// First start to last end, for calls timed in this window.
	const took = $derived.by(() => {
		const times = items.map((i) => conv.timeOf(i.id)).filter((t) => t != null);
		if (!times.length) return null;
		const start = Math.min(...times.map((t) => t.start));
		const end = live ? clock.now : Math.max(...times.map((t) => t.end ?? t.start));
		return end - start;
	});
</script>

<div class="run text-[13px]">
	<button
		class="hover:bg-accent/50 text-muted-foreground hover:text-foreground flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-1 text-start"
		onclick={() => (open = !expanded)}
		aria-expanded={expanded}
	>
		<Layers class="size-3.5 shrink-0" />
		<span class="text-foreground/90 min-w-0 truncate">{summary}</span>
		<span class="ms-auto flex shrink-0 items-center gap-1.5 text-[12px]">
			{#if took != null && took >= 1000}<span class="tabular-nums">{fmtElapsed(took)}</span>{/if}
			{#if live}
				<span class="tool-running" aria-label="Running">⏺&#xFE0E;</span>
			{:else if failed}
				<span class="text-destructive flex items-center gap-0.5"><XIcon class="size-3.5" />{failed}</span>
			{:else}
				<Check class="text-ok size-3.5" />
			{/if}
			<ChevronRight class="size-3.5 transition-transform {expanded ? 'rotate-90' : ''}" />
		</span>
	</button>
	{#if !expanded && live}
		<div class="text-muted-foreground truncate ps-7.5 text-[12px]" dir="auto">{toolStep(live)}…</div>
	{/if}
	{#if !expanded && files.length}
		<div class="flex flex-wrap gap-1.5 ps-7.5 pt-1">
			{#each files.slice(0, 8) as f (f.path)}
				<button
					class="border-border bg-card hover:bg-accent inline-flex max-w-60 items-center gap-1.5 rounded-md border px-1.5 py-0.5 font-mono text-[11.5px]"
					title={f.path}
					onclick={() => (open = true)}
				>
					<span class="truncate" dir="ltr">{basename(f.path)}</span>
					{#if f.added || f.removed}
						<span class="text-ok">+{f.added}</span><span class="text-destructive">−{f.removed}</span>
					{/if}
				</button>
			{/each}
			{#if files.length > 8}<span class="text-muted-foreground self-center text-[12px]">+{files.length - 8} more</span>{/if}
		</div>
	{/if}
	{#if expanded}
		<div class="border-border ms-3.5 mt-1 flex flex-col gap-1 border-s ps-2">
			{#each items as it (it.id)}
				{@render row(it)}
			{/each}
		</div>
	{/if}
</div>

<style>
	.tool-running {
		color: var(--muted-foreground);
		font-size: 11px;
		line-height: 1;
		font-variant-emoji: text;
		animation: run-blink 1s steps(1, end) infinite;
	}
	@keyframes run-blink {
		50% {
			opacity: 0.15;
		}
	}
</style>
