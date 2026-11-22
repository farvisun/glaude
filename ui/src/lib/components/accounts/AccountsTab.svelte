<script lang="ts">
	import * as Table from "$lib/components/ui/table";
	import * as Select from "$lib/components/ui/select";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Switch } from "$lib/components/ui/switch";
	import { Skeleton } from "$lib/components/ui/skeleton";
	import * as Sheet from "$lib/components/ui/sheet";
	import Plus from "@lucide/svelte/icons/plus";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import Search from "@lucide/svelte/icons/search";
	import Users from "@lucide/svelte/icons/users";
	import LimitBar from "$lib/components/conv/LimitBar.svelte";
	import AccountDetails from "./AccountDetails.svelte";
	import AddAccountDialog from "./AddAccountDialog.svelte";
	import { accounts, accountKey, type AccountView, type SortKey } from "$lib/state/accounts.svelte";
	import { fmtAgo, fmtTokens } from "$lib/format";

	let { active = true }: { active?: boolean } = $props();

	type ToolFilter = "all" | "claude" | "codex";
	const FILTER_KEY = "glaude.accounts.v1";
	const saved = (() => {
		try {
			return JSON.parse(localStorage.getItem(FILTER_KEY) ?? "{}") as { tool?: ToolFilter; archived?: boolean; selected?: string };
		} catch {
			return {};
		}
	})();

	let tool = $state<ToolFilter>(saved.tool ?? "all");
	let showArchived = $state(saved.archived ?? false);
	let query = $state("");
	let sort = $state<SortKey>("name");
	let selected = $state<string | null>(saved.selected ?? null);
	let addOpen = $state(false);
	let width = $state(1200);
	const narrow = $derived(width < 860);

	$effect(() => {
		localStorage.setItem(FILTER_KEY, JSON.stringify({ tool, archived: showArchived, selected }));
	});

	// Sort is stored in mamadoo's own config (ui.sort), per tool.
	$effect(() => {
		const p = tool === "codex" ? "codex" : "claude";
		accounts.sortPref(p).then((s) => (sort = s));
	});

	function setSort(s: SortKey) {
		sort = s;
		if (tool === "all") {
			accounts.saveSort("claude", s);
			accounts.saveSort("codex", s);
		} else accounts.saveSort(tool, s);
	}

	// Reload when the tab is shown; scan in the background.
	$effect(() => {
		if (!active) return;
		accounts.load().then(() => accounts.refreshAll());
	});

	const lastUsedMs = (a: AccountView) => (a.lastUsed ? new Date(a.lastUsed).getTime() : 0);

	const rows = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = accounts.list
			.filter((a) => tool === "all" || a.provider === tool)
			.filter((a) => showArchived || !a.archived)
			.filter((a) => !q || a.name.toLowerCase().includes(q) || a.email.toLowerCase().includes(q) || a.plan.toLowerCase().includes(q))
			.map((a) => accounts.withLive(a));
		const by: Record<SortKey, (a: AccountView, b: AccountView) => number> = {
			name: (a, b) => Number(b.protected) - Number(a.protected) || a.name.localeCompare(b.name),
			recent: (a, b) => lastUsedMs(b) - lastUsedMs(a),
			tokens: (a, b) => b.tokens - a.tokens,
		};
		return list.sort((a, b) => (tool === "all" ? a.provider.localeCompare(b.provider) : 0) || by[sort](a, b));
	});

	const current = $derived(rows.find((a) => accountKey(a) === selected) ?? null);
	const hiddenArchived = $derived(accounts.list.filter((a) => a.archived && (tool === "all" || a.provider === tool)).length);
	const scanning = $derived(Object.keys(accounts.progress).length > 0);
	const totals = $derived(
		rows.reduce((t, a) => ({ tokens: t.tokens + a.tokens, week: t.week + a.tokens7d, today: t.today + a.tokensToday }), {
			tokens: 0,
			week: 0,
			today: 0,
		}),
	);
	const sortLabels: Record<SortKey, string> = { name: "Name", recent: "Recently used", tokens: "Tokens" };
</script>

<div class="flex h-full min-h-0" bind:clientWidth={width}>
	<div class="flex min-w-0 flex-1 flex-col">
		<header class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
			<h2 class="me-2 text-[15px] font-semibold">Accounts</h2>
			<div class="bg-muted inline-flex rounded-md p-0.5 text-[12px]" role="tablist">
				{#each [["all", "All"], ["claude", "Claude Code"], ["codex", "Codex"]] as [value, label] (value)}
					<button
						role="tab"
						aria-selected={tool === value}
						class="rounded px-2.5 py-1 {tool === value ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
						onclick={() => (tool = value as ToolFilter)}>{label}</button
					>
				{/each}
			</div>
			<div class="relative ms-auto w-44">
				<Search class="text-muted-foreground pointer-events-none absolute start-2 top-1/2 size-3.5 -translate-y-1/2" />
				<Input bind:value={query} placeholder="Filter" class="h-8 ps-7 text-[13px]" />
			</div>
			<Select.Root type="single" value={sort} onValueChange={(v) => setSort(v as SortKey)}>
				<Select.Trigger class="h-8 w-36 text-[13px]">{sortLabels[sort]}</Select.Trigger>
				<Select.Content>
					{#each Object.entries(sortLabels) as [value, label] (value)}
						<Select.Item {value} {label}>{label}</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
			<label class="text-muted-foreground inline-flex items-center gap-1.5 text-[12px]">
				<Switch bind:checked={showArchived} aria-label="Show archived accounts" />Archived
			</label>
			<Button variant="ghost" size="icon-sm" aria-label="Refresh usage" title="Refresh usage" disabled={scanning} onclick={() => accounts.refreshAll()}>
				<RefreshCw class={scanning ? "animate-spin" : ""} />
			</Button>
			<Button size="sm" onclick={() => (addOpen = true)}><Plus /> Add account</Button>
		</header>

		<div class="min-h-0 flex-1 overflow-auto">
			{#if !accounts.loaded}
				<div class="space-y-2 p-4">
					{#each Array(4) as _, i (i)}<Skeleton class="h-10 w-full" />{/each}
				</div>
			{:else if rows.length === 0}
				<div class="text-muted-foreground grid h-full place-items-center p-8 text-center text-[13px]">
					<div class="space-y-3">
						<Users class="mx-auto size-8 opacity-50" />
						<p>
							{#if query}No account matches “{query}”.{:else if hiddenArchived}All accounts here are archived.{:else}No accounts yet.{/if}
						</p>
						{#if !query}<Button size="sm" variant="outline" onclick={() => (addOpen = true)}><Plus /> Add account</Button>{/if}
					</div>
				</div>
			{:else}
				<Table.Root class="text-[13px]">
					<Table.Header>
						<Table.Row>
							<Table.Head>Account</Table.Head>
							<Table.Head>Plan</Table.Head>
							<Table.Head>5h left</Table.Head>
							<Table.Head>Week left</Table.Head>
							{#if !narrow}
								<Table.Head class="text-end">Tokens</Table.Head>
								<Table.Head class="text-end">7d</Table.Head>
								<Table.Head>Last used</Table.Head>
								<Table.Head class="text-end">Sessions</Table.Head>
							{/if}
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each rows as a (accountKey(a))}
							{@const k = accountKey(a)}
							<Table.Row
								class="cursor-pointer {selected === k ? 'bg-muted/70' : ''} {a.archived ? 'opacity-60' : ''}"
								onclick={() => (selected = selected === k ? null : k)}
							>
								<Table.Cell class="max-w-72">
									<div class="flex min-w-0 items-center gap-2">
										<span class="size-2 shrink-0 rounded-full {a.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
										<div class="min-w-0">
											<div class="flex items-center gap-1.5">
												<span class="truncate font-medium">{a.name}</span>
												{#if a.active}<span class="text-primary text-[11px]">active</span>{/if}
												{#if a.running}<span class="bg-ok size-1.5 rounded-full" title="Running"></span>{/if}
												{#if a.archived}<span class="text-muted-foreground text-[11px]">archived</span>{/if}
											</div>
											<div class="text-muted-foreground truncate text-[12px]">{a.email || a.path}</div>
										</div>
									</div>
									{#if k in accounts.progress}
										<div class="bg-muted mt-1 h-0.5 overflow-hidden rounded-full">
											<div class="bg-primary h-full transition-[width]" style:width="{Math.round(accounts.progress[k] * 100)}%"></div>
										</div>
									{/if}
								</Table.Cell>
								<Table.Cell class="text-muted-foreground">{a.plan || "·"}</Table.Cell>
								<Table.Cell>{#if a.fiveHour}<LimitBar label="" win={a.fiveHour} wide />{:else}<span class="text-muted-foreground">·</span>{/if}</Table.Cell>
								<Table.Cell>{#if a.week}<LimitBar label="" win={a.week} wide />{:else}<span class="text-muted-foreground">·</span>{/if}</Table.Cell>
								{#if !narrow}
									<Table.Cell class="text-end">{a.tokens ? fmtTokens(a.tokens) : "–"}</Table.Cell>
									<Table.Cell class="text-end">{a.tokens7d ? fmtTokens(a.tokens7d) : "–"}</Table.Cell>
									<Table.Cell class="text-muted-foreground">{fmtAgo(a.lastUsed)}</Table.Cell>
									<Table.Cell class="text-end">{a.sessions || "–"}</Table.Cell>
								{/if}
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
				<p class="text-muted-foreground px-4 py-3 text-[12px]">
					{rows.length} account{rows.length === 1 ? "" : "s"} · {fmtTokens(totals.tokens)} tokens · today {fmtTokens(totals.today)} · 7d {fmtTokens(totals.week)}
					{#if !showArchived && hiddenArchived}· {hiddenArchived} archived hidden{/if}
				</p>
			{/if}
		</div>
	</div>

	{#if current && !narrow}
		<aside class="w-80 shrink-0 overflow-y-auto border-s">
			<AccountDetails account={current} onClose={() => (selected = null)} />
		</aside>
	{/if}
</div>

{#if narrow}
	<Sheet.Root open={!!current} onOpenChange={(o) => !o && (selected = null)}>
		<Sheet.Content side="bottom" class="max-h-[80vh] overflow-y-auto p-0">
			{#if current}<AccountDetails account={current} onClose={() => (selected = null)} sheet />{/if}
		</Sheet.Content>
	</Sheet.Root>
{/if}

<AddAccountDialog bind:open={addOpen} defaultProvider={tool === "codex" ? "codex" : "claude"} />
