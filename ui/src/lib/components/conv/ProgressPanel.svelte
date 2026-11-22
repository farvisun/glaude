<script lang="ts">
	import type { Conversation } from "$lib/state/conversations.svelte";
	import type { Item, PlanStep, TurnState } from "$lib/types";
	import PlanCard from "./PlanCard.svelte";
	import ActivityTimeline from "./ActivityTimeline.svelte";
	import { fmtDuration } from "$lib/format";
	import Bot from "@lucide/svelte/icons/bot";
	import Spark from "./Spark.svelte";
	import Check from "@lucide/svelte/icons/check";
	import Terminal from "@lucide/svelte/icons/terminal";
	import Workflow from "@lucide/svelte/icons/workflow";
	import Eye from "@lucide/svelte/icons/eye";
	import TaskStatus from "./TaskStatus.svelte";
	import { convActions } from "$lib/state/sessions";
	import { ui } from "$lib/state/ui.svelte";
	import type { TaskInfo } from "$lib/types";

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
	// Live tasks first (running, then newest), then agent calls from history.
	const tasks = $derived.by(() => {
		const live = Object.values(conv.tasks).sort((a, b) => {
			const ra = a.status === "running" || a.status === "pending" ? 0 : 1;
			const rb = b.status === "running" || b.status === "pending" ? 0 : 1;
			return ra - rb || b.startedAt - a.startedAt;
		});
		const known = new Set(live.map((t) => t.toolUseId));
		const past = conv.items
			.filter((i): i is ToolItem => i.kind === "tool" && (i.name === "Task" || i.name === "Agent") && !known.has(i.id))
			.map(
				(t): TaskInfo => ({
					id: t.id,
					toolUseId: t.id,
					kind: "agent",
					description: t.title,
					status: t.status === "running" ? "running" : t.status === "error" ? "failed" : "completed",
					background: false,
					startedAt: 0,
				}),
			);
		return [...live, ...past.reverse()].slice(0, 30);
	});

	const iconOf = (t: TaskInfo) => (t.kind === "shell" || t.kind === "monitor" ? Terminal : t.kind === "workflow" ? Workflow : Bot);
	const isLive = (t: TaskInfo) => !!conv.tasks[t.id];
	const running = (t: TaskInfo) => t.status === "running" || t.status === "pending";
	function reveal(t: TaskInfo) {
		const el = t.toolUseId ? document.getElementById(`tool-${t.toolUseId}`) : null;
		el?.scrollIntoView({ behavior: "smooth", block: "center" });
	}

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
		<h3 class="text-muted-foreground pb-1.5 text-xs font-medium">Timeline</h3>
		<ActivityTimeline {conv} />
	</section>

	<section>
		<h3 class="text-muted-foreground pb-1.5 text-xs font-medium">Plan</h3>
		{#if todos?.steps.length}
			<PlanCard steps={todos.steps} explanation={todos.explanation} compact />
		{:else}
			<p class="text-muted-foreground text-xs">The agent hasn't made a todo list in this conversation.</p>
		{/if}
	</section>

	<section>
		<h3 class="text-muted-foreground flex items-center pb-1.5 text-xs font-medium">
			Tasks
			{#if conv.runningTasks.length}<span class="ms-auto">{conv.runningTasks.length} running</span>{/if}
		</h3>
		{#if tasks.length}
			<ul class="flex flex-col gap-1">
				{#each tasks as t (t.id)}
					{@const Icon = iconOf(t)}
					<li class="group/task hover:bg-accent/40 flex flex-col rounded-md px-1.5 py-1">
						<button class="flex min-w-0 items-center gap-2 text-start" onclick={() => reveal(t)}>
							<Icon class="text-muted-foreground size-3.5 shrink-0" />
							<span class="min-w-0 truncate" dir="auto">{t.description || t.subagentType || t.kind}</span>
							<span class="ms-auto shrink-0"><TaskStatus status={t.status} color={sparkColor} size={12} /></span>
						</button>
						<div class="text-muted-foreground flex items-center gap-2 ps-5.5 text-[11px]">
							<span class="min-w-0 truncate" dir="auto">
								{#if running(t)}{t.activity ?? (t.lastTool ? `Using ${t.lastTool}` : t.subagentType ?? t.kind)}{:else}{t.subagentType ??
										t.workflowName ??
										t.kind}{t.endedAt && t.startedAt ? ` · ${fmtDuration(t.endedAt - t.startedAt)}` : ""}{/if}
							</span>
							{#if isLive(t)}
								<span class="ms-auto flex shrink-0 gap-2 opacity-0 transition-opacity group-hover/task:opacity-100 focus-within:opacity-100">
									{#if t.kind === "shell" || t.kind === "monitor"}
										<button class="hover:text-foreground" aria-label="Show output" onclick={() => (ui.taskOutput = { convId: conv.id, taskId: t.id })}
											><Eye class="size-3" /></button
										>
									{/if}
									{#if running(t)}
										<button class="hover:text-foreground" onclick={() => convActions.stopTask(conv, t.id)}>Stop</button>
									{/if}
								</span>
							{/if}
						</div>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="text-muted-foreground text-xs">Subagents, background shells and workflows show up here.</p>
		{/if}
	</section>
</aside>
