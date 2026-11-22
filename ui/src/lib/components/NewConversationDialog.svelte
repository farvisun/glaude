<script lang="ts">
	import { untrack } from "svelte";
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Select from "$lib/components/ui/select";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { Switch } from "$lib/components/ui/switch";
	import FolderOpen from "@lucide/svelte/icons/folder-open";
	import { ui } from "$lib/state/ui.svelte";
	import { permissionModes, startConversation, showError } from "$lib/state/sessions";
	import { proxies, isSocks } from "$lib/state/proxies.svelte";
	import { projects, PROJECT_COLORS } from "$lib/state/projects.svelte";
	import ProjectIcon from "./ProjectIcon.svelte";
	import { basename } from "$lib/format";
	import ProxyPicker from "./ProxyPicker.svelte";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import { call } from "$lib/ipc";
	import type { AgentInfo, Provider } from "$lib/types";
	import { claudeModels, efforts, loadNewConv, saveNewConv, type NewConvDefaults } from "$lib/state/newconv";

	interface AccountOption {
		provider: Provider;
		name: string;
		email?: string;
		active: boolean;
		archived: boolean;
	}

	type Last = NewConvDefaults;
	const initial = loadNewConv();
	let last = $state(initial);
	let provider = $state<Provider>(initial.provider);
	let cwd = $state(initial.cwd);
	let account = $state(initial.account[initial.provider]);
	let mode = $state(initial.mode[initial.provider]);
	let model = $state(initial.model[initial.provider]);
	let effort = $state(initial.effort[initial.provider]);
	let accounts = $state<AccountOption[]>([]);
	let starting = $state(false);
	/** Proxy for this conversation: an id, "none", or undefined for the defaults. */
	let proxy = $state<string | undefined>();
	$effect(() => {
		if (ui.newConversationOpen) void proxies.ready();
	});
	const net = $derived(proxies.resolve({ provider, account, cwd }, proxy));

	// Inside a project: its defaults fill the form (once per project picked).
	const project = $derived(projects.of(cwd.trim()));
	let appliedFor: string | null = null;
	$effect(() => {
		if (!ui.newConversationOpen) {
			appliedFor = null;
			return;
		}
		const p = project;
		const key = p?.id ?? "";
		if (key === appliedFor) return;
		appliedFor = key;
		if (!p || ui.newConversationPreset) return;
		untrack(() => {
			const d = p.defaults;
			if (d.provider) switchProvider(d.provider);
			if (d.account) account = d.account;
			if (d.model) model = d.model;
			if (d.effort) effort = d.effort;
			if (d.mode) mode = d.mode;
		});
	});
	/** Outside every project: make the folder one. */
	let addProject = $state(false);
	/** Claude: run the session as this agent ("" = the default). */
	let agent = $state("");
	// Work in a new git worktree (own branch and folder).
	let isRepo = $state(false);
	let inWorktree = $state(false);
	let branchName = $state("");
	$effect(() => {
		const dir = cwd.trim();
		isRepo = false;
		if (!dir) return;
		const t = setTimeout(() => {
			call("git_info", { cwd: dir })
				.then(() => {
					if (cwd.trim() === dir) isRepo = true;
				})
				.catch(() => {});
		}, 300);
		return () => clearTimeout(t);
	});
	let agentList = $state<AgentInfo[]>([]);
	$effect(() => {
		if (!ui.newConversationOpen || provider !== "claude") return;
		const args = { account, cwd: cwd.trim() || null };
		const t = setTimeout(() => {
			call<AgentInfo[]>("claude_agents", args)
				.then((list) => {
					agentList = list;
					if (agent && !list.some((a) => a.name === agent)) agent = "";
				})
				.catch(() => (agentList = []));
		}, 200);
		return () => clearTimeout(t);
	});

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
		untrack(() => {
			last = loadNewConv();
			switchProvider(last.provider);
		});
		if (ui.newConversationCwd) cwd = ui.newConversationCwd;
		const preset = ui.newConversationPreset;
		if (preset)
			untrack(() => {
				switchProvider(preset.provider);
				account = preset.account;
			});
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
		effort = last.effort[p] ?? "";
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
		const recent = [cwd, ...last.recent.filter((r) => r !== cwd)].slice(0, 8);
		// Settings → New conversations decides whether choices become the defaults.
		const next: Last = last.remember
			? {
					...last,
					provider,
					cwd,
					account: { ...last.account, [provider]: account },
					mode: { ...last.mode, [provider]: mode },
					model: { ...last.model, [provider]: model },
					effort: { ...last.effort, [provider]: effort },
					recent,
				}
			: { ...last, cwd, recent };
		saveNewConv(next);
		last = next;
		let dir = cwd.trim();
		let title: string | undefined;
		if (inWorktree && isRepo) {
			try {
				const name = branchName.trim() || `chat-${new Date().toISOString().slice(5, 16).replace(/[-T:]/g, "")}`;
				const wt = await call<{ path: string; branch: string; setup?: string | null }>("worktree_add", { cwd: dir, name });
				dir = wt.path;
				title = wt.branch.replace(/^glaude\//, "");
				if (wt.setup) ui.openTerminal({ cwd: wt.path, command: wt.setup }, "Worktree setup");
			} catch (e) {
				starting = false;
				showError("Could not create the worktree", e);
				return;
			}
		}
		if (addProject && !project) {
			const used = new Set(projects.list.map((p) => p.color));
			const color = (PROJECT_COLORS.find((c) => c.id !== "gray" && !used.has(c.id)) ?? PROJECT_COLORS[6]).id;
			projects.add({ name: basename(cwd.trim()) || "Project", folders: [cwd.trim()], color, defaults: {} });
		}
		addProject = false;
		ui.newConversationOpen = false;
		inWorktree = false;
		branchName = "";
		await startConversation({
			provider,
			account,
			title,
			cwd: dir,
			model: model || undefined,
			effort: effort || undefined,
			permissionMode: mode,
			agent: provider === "claude" && agent ? agent : undefined,
			proxy,
		});
		proxy = undefined;
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
				{#if projects.list.length || last.recent.length}
					<!-- Projects first, then recent folders outside them. -->
					<div class="flex flex-wrap gap-1">
						{#each projects.list as p (p.id)}
							<button
								type="button"
								class="hover:bg-accent inline-flex h-6 max-w-40 items-center gap-1 rounded-md border px-1.5 text-[12px] {project?.id === p.id
									? 'border-foreground/30 bg-accent'
									: 'border-border'}"
								onclick={() => (cwd = p.folders[0])}
								title={p.folders[0]}><ProjectIcon project={p} class="size-3" /><span class="truncate">{p.name}</span></button
							>
						{/each}
						{#each last.recent.filter((r) => !projects.of(r)).slice(0, 4) as r (r)}
							<button type="button" class="bg-muted hover:bg-accent h-6 truncate rounded-md px-1.5 font-mono text-[11px]" dir="ltr" onclick={() => (cwd = r)}
								>{r.split("/").filter(Boolean).pop()}</button
							>
						{/each}
					</div>
				{/if}
				{#if cwd.trim() && !project}
					<label class="text-muted-foreground flex items-center gap-2 text-[12px]">
						<Switch bind:checked={addProject} aria-label="Add as a project" class="scale-90" />
						Add {basename(cwd.trim()) || "this folder"} as a project
					</label>
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

			{#if provider === "claude" && agentList.length}
				<div class="flex flex-col gap-1.5">
					<Label>Agent</Label>
					<Select.Root type="single" bind:value={agent}>
						<Select.Trigger class="w-full">{agent || "Default (Claude)"}</Select.Trigger>
						<Select.Content class="max-h-72">
							<Select.Item value="" label="Default (Claude)">Default (Claude)</Select.Item>
							{#each agentList as a (a.name)}
								<Select.Item value={a.name} label={a.name} title={a.description || undefined}>
									<span class="shrink-0">{a.name}</span>
									{#if a.description}<span class="text-muted-foreground min-w-0 flex-1 truncate text-xs">{a.description}</span>{/if}
								</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>
			{/if}

			<div class="grid grid-cols-2 gap-3">
				<div class="flex flex-col gap-1.5">
					<Label>Effort</Label>
					<Select.Root type="single" bind:value={effort}>
						<Select.Trigger class="w-full">{efforts[provider].find((x) => x.value === effort)?.label ?? effort}</Select.Trigger>
						<Select.Content>
							{#each efforts[provider] as x (x.value)}
								<Select.Item value={x.value} label={x.label}>{x.label}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
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
			</div>

			{#if proxies.list.length}
				<div class="flex flex-col gap-1.5">
					<Label>Proxy</Label>
					<ProxyPicker value={proxy} onChange={(v) => (proxy = v)} target={{ provider, account, cwd }} />
					{#if provider === "claude" && net.proxy && isSocks(net.proxy.url)}
						<p class="text-attention flex items-start gap-1.5 text-[12px]">
							<TriangleAlert class="mt-0.5 size-3.5 shrink-0" />{net.proxy.name} is a SOCKS proxy. Claude Code only works with HTTP proxies.
						</p>
					{/if}
				</div>
			{/if}

			{#if isRepo}
				<div class="flex flex-col gap-2">
					<label class="flex items-start justify-between gap-3 text-[13px]">
						<span>
							<span class="font-medium">In a new worktree</span>
							<span class="text-muted-foreground block text-xs">Its own branch and folder, so your checkout stays untouched.</span>
						</span>
						<Switch bind:checked={inWorktree} aria-label="In a new worktree" />
					</label>
					{#if inWorktree}
						<Input bind:value={branchName} placeholder="Branch name (glaude/…)" aria-label="Branch name" class="font-mono text-[13px]" dir="ltr" />
					{/if}
				</div>
			{/if}

			<Dialog.Footer>
				<Button type="button" variant="ghost" onclick={() => (ui.newConversationOpen = false)}>Cancel</Button>
				<Button type="submit" disabled={!cwd.trim() || starting}>Start</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
