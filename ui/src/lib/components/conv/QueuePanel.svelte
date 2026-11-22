<script lang="ts">
	// Messages waiting for the current turn to end: edit, remove, or send now.
	import type { QueuedMessage } from "$lib/types";
	import ListOrdered from "@lucide/svelte/icons/list-ordered";
	import Pencil from "@lucide/svelte/icons/pencil";
	import X from "@lucide/svelte/icons/x";
	import CornerDownLeft from "@lucide/svelte/icons/corner-down-left";
	import Paperclip from "@lucide/svelte/icons/paperclip";

	let {
		items,
		onChange,
		onSendNow,
	}: { items: QueuedMessage[]; onChange: (items: QueuedMessage[]) => void; onSendNow: (m: QueuedMessage) => void } = $props();

	let editing = $state<string | null>(null);
	let draft = $state("");

	function save(id: string) {
		const text = draft.trim();
		onChange(text ? items.map((m) => (m.id === id ? { ...m, text } : m)) : items.filter((m) => m.id !== id));
		editing = null;
	}
</script>

{#if items.length}
	<div class="border-border bg-card/60 mb-1.5 rounded-lg border text-[13px]" role="list" aria-label="Queued messages">
		<div class="text-muted-foreground flex items-center gap-1.5 px-3 pt-1.5 text-xs">
			<ListOrdered class="size-3.5" />
			{items.length} queued · sent when the turn ends
		</div>
		{#each items as m, i (m.id)}
			<div class="group/q flex items-start gap-2 px-3 py-1.5" role="listitem">
				<span class="text-muted-foreground mt-0.5 w-4 shrink-0 text-xs tabular-nums">{i + 1}</span>
				{#if editing === m.id}
					<textarea
						class="border-input bg-background min-h-9 flex-1 resize-y rounded-md border px-2 py-1"
						dir="auto"
						bind:value={draft}
						onkeydown={(e) => {
							if (e.key === "Enter" && !e.shiftKey) {
								e.preventDefault();
								save(m.id);
							} else if (e.key === "Escape") editing = null;
						}}
					></textarea>
				{:else}
					<span class="line-clamp-2 min-w-0 flex-1 whitespace-pre-wrap" dir="auto">{m.text}</span>
					{#if m.attachments?.length}<Paperclip class="text-muted-foreground mt-0.5 size-3 shrink-0" />{/if}
				{/if}
				<span class="text-muted-foreground flex shrink-0 items-center gap-0.5 opacity-60 group-hover/q:opacity-100">
					<button class="hover:text-foreground hover:bg-accent rounded p-1" aria-label="Send now" title="Send now (into the running turn)" onclick={() => onSendNow(m)}
						><CornerDownLeft class="size-3.5" /></button
					>
					<button
						class="hover:text-foreground hover:bg-accent rounded p-1"
						aria-label="Edit"
						onclick={() => {
							editing = m.id;
							draft = m.text;
						}}><Pencil class="size-3.5" /></button
					>
					<button class="hover:text-foreground hover:bg-accent rounded p-1" aria-label="Remove" onclick={() => onChange(items.filter((x) => x.id !== m.id))}
						><X class="size-3.5" /></button
					>
				</span>
			</div>
		{/each}
	</div>
{/if}
