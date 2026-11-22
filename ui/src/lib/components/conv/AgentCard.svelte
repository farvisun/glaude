<script lang="ts">
	// A subagent (Agent/Task tool call) as one card: type, description, live
	// status, usage, the latest activity and, expanded, its own steps and report.
	import type { Item, TaskInfo } from "$lib/types";
	import { ticker } from "$lib/state/ticker.svelte";
	import { fmtDuration, fmtTokens } from "$lib/format";
	import Blocks from "./Blocks.svelte";
	import ToolCard from "./ToolCard.svelte";
	import TaskStatus from "./TaskStatus.svelte";
	import Shimmer from "./Shimmer.svelte";
	import Bot from "@lucide/svelte/icons/bot";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Square from "@lucide/svelte/icons/square";

	type Tool = Extract<Item, { kind: "tool" }>;
	let {
		item,
		task,
		steps,
		color,
		onStop,
	}: { item: Tool; task?: TaskInfo; steps: Item[]; color: string; onStop?: (taskId: string) => void } = $props();

	let open = $state(false);

	const input = $derived((item.input ?? {}) as { description?: string; subagent_type?: string; prompt?: string });
	const type = $derived(task?.subagentType ?? input.subagent_type ?? "general-purpose");
	const description = $derived(task?.description || input.description || item.title);
	const status = $derived.by(() => {
		if (task) return task.status;
		if (item.status === "running" || item.status === "pending") return "running";
		if (item.status === "done") return "completed";
		if (item.status === "error") return "failed";
		return "stopped";
	});
	const running = $derived(status === "running" || status === "pending");

	$effect(() => {
		if (running) return ticker.subscribe();
	});
	const elapsed = $derived.by(() => {
		void ticker.frame;
		if (task?.usage?.durationMs && !running) return task.usage.durationMs;
		if (task?.startedAt) return (task.endedAt ?? Date.now()) - task.startedAt;
		return null;
	});

	// Async launches answer with a placeholder; the real result is the report.
	const report = $derived.by(() => {
		if (task?.report) return task.report;
		const out = item.output ?? "";
		if (/^Async agent launched|async_launched/i.test(out)) return "";
		return status === "running" ? "" : out;
	});
	const stepCount = $derived(steps.filter((s) => s.kind === "tool").length);
	const activity = $derived(task?.activity ?? (task?.lastTool ? `Using ${task.lastTool}` : stepCount ? "Working" : "Starting"));
</script>

<div id="tool-{item.id}" class="border-border bg-card/40 overflow-hidden rounded-lg border text-[13px]" role="group" aria-label="Subagent {type}">
	<button class="hover:bg-accent/40 flex w-full min-w-0 items-center gap-2 px-3 py-2 text-start" onclick={() => (open = !open)}>
		<Bot class="size-4 shrink-0" style="color: {color}" />
		<span class="shrink-0 font-medium">{type}</span>
		<span class="text-muted-foreground min-w-0 truncate" dir="auto">{description}</span>
		<span class="text-muted-foreground ms-auto flex shrink-0 items-center gap-2 text-xs">
			{#if task?.background}<span class="bg-muted rounded px-1.5 py-px text-[11px]">background</span>{/if}
			{#if elapsed != null}<span>{fmtDuration(elapsed)}</span>{/if}
			{#if task?.usage?.tokens}<span>{fmtTokens(task.usage.tokens)} tok</span>{/if}
			{#if task?.usage?.toolUses || stepCount}
				{@const n = task?.usage?.toolUses ?? stepCount}
				<span>{n} tool{n === 1 ? "" : "s"}</span>
			{/if}
			<TaskStatus {status} {color} />
			<ChevronRight class="size-3.5 transition-transform {open ? 'rotate-90' : ''}" />
		</span>
	</button>
	{#if running}
		<div class="text-muted-foreground flex items-center gap-2 px-3 pb-2 text-xs">
			<Shimmer color="var(--muted-foreground)" class="min-w-0 truncate">{activity}…</Shimmer>
			{#if onStop && task}
				<button class="hover:text-foreground ms-auto flex items-center gap-1" onclick={() => onStop?.(task.id)}>
					<Square class="size-3 fill-current" /> Stop
				</button>
			{/if}
		</div>
	{:else if report && !open}
		<p class="text-muted-foreground line-clamp-2 px-3 pb-2 text-xs whitespace-pre-wrap" dir="auto">{report}</p>
	{/if}
	{#if open}
		<div class="border-border flex flex-col gap-1.5 border-t px-2 py-2">
			{#if input.prompt}
				<details class="px-1">
					<summary class="text-muted-foreground hover:text-foreground cursor-pointer text-xs">Prompt</summary>
					<pre class="bg-code border-border selectable scrollbar-thin mt-1 max-h-48 overflow-auto rounded-md border px-2.5 py-2 text-[12px] whitespace-pre-wrap" dir="auto">{input.prompt}</pre>
				</details>
			{/if}
			{#each steps as step (step.id)}
				{#if step.kind === "assistant"}
					<div class="text-muted-foreground px-2 text-[12.5px]"><Blocks blocks={step.blocks} /></div>
				{:else if step.kind === "tool"}
					<ToolCard item={step} />
				{/if}
			{:else}
				<div class="text-muted-foreground px-2 text-xs">{running ? "No steps yet." : "No steps recorded."}</div>
			{/each}
			{#if report}
				<div class="border-border mt-1 rounded-md border px-2.5 py-2">
					<div class="text-muted-foreground pb-1 text-xs font-medium">Report</div>
					<p class="selectable text-[13px] whitespace-pre-wrap" dir="auto">{report}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>
