<script lang="ts">
	// One theme in the gallery; the menu customises or manages it.
	import type { Theme } from "$lib/themes/types";
	import ThemePreview from "./ThemePreview.svelte";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import Check from "@lucide/svelte/icons/check";
	import Ellipsis from "@lucide/svelte/icons/ellipsis";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Copy from "@lucide/svelte/icons/copy";
	import Braces from "@lucide/svelte/icons/braces";
	import Trash from "@lucide/svelte/icons/trash-2";

	let {
		theme,
		selected,
		onSelect,
		onCustomize,
		onEdit,
		onCopyJson,
		onDelete,
	}: {
		theme: Theme;
		selected: boolean;
		onSelect: () => void;
		onCustomize: () => void;
		onEdit?: () => void;
		onCopyJson: () => void;
		onDelete?: () => void;
	} = $props();
</script>

<div class="group/card relative">
	<button
		class="flex w-full flex-col gap-1.5 rounded-lg border p-1.5 text-start transition-colors duration-(--dur-1) {selected
			? 'border-foreground/40 ring-foreground/15 ring-2'
			: 'border-border hover:border-foreground/25'}"
		aria-pressed={selected}
		onclick={onSelect}
		title={theme.name}
	>
		<ThemePreview {theme} />
		<span class="flex h-5 items-center gap-1 px-0.5 text-[12px]">
			<span class="min-w-0 truncate">{theme.name}</span>
			{#if selected}<Check class="ms-auto size-3.5 shrink-0" />{/if}
		</span>
	</button>
	<DropdownMenu.Root>
		<DropdownMenu.Trigger
			class="bg-background/85 text-foreground hover:bg-background absolute end-2.5 top-2.5 grid size-6 place-items-center rounded-md opacity-0 shadow-sm group-hover/card:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100 [@media(hover:none)]:opacity-100"
			aria-label="More for {theme.name}"><Ellipsis class="size-3.5" /></DropdownMenu.Trigger
		>
		<DropdownMenu.Content align="end" class="w-48">
			{#if onEdit}<DropdownMenu.Item onclick={onEdit}><Pencil /> Edit…</DropdownMenu.Item>{/if}
			<DropdownMenu.Item onclick={onCustomize}><Copy /> {theme.custom ? "Duplicate…" : "Customize…"}</DropdownMenu.Item>
			<DropdownMenu.Item onclick={onCopyJson}><Braces /> Copy as JSON</DropdownMenu.Item>
			{#if onDelete}
				<DropdownMenu.Separator />
				<DropdownMenu.Item variant="destructive" onclick={onDelete}><Trash /> Delete</DropdownMenu.Item>
			{/if}
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
