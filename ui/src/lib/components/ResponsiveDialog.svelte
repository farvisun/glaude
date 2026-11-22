<script lang="ts">
	// A modal on wide screens, a bottom sheet on narrow ones.
	import type { Snippet } from "svelte";
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Sheet from "$lib/components/ui/sheet";
	import { narrow } from "$lib/state/media.svelte";

	let {
		open = $bindable(false),
		title,
		description,
		children,
		actions,
		class: cls = "sm:max-w-lg",
	}: { open: boolean; title: string; description?: string; children: Snippet; actions: Snippet; class?: string } = $props();
</script>

{#if narrow.current}
	<Sheet.Root bind:open>
		<Sheet.Content side="bottom" class="max-h-[90vh] gap-3 overflow-y-auto px-4 pb-4">
			<Sheet.Header class="px-0">
				<Sheet.Title>{title}</Sheet.Title>
				{#if description}<Sheet.Description>{description}</Sheet.Description>{/if}
			</Sheet.Header>
			{@render children()}
			<div class="flex justify-end gap-2">{@render actions()}</div>
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Dialog.Root bind:open>
		<Dialog.Content class={cls}>
			<Dialog.Header>
				<Dialog.Title>{title}</Dialog.Title>
				{#if description}<Dialog.Description>{description}</Dialog.Description>{/if}
			</Dialog.Header>
			<div class="flex flex-col gap-3">{@render children()}</div>
			<Dialog.Footer>{@render actions()}</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>
{/if}
