<script lang="ts" generics="T extends string">
	// A small "Sort: …" menu for a list.
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import ArrowUpDown from "@lucide/svelte/icons/arrow-up-down";
	import Check from "@lucide/svelte/icons/check";

	let { value, options, onChange, label = "Sort" }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label?: string } = $props();
	const current = $derived(options.find((o) => o.value === value)?.label ?? "");
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class="text-muted-foreground hover:text-foreground hover:bg-accent inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-[12px]"
		aria-label="{label}: {current}"
		><ArrowUpDown class="size-3.5" /><span class="truncate">{current}</span></DropdownMenu.Trigger
	>
	<DropdownMenu.Content align="end" class="w-48">
		<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">{label}</DropdownMenu.Label>
		{#each options as o (o.value)}
			<DropdownMenu.Item onclick={() => onChange(o.value)}><Check class={o.value === value ? "" : "invisible"} /><span>{o.label}</span></DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
