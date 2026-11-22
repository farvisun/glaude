<script lang="ts">
	import type { Conversation } from "$lib/state/conversations.svelte";
	import type { Item, PlanStep, TurnState } from "$lib/types";
	import PlanCard from "./PlanCard.svelte";
	import { fmtDuration } from "$lib/format";
	import Bot from "@lucide/svelte/icons/bot";
	import Spark from "./Spark.svelte";
	import Check from "@lucide/svelte/icons/check";

	let { conv }: { conv: Conversation } = $props();

	let now = $state(Date.now());
	$effect(() => {
		if (!conv.turnStartedAt) return;
		now = Date.now();
		const t = setInterval(() => (now = Date.now()), 500);
		return () => clearInterval(t);
	});

	const stateLabel: Record<TurnState, string> = {
		idle: "Idle",
		starting: "Starting session",
		requesting: "Waiting for the model",
		thinking: "Thinking",
		writing: "Writing",
		tool: "Running a tool",
		compacting: "Compacting context",
		retrying: "Retrying",
		waitingPermission: "Waiting for your approval",
	};

	// Latest todo list: Codex plan items or Claude TodoWrite input.
	const todos = $derived.by((): { steps: PlanStep[]; explanation?: string } | null => {
		for (let i = conv.items.length - 1; i >= 0; i--) {
			const it = conv.items[i];
			if (it.kind === "plan") return { steps: it.steps, explanation: it.explanation };
			if (it.kind === "tool" && it.name === "TodoWrite") {
				const list = (it.input as { todos?: { content: string; status: string; activeForm?: string }[] })?.todos ?? [];
				return {
					steps: list.map((t) => ({
						text: t.status === "in_progress" && t.activeForm ? t.activeForm : t.content,
						status: t.status === "in_progress" ? "inProgress" : t.status,
					})),
				};
			}
		}
		return null;
	});

	const current = $derived.by(() => {
		for (let i = conv.items.length - 1; i >= 0; i--) {
			const it = conv.items[i];
			if ((it.kind === "tool" || it.kind === "command") && it.status === "running") return it;
		}
		return null;
	});

	type ToolItem = Extract<Item, { kind: "tool" }>;
	const agents = $derived.by(() => {
		const tasks = conv.items.filter((i): i is ToolItem => i.kind === "tool" && (i.name === "Task" || i.name === "Agent"));
		return tasks.map((t) => ({
			id: t.id,
			title: t.title,
			status: t.status,
			steps: conv.items.filter((i) => (i.kind === "tool" || i.kind === "assistant") && i.parent === t.id).length,
		}));
	});

	const sparkColor = $derived(conv.provider === "codex" ? "var(--codex)" : "var(--claude)");
	const elapsed = $derived(conv.turnStartedAt ? Math.max(0, now - conv.turnStartedAt) : null);
</script>

<aside class="border-border bg-sidebar/40 scrollbar-thin flex h-full w-full flex-col gap-5 overflow-y-auto border-s px-3 py-3 text-[13px]">
	<section>
		<h3 class="text-muted-foreground pb-1.5 text-xs font-medium">Now</h3>
		<div class="flex items-center gap-2">
			{#if conv.turn !== "idle"}
				<Spark color={sparkColor} class="text-[14px]" />
			{:else}
				<span class="bg-muted-foreground/50 size-2 rounded-full"></span>
			{/if}
			<span class={conv.turn === "waitingPermission" ? "text-attention font-medium" : ""}>{stateLabel[conv.turn]}</span>
			{#if elapsed != null}<span class="text-muted-foreground ms-auto">{fmtDuration(elapsed)}</span>{/if}
		</div>
		{#if conv.turnDetail}<div class="text-muted-foreground mt-1 truncate text-xs" dir="auto">{conv.turnDetail}</div>{/if}
		{#if current}
			<div class="text-muted-foreground mt-1 truncate font-mono text-xs" dir="ltr">
				{current.kind === "command" ? current.command : `${current.name} ${current.title}`}
			</div>
		{/if}
		{#if conv.turn === "idle" && conv.lastTurnMs}
			<div class="text-muted-foreground mt-1 text-xs">Last turn took {fmtDuration(conv.lastTurnMs)}</div>
		{/if}
	</section>

	<section>
		<h3 class="text-muted-foreground pb-1.5 text-xs font-medium">Plan</h3>
		{#if todos?.steps.length}
			<PlanCard steps={todos.steps} explanation={todos.explanation} compact />
		{:else}
			<p class="text-muted-foreground text-xs">The agent hasn't made a todo list in this conversation.</p>
		{/if}
	</section>

	{#if agents.length}
		<section>
			<h3 class="text-muted-foreground pb-1.5 text-xs font-medium">Subagents</h3>
			<ul class="flex flex-col gap-1.5">
				{#each agents as a (a.id)}
					<li class="flex items-center gap-2">
						<Bot class="text-muted-foreground size-3.5 shrink-0" />
						<span class="min-w-0 truncate" dir="auto">{a.title}</span>
						<span class="text-muted-foreground ms-auto shrink-0 text-xs">{a.steps}</span>
						{#if a.status === "running"}<Spark color={sparkColor} class="text-[12px]" />{:else}<Check
								class="text-ok size-3"
							/>{/if}
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</aside>
