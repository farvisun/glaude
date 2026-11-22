<script lang="ts">
	// Settings → Network: saved proxies, and which one is used by default for
	// the app, each account and each project.
	import { Button } from "$lib/components/ui/button";
	import { Switch } from "$lib/components/ui/switch";
	import SettingRow from "./SettingRow.svelte";
	import Choice from "./Choice.svelte";
	import ProxyCheck from "../ProxyCheck.svelte";
	import { proxies, maskUrl, isSocks, type CheckResult, type ProxySettings } from "$lib/state/proxies.svelte";
	import { accounts, accountKey } from "$lib/state/accounts.svelte";
	import { history } from "$lib/state/history.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { basename } from "$lib/format";
	import { onMount } from "svelte";
	import { toast } from "svelte-sonner";
	import Plus from "@lucide/svelte/icons/plus";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash from "@lucide/svelte/icons/trash-2";
	import Activity from "@lucide/svelte/icons/activity";
	import X from "@lucide/svelte/icons/x";
	import Globe from "@lucide/svelte/icons/globe";

	onMount(() => {
		void proxies.ready();
		if (!accounts.list.length) void accounts.load();
	});

	let checks = $state<Record<string, CheckResult | "busy">>({});
	async function check(id: string, url: string) {
		checks[id] = "busy";
		try {
			checks[id] = await proxies.check(url);
		} catch (e) {
			delete checks[id];
			toast.error("Could not check the proxy", { description: e instanceof Error ? e.message : String(e) });
		}
	}

	const update = (patch: Partial<ProxySettings>) => proxies.save({ ...proxies.s, ...patch });

	function remove(id: string) {
		const s = proxies.s;
		const drop = (m: Record<string, string>) => Object.fromEntries(Object.entries(m).filter(([, v]) => v !== id));
		const name = proxies.byId(id)?.name;
		void update({
			proxies: s.proxies.filter((p) => p.id !== id),
			appDefault: s.appDefault === id ? null : s.appDefault,
			byAccount: drop(s.byAccount),
			byProject: drop(s.byProject),
		}).then((ok) => ok && toast(`Removed ${name}`, { description: "Defaults that used it fall back to the next one." }));
	}

	const appOptions = $derived([{ value: "", label: "No proxy" }, ...proxies.list.map((p) => ({ value: p.id, label: p.name }))]);
	const ruleOptions = $derived([
		{ value: "", label: "App default" },
		...proxies.list.map((p) => ({ value: p.id, label: p.name })),
		{ value: "none", label: "No proxy" },
	]);
	const shownAccounts = $derived(accounts.list.filter((a) => !a.archived));
	const projectRules = $derived(Object.entries(proxies.s.byProject).sort(([a], [b]) => basename(a).localeCompare(basename(b))));
	const freeProjects = $derived(history.groups.filter((g) => !(g.cwd in proxies.s.byProject)));

	function setAccount(key: string, v: string) {
		const byAccount = { ...proxies.s.byAccount };
		if (v) byAccount[key] = v;
		else delete byAccount[key];
		void update({ byAccount });
	}
	function setProject(cwd: string, v: string | null) {
		const byProject = { ...proxies.s.byProject };
		if (v === null) delete byProject[cwd];
		else byProject[cwd] = v;
		void update({ byProject });
	}
</script>

<section class="settings-section">
	<div class="flex items-center gap-2 pt-1 pb-1">
		<h2 class="text-[15px] font-semibold">Network</h2>
		<Button variant="outline" size="sm" class="ms-auto h-7" onclick={() => (ui.proxyEdit = "new")}><Plus /> Add proxy</Button>
	</div>
	<SettingRow label="Proxies" description="Claude, Codex and terminals go out through the one picked for them." keywords="proxy http socks vpn network" stacked>
		{#if !proxies.loaded}
			<div class="skeleton-shimmer h-14 rounded-md"></div>
		{:else if proxies.list.length}
			<ul class="border-border divide-border divide-y rounded-md border">
				{#each proxies.list as p (p.id)}
					{@const c = checks[p.id]}
					<li class="flex flex-col gap-2 px-3 py-2">
						<div class="flex items-center gap-3">
							<Globe class="text-muted-foreground size-4 shrink-0" />
							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-2">
									<span class="truncate text-[13px] font-medium" dir="auto">{p.name}</span>
									{#if proxies.s.appDefault === p.id}<span class="bg-primary/10 text-primary rounded px-1.5 text-[11px]">Default</span>{/if}
									{#if isSocks(p.url)}<span class="bg-muted text-muted-foreground rounded px-1.5 text-[11px]" title="Claude Code does not support SOCKS proxies">SOCKS</span>{/if}
								</div>
								<p class="text-muted-foreground truncate font-mono text-[12px]" dir="ltr">{maskUrl(p.url)}</p>
							</div>
							<Button variant="ghost" size="sm" class="h-7" disabled={c === "busy"} onclick={() => check(p.id, p.url)}><Activity /> Check</Button>
							<Button variant="ghost" size="icon-sm" aria-label="Edit {p.name}" onclick={() => (ui.proxyEdit = p)}><Pencil /></Button>
							<Button variant="ghost" size="icon-sm" aria-label="Delete {p.name}" onclick={() => remove(p.id)}><Trash /></Button>
						</div>
						{#if c}<div class="ps-7"><ProxyCheck result={c === "busy" ? null : c} busy={c === "busy"} /></div>{/if}
					</li>
				{/each}
			</ul>
		{:else}
			<div class="text-muted-foreground border-border flex flex-col items-center gap-2 rounded-md border border-dashed px-3 py-6 text-center text-[13px]">
				No proxies yet. Everything connects directly.
				<Button variant="outline" size="sm" class="h-7" onclick={() => (ui.proxyEdit = "new")}><Plus /> Add proxy</Button>
			</div>
		{/if}
	</SettingRow>
	{#if proxies.list.length}
		<SettingRow label="Default proxy" description="Used unless a project, an account or the conversation picks another." keywords="proxy app default network">
			<Choice label="Default proxy" value={proxies.s.appDefault ?? ""} options={appOptions} onChange={(v) => update({ appDefault: v || null })} />
		</SettingRow>
		<SettingRow label="Terminals use the proxy" description="Shells get HTTPS_PROXY, HTTP_PROXY, ALL_PROXY and NO_PROXY." keywords="proxy terminal shell env">
			<Switch checked={proxies.s.terminals} onCheckedChange={(v) => update({ terminals: v })} aria-label="Terminals use the proxy" />
		</SettingRow>
		<SettingRow label="Per account" description="An account can always go out the same way." keywords="proxy account default" stacked>
			{#if shownAccounts.length}
				<ul class="border-border divide-border divide-y rounded-md border">
					{#each shownAccounts as a (accountKey(a))}
						<li class="flex items-center gap-3 px-3 py-1.5">
							<span class="size-1.5 shrink-0 rounded-full {a.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
							<span class="min-w-0 flex-1 truncate text-[13px]">{a.provider === "claude" ? "Claude" : "Codex"} · {a.name}</span>
							<Choice
								label="Proxy for {a.name}"
								value={proxies.s.byAccount[accountKey(a)] ?? ""}
								options={ruleOptions}
								onChange={(v) => setAccount(accountKey(a), v)}
							/>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="text-muted-foreground text-[13px]">No accounts found.</p>
			{/if}
		</SettingRow>
		<SettingRow label="Per project" description="Wins over the account. Applies to the folder and the folders inside it." keywords="proxy project folder default" stacked>
			<div class="flex flex-col gap-2">
				{#if projectRules.length}
					<ul class="border-border divide-border divide-y rounded-md border">
						{#each projectRules as [cwd, v] (cwd)}
							<li class="flex items-center gap-3 px-3 py-1.5">
								<span class="min-w-0 flex-1 truncate text-[13px]" title={cwd}>{basename(cwd)}</span>
								<Choice label="Proxy for {basename(cwd)}" value={v} options={ruleOptions.slice(1)} onChange={(x) => setProject(cwd, x)} />
								<Button variant="ghost" size="icon-sm" aria-label="Remove the rule for {basename(cwd)}" onclick={() => setProject(cwd, null)}><X /></Button>
							</li>
						{/each}
					</ul>
				{/if}
				{#if freeProjects.length}
					<Choice
						label="Add a project"
						class="w-60"
						value={""}
						options={[{ value: "", label: "Add a project…" }, ...freeProjects.map((g) => ({ value: g.cwd, label: g.name }))]}
						onChange={(cwd) => cwd && setProject(cwd, proxies.list[0]?.id ?? "none")}
					/>
				{/if}
			</div>
		</SettingRow>
	{/if}
</section>
