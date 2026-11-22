<script lang="ts">
	// ⇧⌘P: every action, the open agents and saved prompts in one list.
	import * as Command from "$lib/components/ui/command";
	import { ui } from "$lib/state/ui.svelte";
	import { commandList, run, currentConv, showConversation, usePrompt } from "$lib/state/commands.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { stepOf, byUrgency } from "$lib/state/activity";
	import { prompts, slugOf } from "$lib/state/prompts.svelte";
	import { basename } from "$lib/format";
	import AgentGlyph from "./AgentGlyph.svelte";
	import BookmarkIcon from "@lucide/svelte/icons/bookmark";

	let query = $state("");
	$effect(() => {
		if (!ui.paletteOpen) query = "";
	});
	const conv = $derived(ui.paletteOpen ? currentConv() : undefined);
	const agents = $derived(
		ui.paletteOpen
			? Object.values(conversations.byId)
					.map((c) => ({ conv: c, step: stepOf(c) }))
					.sort(byUrgency)
			: [],
	);
	const saved = $derived(ui.paletteOpen ? prompts.forCwd(conv?.cwd) : []);
	const groups = $derived([...new Set(commandList.map((c) => c.group))]);

	function pick(fn: () => void) {
		ui.paletteOpen = false;
		// After the dialog closes, so focus moves where the action wants it.
		setTimeout(fn, 0);
	}
</script>

<Command.Dialog bind:open={ui.paletteOpen} title="Command palette" description="Run an action, open an agent or use a saved prompt" class="sm:max-w-xl">
	<Command.Input bind:value={query} placeholder="Type a command, an agent or a prompt…" dir="auto" />
	<Command.List class="max-h-[60vh]">
		<Command.Empty>Nothing matches.</Command.Empty>
		{#if agents.length}
			<Command.Group heading="Agents">
				{#each agents as a (a.conv.id)}
					<Command.Item value="agent {a.conv.title} {a.step.text} {basename(a.conv.cwd)}" onSelect={() => pick(() => showConversation(a.conv.id))}>
						<AgentGlyph tone={a.step.tone} provider={a.conv.provider} />
						<span class="min-w-0 flex-1 truncate" dir="auto">{a.conv.title}</span>
						<span class="text-muted-foreground max-w-48 truncate text-xs" dir="auto">{a.step.text}</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		{#if conv && saved.length}
			<Command.Group heading="Saved prompts">
				{#each saved as p (p.id)}
					<Command.Item value="prompt {p.name} {slugOf(p.name)}" onSelect={() => pick(() => usePrompt(p, conv))}>
						<BookmarkIcon />
						<span class="min-w-0 flex-1 truncate" dir="auto">{p.name}</span>
						<span class="text-muted-foreground font-mono text-xs">/{slugOf(p.name)}</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		{#each groups as g (g)}
			<Command.Group heading={g}>
				{#each commandList.filter((c) => c.group === g) as c (c.id)}
					<Command.Item value="{c.label} {c.words ?? ''}" onSelect={() => pick(() => run(c.id))}>
						<span class="min-w-0 flex-1 truncate">{c.label}</span>
						{#if c.keys}<Command.Shortcut>{c.keys}</Command.Shortcut>{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/each}
	</Command.List>
</Command.Dialog>
