<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Select from "$lib/components/ui/select";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import FolderOpen from "@lucide/svelte/icons/folder-open";
	import { ui } from "$lib/state/ui.svelte";
	import { permissionModes, startConversation, showError } from "$lib/state/sessions";
	import { call } from "$lib/ipc";
	import type { Provider } from "$lib/types";

	interface AccountOption {
		provider: Provider;
		name: string;
		email?: string;
		active: boolean;
		archived: boolean;
	}

	const LAST_KEY = "glaude.newconv.v1";
	interface Last {
		provider: Provider;
		cwd: string;
		account: Record<Provider, string>;
		mode: Record<Provider, string>;
		model: Record<Provider, string>;
		recent: string[];
	}

	function loadLast(): Last {
		const fallback: Last = {
			provider: "claude",
			cwd: "",
			account: { claude: "default", codex: "default" },
			mode: { claude: "default", codex: "full" },
			model: { claude: "", codex: "" },
			recent: [],
		};
		try {
			return { ...fallback, ...(JSON.parse(localStorage.getItem(LAST_KEY) ?? "null") ?? {}) };
		} catch {
			return fallback;
		}
	}

	const last = loadLast();
	let provider = $state<Provider>(last.provider);
	let cwd = $state(last.cwd);
	let account = $state(last.account[last.provider]);
	let mode = $state(last.mode[last.provider]);
	let model = $state(last.model[last.provider]);
	let accounts = $state<AccountOption[]>([]);
	let starting = $state(false);

	const claudeModels = [
		{ value: "", label: "Default" },
		{ value: "opus", label: "Opus" },
		{ value: "sonnet", label: "Sonnet" },
		{ value: "haiku", label: "Haiku" },
		{ value: "fable", label: "Fable" },
	];
	let codexModels = $state<{ value: string; label: string }[]>([]);
	const models = $derived(provider === "claude" ? claudeModels : [{ value: "", label: "Default" }, ...codexModels]);

	// Codex lists its models through the app-server of the chosen account.
	$effect(() => {
		if (!ui.newConversationOpen || provider !== "codex") return;
		const acct = account;
		call<{ value: string; label: string }[]>("codex_models", { account: acct })
			.then((list) => (codexModels = list.map((m) => ({ value: m.value, label: m.label || m.value }))))
			.catch(() => (codexModels = []));
	});
	const modes = $derived(permissionModes(provider));
	const providerAccounts = $derived(accounts.filter((a) => a.provider === provider && !a.archived));

	$effect(() => {
		if (!ui.newConversationOpen) return;
		if (ui.newConversationCwd) cwd = ui.newConversationCwd;
		call<AccountOption[]>("accounts_brief")
			.then((list) => (accounts = list))
			.catch(() => (accounts = []));
		if (!cwd) call<string>("default_cwd").then((d) => (cwd ||= d)).catch(() => {});
	});

	function switchProvider(p: Provider) {
		provider = p;
		account = last.account[p] ?? "default";
		mode = last.mode[p] ?? modes[0].value;
		model = last.model[p] ?? "";
	}

	async function pickFolder() {
		try {
			const picked = await call<string | null>("pick_folder", { start: cwd || null });
			if (picked) cwd = picked;
		} catch (e) {
			showError("Could not open the folder picker", e);
		}
	}

	async function start() {
		if (!cwd.trim()) return;
		starting = true;
		const next: Last = {
			...last,
			provider,
			cwd,
			account: { ...last.account, [provider]: account },
			mode: { ...last.mode, [provider]: mode },
			model: { ...last.model, [provider]: model },
			recent: [cwd, ...last.recent.filter((r) => r !== cwd)].slice(0, 8),
		};
		localStorage.setItem(LAST_KEY, JSON.stringify(next));
		ui.newConversationOpen = false;
		await startConversation({ provider, account, cwd: cwd.trim(), model: model || undefined, permissionMode: mode });
		starting = false;
	}
</script>

<Dialog.Root bind:open={ui.newConversationOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>New conversation</Dialog.Title>
			<Dialog.Description>Runs the CLI installed on this computer with the account you pick.</Dialog.Description>
		</Dialog.Header>

		<form
			class="flex flex-col gap-4"
			onsubmit={(e) => {
				e.preventDefault();
				start();
			}}
		>
			<div class="bg-muted grid grid-cols-2 gap-1 rounded-lg p-1">
				{#each [{ id: "claude", label: "Claude Code" }, { id: "codex", label: "Codex" }] as p (p.id)}
					<button
						type="button"
						class="flex items-center justify-center gap-2 rounded-md py-1.5 text-[13px] font-medium {provider === p.id
							? 'bg-background shadow-xs'
							: 'text-muted-foreground'}"
						onclick={() => switchProvider(p.id as Provider)}
					>
						<span class="size-2 rounded-full {p.id === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>{p.label}
					</button>
				{/each}
			</div>

			<div class="flex flex-col gap-1.5">
				<Label for="nc-cwd">Folder</Label>
				<div class="flex gap-1.5">
					<Input id="nc-cwd" bind:value={cwd} dir="ltr" class="font-mono text-[13px]" placeholder="/path/to/project" />
					<Button type="button" variant="secondary" size="icon" aria-label="Choose folder" onclick={pickFolder}><FolderOpen /></Button>
				</div>
				{#if last.recent.length}
					<div class="flex flex-wrap gap-1">
						{#each last.recent.slice(0, 5) as r (r)}
							<button type="button" class="bg-muted hover:bg-accent truncate rounded px-1.5 py-0.5 font-mono text-[11px]" dir="ltr" onclick={() => (cwd = r)}
								>{r.split("/").filter(Boolean).pop()}</button
							>
						{/each}
					</div>
				{/if}
			</div>

			<div class="grid grid-cols-2 gap-3">
				<div class="flex flex-col gap-1.5">
					<Label>Account</Label>
					<Select.Root type="single" bind:value={account}>
						<Select.Trigger class="w-full">{account}</Select.Trigger>
						<Select.Content>
							{#each providerAccounts.length ? providerAccounts : [{ name: "default", email: undefined }] as a (a.name)}
								<Select.Item value={a.name} label={a.name}>
									<span class="truncate">{a.name}</span>
									{#if a.email && a.email !== a.name}<span class="text-muted-foreground truncate text-xs">{a.email}</span>{/if}
								</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>
				<div class="flex flex-col gap-1.5">
					<Label>Model</Label>
					<Select.Root type="single" bind:value={model}>
						<Select.Trigger class="w-full">{models.find((m) => m.value === model)?.label ?? (model || "Default")}</Select.Trigger>
						<Select.Content>
							{#each models as m (m.value)}
								<Select.Item value={m.value} label={m.label}>{m.label}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>
			</div>

			<div class="flex flex-col gap-1.5">
				<Label>Permissions</Label>
				<Select.Root type="single" bind:value={mode}>
					<Select.Trigger class="w-full">{modes.find((m) => m.value === mode)?.label ?? mode}</Select.Trigger>
					<Select.Content>
						{#each modes as m (m.value)}
							<Select.Item value={m.value} label={m.label}>{m.label}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<Dialog.Footer>
				<Button type="button" variant="ghost" onclick={() => (ui.newConversationOpen = false)}>Cancel</Button>
				<Button type="submit" disabled={!cwd.trim() || starting}>Start</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
