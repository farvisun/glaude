<script lang="ts">
	// Rename a past session in the sidebar. The name is Glaude's own; the
	// CLI's transcript keeps its title.
	import ResponsiveDialog from "./ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { ui } from "$lib/state/ui.svelte";
	import { history } from "$lib/state/history.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { layout } from "$lib/state/layout.svelte";
	import { untrack } from "svelte";

	let open = $state(false);
	let name = $state("");
	const s = $derived(ui.renameSession);
	const original = $derived(s ? history.originalTitle(s) : undefined);
	const renamed = $derived(!!s && !!history.names[`${s.provider}:${s.id}`]);

	$effect(() => {
		const target = ui.renameSession;
		if (!target) return;
		untrack(() => {
			name = target.title;
			open = true;
		});
	});
	$effect(() => {
		if (!open) ui.renameSession = null;
	});

	function apply(title: string) {
		if (!s) return;
		const shown = history.rename(s, title);
		// An open tab of this session takes the name too.
		for (const c of Object.values(conversations.byId)) {
			if (c.session.sessionId !== s.id) continue;
			c.title = shown;
			const tab = Object.values(layout.tabs).find((t) => t.ref === c.id);
			if (tab) layout.rename(tab.id, shown);
		}
		open = false;
	}
</script>

<ResponsiveDialog bind:open title="Rename conversation" description="The new name shows in Glaude only. The CLI keeps its own title.">
	<form
		id="rename-session"
		class="flex flex-col gap-1.5"
		onsubmit={(e) => {
			e.preventDefault();
			if (name.trim()) apply(name);
		}}
	>
		<Label for="session-name">Name</Label>
		<!-- svelte-ignore a11y_autofocus -->
		<Input id="session-name" bind:value={name} dir="auto" autofocus onfocus={(e) => e.currentTarget.select()} />
		{#if renamed && original}
			<p class="text-muted-foreground truncate text-[12px]" dir="auto">Original: {original}</p>
		{/if}
	</form>
	{#snippet actions()}
		{#if renamed}<Button variant="outline" class="sm:me-auto" onclick={() => apply("")}>Use the original</Button>{/if}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button type="submit" form="rename-session" disabled={!name.trim()}>Rename</Button>
	{/snippet}
</ResponsiveDialog>
