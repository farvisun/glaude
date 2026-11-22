<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { accounts } from "$lib/state/accounts.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import type { Provider } from "$lib/types";

	let { open = $bindable(false), defaultProvider = "claude" }: { open?: boolean; defaultProvider?: Provider } = $props();

	let provider = $state<Provider>("claude");
	let name = $state("");
	let busy = $state(false);

	$effect(() => {
		if (open) {
			provider = defaultProvider;
			name = "";
		}
	});

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!name.trim()) return;
		busy = true;
		const created = await accounts.create(provider, name);
		busy = false;
		if (!created) return;
		open = false;
		// Log in with the tool's own flow, in a terminal with the new home.
		const title = `${provider === "claude" ? "Claude" : "Codex"} login · ${created}`;
		ui.openTerminal({ run: { provider, account: created, login: true } }, title);
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<form onsubmit={submit} class="space-y-4">
			<Dialog.Header>
				<Dialog.Title>Add account</Dialog.Title>
				<Dialog.Description>
					Creates a separate home for the tool, then opens a terminal to log in. Sessions and settings are shared with the default home on
					first use.
				</Dialog.Description>
			</Dialog.Header>
			<div class="bg-muted grid grid-cols-2 gap-1 rounded-lg p-1">
				{#each [{ id: "claude", label: "Claude Code" }, { id: "codex", label: "Codex" }] as p (p.id)}
					<button
						type="button"
						class="flex items-center justify-center gap-2 rounded-md py-1.5 text-[13px] {provider === p.id
							? 'bg-background shadow-sm'
							: 'text-muted-foreground hover:text-foreground'}"
						onclick={() => (provider = p.id as Provider)}
					>
						<span class="size-2 rounded-full {p.id === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>{p.label}
					</button>
				{/each}
			</div>
			<div class="space-y-1.5">
				<Label for="acct-name">Name</Label>
				<Input id="acct-name" bind:value={name} placeholder="work" autocomplete="off" />
				<p class="text-muted-foreground text-[12px]">
					Home: {provider === "claude" ? `~/.claude_multi/${name.trim() || "<name>"}` : `~/.codex_${name.trim() || "<name>"}`}
				</p>
			</div>
			<Dialog.Footer>
				<Button type="button" variant="ghost" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={!name.trim() || busy}>{busy ? "Adding…" : "Add and log in"}</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
