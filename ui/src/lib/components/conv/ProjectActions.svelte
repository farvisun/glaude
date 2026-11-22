<script lang="ts">
	// Project actions from `.glaude.json` ("Dev", "Test", ...), run in a terminal tab.
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import { call } from "$lib/ipc";
	import { ui } from "$lib/state/ui.svelte";
	import type { ProjectConfig } from "$lib/state/git";
	import Play from "@lucide/svelte/icons/play";

	let { cwd }: { cwd: string } = $props();
	let actions = $state<{ name: string; command: string }[]>([]);

	$effect(() => {
		const dir = cwd;
		call<ProjectConfig | null>("project_config", { cwd: dir })
			.then((c) => {
				if (dir === cwd) actions = (c?.actions ?? []).filter((a) => a?.name && a?.command);
			})
			.catch(() => (actions = []));
	});

	const run = (a: { name: string; command: string }) => ui.openTerminal({ cwd, command: a.command }, a.name);
</script>

{#if actions.length === 1}
	<button class="hover:text-foreground inline-flex shrink-0 items-center gap-1" title="Run: {actions[0].command}" onclick={() => run(actions[0])}
		><Play class="size-3" />{actions[0].name}</button
	>
{:else if actions.length > 1}
	<DropdownMenu.Root>
		<DropdownMenu.Trigger class="hover:text-foreground inline-flex shrink-0 items-center gap-1" title="Project actions"
			><Play class="size-3" />Run</DropdownMenu.Trigger
		>
		<DropdownMenu.Content align="end" class="w-56">
			<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">From .glaude.json</DropdownMenu.Label>
			{#each actions as a (a.name)}
				<DropdownMenu.Item onclick={() => run(a)}>
					<Play />
					<span class="flex min-w-0 flex-col">
						<span>{a.name}</span>
						<span class="text-muted-foreground truncate font-mono text-[11px]">{a.command}</span>
					</span>
				</DropdownMenu.Item>
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/if}
