<script lang="ts">
	// ⌘F in a conversation: type to find, Enter/⇧Enter to step, Esc to close.
	import ChevronUp from "@lucide/svelte/icons/chevron-up";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import X from "@lucide/svelte/icons/x";
	import { onMount } from "svelte";

	let {
		query = $bindable(""),
		count,
		index,
		onStep,
		onClose,
	}: { query?: string; count: number; index: number; onStep: (dir: 1 | -1) => void; onClose: () => void } = $props();

	let input = $state<HTMLInputElement>();
	onMount(() => input?.select());
	export function focus() {
		input?.select();
	}
</script>

<div
	class="bg-popover text-popover-foreground border-border absolute end-4 top-2 z-30 flex items-center gap-1 rounded-lg border p-1 shadow-md"
	role="search"
>
	<input
		bind:this={input}
		bind:value={query}
		class="placeholder:text-muted-foreground h-7 w-52 bg-transparent px-2 text-[13px] outline-none"
		placeholder="Find in conversation"
		dir="auto"
		aria-label="Find in conversation"
		onkeydown={(e) => {
			if (e.key === "Enter") {
				e.preventDefault();
				onStep(e.shiftKey ? -1 : 1);
			} else if (e.key === "Escape") {
				e.preventDefault();
				e.stopPropagation();
				onClose();
			}
		}}
	/>
	<span class="text-muted-foreground min-w-12 text-center text-[12px] tabular-nums" aria-live="polite"
		>{query.trim() ? (count ? `${index + 1} of ${count}` : "None") : ""}</span
	>
	<button class="hover:bg-accent grid size-7 place-items-center rounded-md disabled:opacity-40" aria-label="Previous match" disabled={!count} onclick={() => onStep(-1)}
		><ChevronUp class="size-4" /></button
	>
	<button class="hover:bg-accent grid size-7 place-items-center rounded-md disabled:opacity-40" aria-label="Next match" disabled={!count} onclick={() => onStep(1)}
		><ChevronDown class="size-4" /></button
	>
	<button class="hover:bg-accent grid size-7 place-items-center rounded-md" aria-label="Close find" onclick={onClose}><X class="size-4" /></button>
</div>
