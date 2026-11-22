<script lang="ts">
	import type { Item, Provider, TaskInfo } from "$lib/types";
	import AgentCard from "./AgentCard.svelte";
	import WorkflowCard from "./WorkflowCard.svelte";
	import QuestionCard from "./QuestionCard.svelte";
	import PlanApprovalCard from "./PlanApprovalCard.svelte";
	import Undo2 from "@lucide/svelte/icons/undo-2";
	import Bot from "@lucide/svelte/icons/bot";
	import Blocks from "./Blocks.svelte";
	import ToolCard from "./ToolCard.svelte";
	import PermissionCard from "./PermissionCard.svelte";
	import PlanCard from "./PlanCard.svelte";
	import Brain from "@lucide/svelte/icons/brain";
	import Shimmer from "./Shimmer.svelte";
	import { fmtDuration } from "$lib/format";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	type Perm = Extract<Item, { kind: "permission" }>;
	let {
		item,
		providerLabel,
		provider = "claude",
		thoughtFor,
		onDecide,
		task,
		steps = [],
		onStopTask,
		onTaskOutput,
		onAnswer,
		onRewind,
	}: {
		item: Item;
		providerLabel: string;
		provider?: Provider;
		thoughtFor?: (id: string) => number | null;
		onDecide: (item: Perm | Question, optionId: string, message?: string) => void;
		/** The task a tool call started (subagent, background shell, workflow). */
		task?: TaskInfo;
		/** A subagent's own messages and tool calls. */
		steps?: Item[];
		onStopTask?: (taskId: string) => void;
		onTaskOutput?: (taskId: string) => void;
		onAnswer?: (item: Question, answers: Record<string, string>) => void;
		onRewind?: (item: Item) => void;
	} = $props();
	type Question = Extract<Item, { kind: "question" }>;

	let thinkingOpen = $state(false);
	const sparkColor = $derived(provider === "codex" ? "var(--codex)" : "var(--claude)");
	// Read when the item stops streaming (the item object changes then).
	const thoughtMs = $derived(item.kind === "reasoning" && !item.streaming ? (thoughtFor?.(item.id) ?? null) : null);
</script>

{#if item.kind === "user"}
	<div class="group/user flex items-start justify-end gap-1">
		{#if onRewind}
			<button
				class="text-muted-foreground hover:text-foreground hover:bg-accent mt-1.5 rounded-md p-1 opacity-0 transition-opacity group-hover/user:opacity-100 focus-visible:opacity-100"
				aria-label="Rewind to here"
				title="Rewind to before this message"
				onclick={() => onRewind?.(item)}><Undo2 class="size-3.5" /></button
			>
		{/if}
		<div class="bg-user-bubble selectable max-w-[85%] rounded-2xl rounded-ee-md px-4 py-2.5">
			<Blocks blocks={item.blocks} />
			{#if item.attachments.length}
				<div class="mt-2 flex flex-wrap gap-1.5">
					{#each item.attachments as a (a.path)}
						<span class="bg-background/60 text-muted-foreground inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-xs">
							<Paperclip class="size-3" />{a.name}
						</span>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{:else if item.kind === "assistant"}
	<div class="selectable {item.parent ? 'border-border ms-3 border-s ps-3' : ''}">
		<Blocks blocks={item.blocks} streaming={item.streaming} />
	</div>
{:else if item.kind === "reasoning"}
	<div class="text-muted-foreground">
		<button
			class="hover:text-foreground flex items-center gap-1.5 text-[13px] disabled:hover:text-inherit"
			disabled={item.blocks.length === 0}
			onclick={() => (thinkingOpen = !thinkingOpen)}
		>
			{#if item.streaming}
				<!-- The ✻ spinner lives on the turn line below; one per conversation. -->
				<Brain class="size-3.5" style="color: {sparkColor}" />
				<Shimmer color={sparkColor}>Thinking…</Shimmer>
			{:else}
				<Brain class="size-3.5" />
				<span>{thoughtMs != null ? `Thought for ${fmtDuration(Math.max(1000, thoughtMs))}` : "Thought"}</span>
			{/if}
			{#if item.blocks.length}
				<ChevronRight class="size-3.5 transition-transform {thinkingOpen ? 'rotate-90' : ''}" />
			{/if}
		</button>
		{#if thinkingOpen}
			<div class="border-border mt-1.5 ms-1.5 border-s ps-3 text-[13px] opacity-90">
				<Blocks blocks={item.blocks} />
			</div>
		{/if}
	</div>
{:else if item.kind === "tool" && (item.name === "Agent" || item.name === "Task")}
	<AgentCard {item} {task} {steps} color={sparkColor} onStop={onStopTask} />
{:else if item.kind === "tool" && item.name === "Workflow"}
	<WorkflowCard {item} {task} color={sparkColor} onStop={onStopTask} />
{:else if item.kind === "tool" || item.kind === "command" || item.kind === "fileChange"}
	<div class={item.kind === "tool" && item.parent ? "border-border ms-3 border-s ps-1" : ""}>
		<ToolCard {item} {task} onOutput={onTaskOutput} onStop={onStopTask} />
	</div>
{:else if item.kind === "permission" && item.tool === "ExitPlanMode"}
	<PlanApprovalCard {item} {onDecide} />
{:else if item.kind === "permission"}
	<PermissionCard {item} {providerLabel} {onDecide} />
{:else if item.kind === "question"}
	<QuestionCard
		{item}
		{providerLabel}
		onAnswer={(answers) => onAnswer?.(item, answers)}
		onSkip={() => onDecide(item, "deny", "The user skipped the question.")}
	/>
{:else if item.kind === "plan"}
	<PlanCard steps={item.steps} explanation={item.explanation} />
{:else if item.kind === "system"}
	{#if item.level === "compact"}
		<div class="text-muted-foreground flex items-center gap-3 py-1 text-xs">
			<span class="bg-border h-px flex-1"></span><span>{item.text}</span><span class="bg-border h-px flex-1"></span>
		</div>
	{:else if item.level === "task"}
		<div class="text-muted-foreground flex items-center gap-2 px-2.5 text-[13px]" dir="auto">
			<Bot class="size-3.5 shrink-0" style="color: {sparkColor}" />
			<span class="selectable min-w-0">{item.text}</span>
		</div>
	{:else}
		<div
			class="flex items-start gap-2 rounded-md px-2.5 py-1.5 text-[13px] {item.level === 'error'
				? 'bg-destructive/10 text-destructive'
				: item.level === 'warn'
					? 'bg-attention/10 text-attention'
					: 'text-muted-foreground'}"
			dir="auto"
		>
			{#if item.level === "error" || item.level === "warn"}<TriangleAlert class="mt-0.5 size-3.5 shrink-0" />{/if}
			<span class="selectable min-w-0">{item.text}</span>
		</div>
	{/if}
{/if}
