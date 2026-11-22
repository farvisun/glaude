<script lang="ts">
	// Fill in a saved prompt's parts, then put it in the composer.
	import ResponsiveDialog from "./ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { ui } from "$lib/state/ui.svelte";
	import { fillPrompt, variablesOf } from "$lib/state/prompts.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { showConversation } from "$lib/state/commands.svelte";
	import { untrack } from "svelte";

	let open = $state(false);
	let values = $state<Record<string, string>>({});
	const req = $derived(ui.promptFill);
	const parts = $derived(req ? variablesOf(req.prompt.body) : []);
	const conv = $derived(req ? conversations.get(req.convId) : undefined);
	const preview = $derived(req && conv ? fillPrompt(req.prompt.body, values, { cwd: conv.cwd, branch: conv.session.gitBranch }) : "");

	$effect(() => {
		if (!req) return;
		untrack(() => {
			values = Object.fromEntries(parts.map((p) => [p, ""]));
			open = true;
		});
	});
	$effect(() => {
		if (!open) ui.promptFill = null;
	});

	function insert() {
		if (!conv) return;
		conv.draft = preview;
		open = false;
		showConversation(conv.id);
	}
</script>

<ResponsiveDialog bind:open title={req?.prompt.name ?? "Saved prompt"} description="Fill in the parts; you can still edit the message before sending.">
	<form
		class="flex flex-col gap-3"
		onsubmit={(e) => {
			e.preventDefault();
			insert();
		}}
	>
		{#each parts as p, i (p)}
			<div class="flex flex-col gap-1.5">
				<Label for="fill-{i}">{p}</Label>
				<!-- svelte-ignore a11y_autofocus -->
				<Input id="fill-{i}" bind:value={values[p]} dir="auto" autofocus={i === 0} />
			</div>
		{/each}
		<div class="bg-muted text-muted-foreground max-h-40 overflow-y-auto rounded-md px-3 py-2 text-[13px] whitespace-pre-wrap" dir="auto">{preview}</div>
		<button type="submit" class="hidden" aria-hidden="true" tabindex="-1"></button>
	</form>
	{#snippet actions()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button onclick={insert}>Use prompt</Button>
	{/snippet}
</ResponsiveDialog>
