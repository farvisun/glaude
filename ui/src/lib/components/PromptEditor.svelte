<script lang="ts">
	// Create or edit a saved prompt (modal; a bottom sheet on narrow windows).
	import ResponsiveDialog from "./ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Textarea } from "$lib/components/ui/textarea";
	import { Label } from "$lib/components/ui/label";
	import Choice from "./settings/Choice.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { prompts, slugOf, variablesOf } from "$lib/state/prompts.svelte";
	import { history } from "$lib/state/history.svelte";
	import { currentConv } from "$lib/state/commands.svelte";
	import { basename } from "$lib/format";
	import { toast } from "svelte-sonner";
	import { untrack } from "svelte";

	let open = $state(false);
	let id = $state<string | undefined>();
	let name = $state("");
	let body = $state("");
	let scope = $state("all");

	$effect(() => {
		const e = ui.promptEdit;
		if (!e) return;
		untrack(() => {
			if (e === "new") {
				id = undefined;
				name = "";
				body = "";
				scope = "all";
			} else {
				id = e.id;
				name = e.name;
				body = e.body;
				scope = e.cwd ?? "all";
			}
			open = true;
		});
	});
	$effect(() => {
		if (!open) ui.promptEdit = null;
	});

	const folders = $derived.by(() => {
		const set = new Set(history.groups.map((g) => g.cwd));
		const cur = currentConv()?.cwd;
		if (cur) set.add(cur);
		if (scope !== "all") set.add(scope);
		return [...set];
	});
	const parts = $derived(variablesOf(body));
	const taken = $derived(prompts.list.some((p) => p.id !== id && slugOf(p.name) === slugOf(name) && (p.cwd ?? "all") === scope));

	function save() {
		if (!name.trim() || !body.trim() || taken) return;
		prompts.upsert({ id, name: name.trim(), body, cwd: scope === "all" ? null : scope });
		toast(id ? "Prompt saved" : "Prompt added", { description: `Type /${slugOf(name)} in a message to use it.` });
		open = false;
	}
</script>

<ResponsiveDialog bind:open title={id ? "Edit saved prompt" : "New saved prompt"} description={"Reuse a message. Write {{name}} for a part you fill in each time."}>
	<div class="flex flex-col gap-1.5">
		<Label for="prompt-name">Name</Label>
		<Input id="prompt-name" bind:value={name} placeholder="Review changes" dir="auto" />
		{#if name.trim()}
			<p class="text-[12px] {taken ? 'text-destructive' : 'text-muted-foreground'}">
				{taken ? "Another prompt here has this name." : `Use it by typing /${slugOf(name)}`}
			</p>
		{/if}
	</div>
	<div class="flex flex-col gap-1.5">
		<Label for="prompt-body">Message</Label>
		<Textarea id="prompt-body" bind:value={body} rows={7} placeholder={"Explain how {{topic}} works in {{project}}."} dir="auto" class="font-mono text-[13px]" />
		<p class="text-muted-foreground text-[12px]">
			{#if parts.length}You fill in: {parts.join(", ")}.{/if}
			Filled for you: {"{{cwd}}"}, {"{{project}}"}, {"{{branch}}"}, {"{{date}}"}.
		</p>
	</div>
	<div class="flex flex-col gap-1.5">
		<Label>Available in</Label>
		<Choice
			label="Available in"
			value={scope}
			options={[{ value: "all", label: "Every project" }, ...folders.map((f) => ({ value: f, label: basename(f) }))]}
			onChange={(v) => (scope = v)}
		/>
	</div>
	{#snippet actions()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button disabled={!name.trim() || !body.trim() || taken} onclick={save}>{id ? "Save" : "Add prompt"}</Button>
	{/snippet}
</ResponsiveDialog>
