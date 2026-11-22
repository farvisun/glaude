<script lang="ts">
	import type { Item } from "$lib/types";
	import Blocks from "./Blocks.svelte";
	import ToolCard from "./ToolCard.svelte";
	import PermissionCard from "./PermissionCard.svelte";
	import PlanCard from "./PlanCard.svelte";
	import Brain from "@lucide/svelte/icons/brain";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	type Perm = Extract<Item, { kind: "permission" }>;
	let {
		item,
		providerLabel,
		onDecide,
	}: { item: Item; providerLabel: string; onDecide: (item: Perm, optionId: string, message?: string) => void } = $props();

	let thinkingOpen = $state(false);
</script>

{#if item.kind === "user"}
	<div class="flex justify-end">
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
		<button class="hover:text-foreground flex items-center gap-1.5 text-[13px]" onclick={() => (thinkingOpen = !thinkingOpen)}>
			<Brain class="size-3.5 {item.streaming ? 'text-primary animate-pulse' : ''}" />
			<span>{item.streaming ? "Thinking…" : "Thought"}</span>
			<ChevronRight class="size-3.5 transition-transform {thinkingOpen ? 'rotate-90' : ''}" />
		</button>
		{#if thinkingOpen}
			<div class="border-border mt-1.5 ms-1.5 border-s ps-3 text-[13px] opacity-90">
				<Blocks blocks={item.blocks} />
			</div>
		{/if}
	</div>
{:else if item.kind === "tool" || item.kind === "command" || item.kind === "fileChange"}
	<div class={item.kind === "tool" && item.parent ? "border-border ms-3 border-s ps-1" : ""}>
		<ToolCard {item} />
	</div>
{:else if item.kind === "permission"}
	<PermissionCard {item} {providerLabel} {onDecide} />
{:else if item.kind === "plan"}
	<PlanCard steps={item.steps} explanation={item.explanation} />
{:else if item.kind === "system"}
	{#if item.level === "compact"}
		<div class="text-muted-foreground flex items-center gap-3 py-1 text-xs">
			<span class="bg-border h-px flex-1"></span><span>{item.text}</span><span class="bg-border h-px flex-1"></span>
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
