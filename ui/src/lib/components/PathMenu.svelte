<script lang="ts">
	// Actions for a file or folder path clicked in a reply.
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import { ui } from "$lib/state/ui.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import { pathActions, fileManager } from "$lib/state/open";
	import { basename } from "$lib/format";
	import FileCode from "@lucide/svelte/icons/file-code";
	import Eye from "@lucide/svelte/icons/eye";
	import AppWindow from "@lucide/svelte/icons/app-window";
	import FolderSearch from "@lucide/svelte/icons/folder-search";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import Copy from "@lucide/svelte/icons/copy";
	import FolderOpen from "@lucide/svelte/icons/folder-open";

	const m = $derived(ui.pathMenu);
	let open = $state(false);
	$effect(() => {
		open = !!m;
	});
	function run(f: (h: NonNullable<typeof m>["hit"]) => unknown) {
		if (m) void f(m.hit);
		ui.pathMenu = null;
	}
</script>

{#if m}
	<DropdownMenu.Root
		bind:open
		onOpenChange={(o) => {
			if (!o) ui.pathMenu = null;
		}}
	>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<span {...props} class="pointer-events-none fixed size-0" style="left: {m.x}px; top: {m.y}px" aria-hidden="true"></span>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content align="start" class="w-64">
			<DropdownMenu.Label class="text-muted-foreground truncate text-xs font-normal" title={m.hit.abs}>
				{basename(m.hit.abs)}{m.hit.line ? `:${m.hit.line}` : ""}
			</DropdownMenu.Label>
			{#if m.hit.dir}
				<DropdownMenu.Item onclick={() => run(pathActions.system)}><FolderOpen /> Open in {fileManager}</DropdownMenu.Item>
				{#if prefs.editor !== "default"}
					<DropdownMenu.Item onclick={() => run(pathActions.editor)}><FileCode /> Open in {pathActions.editorLabel()}</DropdownMenu.Item>
				{/if}
			{:else}
				{#if prefs.editor !== "default"}
					<DropdownMenu.Item onclick={() => run(pathActions.editor)}><FileCode /> Open in {pathActions.editorLabel()}</DropdownMenu.Item>
				{/if}
				<DropdownMenu.Item onclick={() => run(pathActions.view)}><Eye /> View in Glaude</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => run(pathActions.system)}><AppWindow /> Open with default app</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => run(pathActions.reveal)}><FolderSearch /> Show in {fileManager}</DropdownMenu.Item>
			{/if}
			<DropdownMenu.Item onclick={() => run(pathActions.terminal)}><SquareTerminal /> Open terminal here</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item onclick={() => run(pathActions.copy)}><Copy /> Copy path</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/if}
