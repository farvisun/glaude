<script lang="ts">
	// Plan mode finished (ExitPlanMode): the plan, rendered, with Approve
	// (auto-accept edits / ask before edits) or Keep planning with feedback.
	import type { Item } from "$lib/types";
	import { Button } from "$lib/components/ui/button";
	import Blocks from "./Blocks.svelte";
	import Map from "@lucide/svelte/icons/map";

	type Perm = Extract<Item, { kind: "permission" }>;
	let { item, onDecide }: { item: Perm; onDecide: (item: Perm, optionId: string, message?: string) => void } = $props();

	let feedback = $state("");
	let asking = $state(false);
	let open = $state(true);
	const pending = $derived(item.status === "pending");
	const planPath = $derived((item.input as { planFilePath?: string } | null)?.planFilePath);
	const verdict = $derived(
		item.status === "done" ? "Approved" : item.status === "declined" ? "Kept planning" : item.status === "interrupted" ? "Cancelled" : "",
	);
</script>

<div class="rounded-lg border text-[13px] {pending ? 'border-attention/70 bg-attention/8' : 'border-border bg-card/40'}" role="group" aria-label="Plan for review">
	<button class="flex w-full items-center gap-2 px-3 pt-2.5 pb-1.5 text-start" onclick={() => !pending && (open = !open)}>
		<Map class="size-4 shrink-0 {pending ? 'text-attention' : 'text-muted-foreground'}" />
		<span class="font-medium">{pending ? "Ready to code? Review the plan" : "Plan"}</span>
		{#if verdict}<span class="text-muted-foreground ms-auto text-xs">{verdict}</span>{/if}
	</button>
	{#if open || pending}
		<div class="border-border bg-background/60 selectable scrollbar-thin mx-3 mb-2 max-h-[28rem] overflow-y-auto rounded-md border px-3.5 py-2.5">
			<Blocks blocks={item.blocks ?? []} />
		</div>
		{#if planPath}<div class="text-muted-foreground mx-3 mb-2 truncate font-mono text-[11px]" dir="ltr">{planPath}</div>{/if}
	{/if}
	{#if pending}
		<div class="flex flex-col gap-2 px-3 pb-3">
			{#if asking}
				<textarea
					class="border-input bg-background min-h-16 rounded-md border px-2.5 py-1.5"
					dir="auto"
					placeholder="What should change in the plan?"
					bind:value={feedback}
					onkeydown={(e) => {
						if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && feedback.trim()) onDecide(item, "deny", feedback.trim());
					}}
				></textarea>
			{/if}
			<div class="flex flex-wrap items-center gap-1.5">
				{#if asking}
					<Button size="sm" disabled={!feedback.trim()} onclick={() => onDecide(item, "deny", feedback.trim())}>Send feedback</Button>
					<Button size="sm" variant="ghost" onclick={() => (asking = false)}>Back</Button>
				{:else}
					<Button size="sm" onclick={() => onDecide(item, "allow-accept-edits")}>Yes, auto-accept edits</Button>
					<Button size="sm" variant="secondary" onclick={() => onDecide(item, "allow-default")}>Yes, ask before edits</Button>
					<Button size="sm" variant="ghost" onclick={() => (asking = true)}>No, keep planning</Button>
				{/if}
			</div>
		</div>
	{/if}
</div>
