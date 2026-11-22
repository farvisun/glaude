<script lang="ts">
	// A Workflow run: phases with their agents, each with live state, model,
	// tokens and what it is doing or returned. Stop while running.
	import type { Item, TaskInfo, WorkflowAgent } from "$lib/types";
	import { ticker } from "$lib/state/ticker.svelte";
	import { fmtDuration, fmtTokens } from "$lib/format";
	import TaskStatus from "./TaskStatus.svelte";
	import Workflow from "@lucide/svelte/icons/workflow";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Square from "@lucide/svelte/icons/square";

	type Tool = Extract<Item, { kind: "tool" }>;
	let { item, task, color, onStop }: { item: Tool; task?: TaskInfo; color: string; onStop?: (taskId: string) => void } = $props();

	let showScript = $state(false);
	let expanded = $state<Record<number, boolean>>({});

	const input = $derived((item.input ?? {}) as { script?: string; description?: string });
	const name = $derived(task?.workflowName ?? input.description ?? "Workflow");
	const status = $derived(task?.status ?? (item.status === "running" || item.status === "pending" ? "running" : item.status === "error" ? "failed" : "completed"));
	const running = $derived(status === "running" || status === "pending");
	const agents = $derived(task?.agents ?? []);
	const done = $derived(agents.filter((a) => a.state === "done").length);
	const failed = $derived(agents.filter((a) => a.state === "error").length);

	const groups = $derived.by(() => {
		const phases = task?.phases ?? [];
		const out: { title: string; index: number; agents: WorkflowAgent[] }[] = phases.map((p) => ({ title: p.title, index: p.index, agents: [] }));
		const loose: WorkflowAgent[] = [];
		for (const a of agents) {
			const g = out.find((p) => p.index === a.phaseIndex);
			if (g) g.agents.push(a);
			else loose.push(a);
		}
		if (loose.length) out.push({ title: phases.length ? "Other" : "", index: -1, agents: loose });
		return out;
	});

	$effect(() => {
		if (running) return ticker.subscribe();
	});
	const elapsed = $derived.by(() => {
		void ticker.frame;
		if (!task) return null;
		if (!running && task.usage?.durationMs) return task.usage.durationMs;
		return (task.endedAt ?? Date.now()) - task.startedAt;
	});

	const agentLine = (a: WorkflowAgent) =>
		a.error ?? (a.state === "done" ? a.resultPreview : a.lastToolSummary ?? (a.lastToolName ? `Using ${a.lastToolName}` : a.promptPreview)) ?? "";
	const shortModel = (m?: string) => m?.replace(/^claude-/, "").replace(/-\d{8}$/, "");
</script>

<div id="tool-{item.id}" class="border-border bg-card/40 overflow-hidden rounded-lg border text-[13px]" role="group" aria-label="Workflow {name}">
	<div class="flex min-w-0 items-center gap-2 px-3 py-2">
		<Workflow class="size-4 shrink-0" style="color: {color}" />
		<span class="shrink-0 font-medium">Workflow</span>
		<span class="text-muted-foreground min-w-0 truncate" dir="auto">{name}</span>
		<span class="text-muted-foreground ms-auto flex shrink-0 items-center gap-2 text-xs">
			{#if agents.length}<span>{done}/{agents.length} agents{failed ? ` · ${failed} failed` : ""}</span>{/if}
			{#if task?.usage?.tokens}<span>{fmtTokens(task.usage.tokens)} tok</span>{/if}
			{#if elapsed != null}<span>{fmtDuration(elapsed)}</span>{/if}
			<TaskStatus {status} {color} />
			{#if running && task && onStop}
				<button class="hover:text-foreground flex items-center gap-1" onclick={() => onStop?.(task.id)} aria-label="Stop workflow">
					<Square class="size-3 fill-current" /> Stop
				</button>
			{/if}
		</span>
	</div>
	{#if groups.length}
		<div class="border-border flex flex-col gap-2 border-t px-3 py-2">
			{#each groups as g (g.index)}
				<div>
					{#if g.title}<div class="text-muted-foreground pb-1 text-[11px] font-medium tracking-wide uppercase">{g.title}</div>{/if}
					<div class="flex flex-col">
						{#each g.agents as a (a.index)}
							<div class="hover:bg-accent/40 rounded-md">
								<button class="flex w-full min-w-0 items-center gap-2 px-1.5 py-1 text-start" onclick={() => (expanded[a.index] = !expanded[a.index])}>
									<TaskStatus status={a.state} {color} size={13} />
									<span class="shrink-0 font-medium">{a.label}</span>
									{#if a.agentType}<span class="bg-muted shrink-0 rounded px-1.5 py-px text-[11px]">{a.agentType}</span>{/if}
									<span class="text-muted-foreground min-w-0 flex-1 truncate text-xs {a.error ? 'text-destructive' : ''}" dir="auto">{agentLine(a)}</span>
									<span class="text-muted-foreground shrink-0 text-[11px]">
										{shortModel(a.model) ?? ""}{a.tokens ? ` · ${fmtTokens(a.tokens)}` : ""}{a.durationMs ? ` · ${fmtDuration(a.durationMs)}` : ""}
									</span>
								</button>
								{#if expanded[a.index]}
									<div class="text-muted-foreground flex flex-col gap-1 px-7 pb-2 text-xs">
										{#if a.promptPreview}<div><span class="text-foreground/80">Prompt:</span> <span dir="auto" class="whitespace-pre-wrap">{a.promptPreview}</span></div>{/if}
										{#if a.resultPreview}<div><span class="text-foreground/80">Result:</span> <span dir="auto" class="whitespace-pre-wrap">{a.resultPreview}</span></div>{/if}
										{#if a.error}<div class="text-destructive whitespace-pre-wrap">{a.error}</div>{/if}
										{#if a.toolCalls}<div>{a.toolCalls} tool calls</div>{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else if running}
		<div class="text-muted-foreground border-border border-t px-3 py-2 text-xs">Starting agents…</div>
	{/if}
	{#if task?.report && !running}
		<p class="text-muted-foreground border-border border-t px-3 py-2 text-xs whitespace-pre-wrap" dir="auto">{task.report}</p>
	{/if}
	{#if input.script}
		<button class="text-muted-foreground hover:text-foreground border-border flex w-full items-center gap-1 border-t px-3 py-1.5 text-xs" onclick={() => (showScript = !showScript)}>
			<ChevronRight class="size-3 transition-transform {showScript ? 'rotate-90' : ''}" /> Script
		</button>
		{#if showScript}
			<pre class="bg-code selectable scrollbar-thin max-h-72 overflow-auto px-3 py-2 font-mono text-[12px] whitespace-pre" dir="ltr">{input.script}</pre>
		{/if}
	{/if}
</div>
