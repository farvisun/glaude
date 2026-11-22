<script lang="ts">
	// A table header cell that sorts its column (click again to flip).
	import ArrowUp from "@lucide/svelte/icons/arrow-up";
	import ArrowDown from "@lucide/svelte/icons/arrow-down";

	let {
		label,
		active,
		desc,
		onclick,
		end = false,
		class: cls = "",
	}: { label: string; active: boolean; desc: boolean; onclick: () => void; end?: boolean; class?: string } = $props();
</script>

<th class="font-normal {end ? 'text-end' : 'text-start'} {cls}" aria-sort={active ? (desc ? "descending" : "ascending") : "none"}>
	<button class="hover:text-foreground inline-flex items-center gap-1 {active ? 'text-foreground' : ''} {end ? 'flex-row-reverse' : ''}" {onclick}>
		{label}
		{#if active}{#if desc}<ArrowDown class="size-3" />{:else}<ArrowUp class="size-3" />{/if}{:else}<span class="size-3"></span>{/if}
	</button>
</th>
