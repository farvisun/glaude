<script lang="ts">
	// What a Claude session has loaded: context usage, MCP servers, skills,
	// agents, hooks, memory files and the output style.
	import * as Sheet from "$lib/components/ui/sheet";
	import * as Tabs from "$lib/components/ui/tabs";
	import { Button } from "$lib/components/ui/button";
	import { Switch } from "$lib/components/ui/switch";
	import { ui } from "$lib/state/ui.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { convActions, showError, startConversation, type HookRun } from "$lib/state/sessions";
	import { narrow } from "$lib/state/media.svelte";
	import { call } from "$lib/ipc";
	import { fmtAgo, fmtTokens } from "$lib/format";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import ExternalLink from "@lucide/svelte/icons/external-link";
	import Check from "@lucide/svelte/icons/check";

	const TABS = [
		{ id: "context", label: "Context" },
		{ id: "mcp", label: "MCP" },
		{ id: "skills", label: "Skills" },
		{ id: "agents", label: "Agents" },
		{ id: "hooks", label: "Hooks" },
		{ id: "memory", label: "Memory" },
		{ id: "style", label: "Style" },
	];

	type Ctx = {
		categories: { id: string; name: string; tokens: number; kind: string }[];
		totalTokens: number;
		maxTokens: number;
		percentage: number;
		model?: string;
		memoryFiles?: { path: string; type: string; tokens: number }[];
		mcpTools?: { name: string; serverName: string; tokens: number; isLoaded?: boolean }[];
		agents?: { agentType: string; source: string; tokens: number }[];
		isAutoCompactEnabled?: boolean;
		autoCompactThreshold?: number;
	};
	type McpServer = {
		name: string;
		status: string;
		scope?: string;
		source?: string;
		transport?: string;
		error?: string;
		serverInfo?: { name?: string; version?: string };
		tools?: { name: string; description?: string }[];
	};
	type Skill = { name: string; display_name?: string; description?: string; source?: string; tokens?: number; state?: string };
	type Hook = { event: string; matcher?: string; source?: string; sourceLabel?: string; type?: string; displayText?: string; timeout?: number };
	type MemoryDialog = {
		files: { kind: string; path: string; label: string; description?: string; exists: boolean }[];
		folders?: { kind: string; path: string; label: string }[];
		auto_memory?: { enabled: boolean; status: string };
	};

	const conv = $derived(ui.sessionPanel ? conversations.get(ui.sessionPanel.convId) : undefined);
	const tab = $derived(ui.sessionPanel?.tab ?? "context");

	let data = $state<Record<string, unknown>>({});
	let errors = $state<Record<string, string>>({});
	let loading = $state<Record<string, boolean>>({});
	let hookRuns = $state<HookRun[]>([]);
	let openServer = $state<string | null>(null);
	let skillFilter = $state("");

	const requests: Record<string, Record<string, unknown> | null> = {
		context: { subtype: "get_context_usage" },
		mcp: { subtype: "mcp_status" },
		skills: { subtype: "get_skills_dialog" },
		hooks: { subtype: "get_hooks_listing" },
		memory: { subtype: "get_memory_dialog" },
		agents: null,
		style: null,
	};

	async function load(t: string) {
		const req = requests[t];
		if (!conv || !req) return;
		loading[t] = true;
		try {
			data[t] = await convActions.control(conv, req);
			delete errors[t];
			if (t === "hooks") hookRuns = await convActions.hookRuns(conv);
		} catch (e) {
			errors[t] = e instanceof Error ? e.message : String(e);
		} finally {
			loading[t] = false;
		}
	}

	// Load a tab the first time it opens for this conversation.
	let loadedFor = "";
	$effect(() => {
		if (!conv) return;
		if (loadedFor !== conv.id) {
			loadedFor = conv.id;
			data = {};
			errors = {};
		}
		if (data[tab] === undefined && !loading[tab] && !errors[tab]) void load(tab);
	});

	const open = {
		get value() {
			return !!ui.sessionPanel;
		},
		set value(v: boolean) {
			if (!v) ui.sessionPanel = null;
		},
	};
	const setTab = (t: string) => ui.sessionPanel && (ui.sessionPanel = { ...ui.sessionPanel, tab: t });

	const ctx = $derived(data.context as Ctx | undefined);
	const COLORS: Record<string, string> = {
		system_prompt: "#9ca3af",
		system_tools: "#64748b",
		mcp_instructions: "#22c55e",
		mcp_tools: "#16a34a",
		custom_agents: "#3b82f6",
		memory_files: "var(--claude)",
		skills: "#f59e0b",
		messages: "#a855f7",
	};
	const colorOf = (id: string) => COLORS[id] ?? "#94a3b8";
	const used = $derived((ctx?.categories ?? []).filter((c) => c.kind === "used" && c.tokens > 0));
	const buffer = $derived((ctx?.categories ?? []).find((c) => c.kind === "buffer"));
	const deferred = $derived((ctx?.categories ?? []).filter((c) => c.kind === "deferred"));
	const pct = (n: number) => (ctx?.maxTokens ? (n / ctx.maxTokens) * 100 : 0);

	const servers = $derived(((data.mcp as { mcpServers?: McpServer[] } | undefined)?.mcpServers ?? []).slice().sort((a, b) => a.name.localeCompare(b.name)));
	async function mcpAction(req: Record<string, unknown>, done: string) {
		if (!conv) return;
		try {
			await convActions.control(conv, req);
			await load("mcp");
			showDone(done);
		} catch (e) {
			showError("MCP server", e);
		}
	}
	let doneNote = $state("");
	function showDone(text: string) {
		doneNote = text;
		setTimeout(() => doneNote === text && (doneNote = ""), 2500);
	}

	const skills = $derived.by(() => {
		const list = (data.skills as { skills?: Skill[] } | undefined)?.skills ?? [];
		const q = skillFilter.trim().toLowerCase();
		return q ? list.filter((s) => s.name.toLowerCase().includes(q) || s.description?.toLowerCase().includes(q)) : list;
	});

	const hooks = $derived((data.hooks as { hooks?: Hook[] } | undefined)?.hooks ?? []);
	const hookGroups = $derived.by(() => {
		const g: Record<string, Hook[]> = {};
		for (const h of hooks) (g[h.event] ??= []).push(h);
		return Object.entries(g);
	});

	const memory = $derived(data.memory as MemoryDialog | undefined);
	async function openPath(path: string) {
		try {
			await call("open_memory_path", { path });
		} catch (e) {
			showError("Could not open the file", e);
		}
	}

	async function setStyle(style: string) {
		if (!conv) return;
		try {
			await convActions.control(conv, { subtype: "apply_flag_settings", settings: { outputStyle: style } });
			conv.session = { ...conv.session, outputStyle: style };
		} catch (e) {
			showError("Could not change the output style", e);
		}
	}

	function startWithAgent(name: string) {
		if (!conv) return;
		ui.sessionPanel = null;
		void startConversation({ provider: "claude", account: conv.account, cwd: conv.cwd, agent: name, title: name });
	}

	const statusTone = (s: string) =>
		s === "connected" ? "bg-ok" : s === "failed" ? "bg-destructive" : s === "needs-auth" || s === "pending" ? "bg-attention" : "bg-muted-foreground/50";
</script>

{#snippet loadingRows()}
	<div class="flex flex-col gap-2 pt-1">
		{#each [0, 1, 2, 3] as i (i)}<div class="skeleton-shimmer h-9 rounded-md"></div>{/each}
	</div>
{/snippet}

{#snippet failed(t: string)}
	<div class="text-muted-foreground flex flex-col items-center gap-2 py-10 text-center text-[13px]">
		<p>{errors[t]}</p>
		<Button size="sm" variant="secondary" onclick={() => load(t)}>Try again</Button>
	</div>
{/snippet}

{#snippet panel()}
	{#if conv?.provider !== "claude"}
		<p class="text-muted-foreground py-10 text-center text-[13px]">Session details are available for Claude conversations.</p>
	{:else}
		<Tabs.Root value={tab} onValueChange={(v) => setTab(v)} class="flex min-h-0 flex-1 flex-col gap-3">
			<div class="flex items-center gap-2">
				<Tabs.List class="min-w-0 flex-1 flex-wrap justify-start gap-y-0.5 group-data-horizontal/tabs:h-auto">
					{#each TABS as t (t.id)}<Tabs.Trigger value={t.id} class="flex-none px-2 text-xs">{t.label}</Tabs.Trigger>{/each}
				</Tabs.List>
				{#if requests[tab]}
					<Button variant="ghost" size="icon-sm" class="shrink-0" aria-label="Refresh" disabled={loading[tab]} onclick={() => load(tab)}
						><RefreshCw class={loading[tab] ? "animate-spin" : ""} /></Button
					>
				{/if}
			</div>
			{#if doneNote}<div class="text-ok flex items-center gap-1 text-xs"><Check class="size-3" />{doneNote}</div>{/if}
			<div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto pe-1 text-[13px]">
				<Tabs.Content value="context">
					{#if errors.context}{@render failed("context")}
					{:else if !ctx}{@render loadingRows()}
					{:else}
						<div class="flex items-baseline gap-2 pb-2">
							<span class="text-2xl font-semibold">{ctx.percentage}%</span>
							<span class="text-muted-foreground">{fmtTokens(ctx.totalTokens)} of {fmtTokens(ctx.maxTokens)} tokens{ctx.model ? ` · ${ctx.model}` : ""}</span>
						</div>
						<div class="bg-muted flex h-3 overflow-hidden rounded-full" role="img" aria-label="Context usage {ctx.percentage}%">
							{#each used as c (c.id)}<span style="width: {pct(c.tokens)}%; background: {c.id === 'memory_files' ? 'var(--claude)' : colorOf(c.id)}" title="{c.name}: {fmtTokens(c.tokens)}"></span>{/each}
							{#if buffer}<span class="ms-auto opacity-60" style="width: {pct(buffer.tokens)}%; background: repeating-linear-gradient(45deg, var(--muted-foreground) 0 2px, transparent 2px 6px)" title="{buffer.name}"></span>{/if}
						</div>
						<ul class="mt-3 flex flex-col gap-1">
							{#each used as c (c.id)}
								<li class="flex items-center gap-2">
									<span class="size-2.5 shrink-0 rounded-sm" style="background: {colorOf(c.id)}"></span>
									<span>{c.name}</span>
									<span class="text-muted-foreground ms-auto tabular-nums">{fmtTokens(c.tokens)}</span>
								</li>
							{/each}
							{#if buffer}
								<li class="text-muted-foreground flex items-center gap-2">
									<span class="border-muted-foreground size-2.5 shrink-0 rounded-sm border border-dashed"></span>{buffer.name}<span class="ms-auto tabular-nums">{fmtTokens(buffer.tokens)}</span>
								</li>
							{/if}
							{#each deferred as c (c.id)}
								<li class="text-muted-foreground flex items-center gap-2 text-xs">
									<span class="size-2.5 shrink-0"></span>{c.name} · loaded on use<span class="ms-auto tabular-nums">{fmtTokens(c.tokens)}</span>
								</li>
							{/each}
						</ul>
						{#if ctx.memoryFiles?.length}
							<h4 class="text-muted-foreground pt-4 pb-1 text-xs font-medium">Memory files</h4>
							<ul class="flex flex-col gap-0.5">
								{#each ctx.memoryFiles as f (f.path)}
									<li class="flex min-w-0 items-center gap-2 text-xs">
										<span class="bg-muted shrink-0 rounded px-1.5 py-px text-[10px]">{f.type}</span>
										<span class="min-w-0 truncate font-mono" dir="ltr" title={f.path}>{f.path}</span>
										<span class="text-muted-foreground ms-auto shrink-0 tabular-nums">{fmtTokens(f.tokens)}</span>
									</li>
								{/each}
							</ul>
						{/if}
						{#if ctx.isAutoCompactEnabled === false}
							<p class="text-attention pt-3 text-xs">Auto-compact is off for this session.</p>
						{/if}
					{/if}
				</Tabs.Content>

				<Tabs.Content value="mcp">
					{#if errors.mcp}{@render failed("mcp")}
					{:else if !data.mcp}{@render loadingRows()}
					{:else if !servers.length}
						<p class="text-muted-foreground py-10 text-center">No MCP servers are configured for this session.</p>
					{:else}
						<ul class="flex flex-col gap-1.5">
							{#each servers as s (s.name)}
								<li class="border-border rounded-md border">
									<div class="flex min-w-0 items-center gap-2 px-2.5 py-1.5">
										<button class="flex min-w-0 flex-1 items-center gap-2 text-start" onclick={() => (openServer = openServer === s.name ? null : s.name)}>
											<span class="size-2 shrink-0 rounded-full {statusTone(s.status)}" title={s.status}></span>
											<span class="truncate font-medium">{s.name}</span>
											<span class="text-muted-foreground shrink-0 text-xs">{s.status}{s.tools?.length ? ` · ${s.tools.length} tools` : ""}</span>
											<ChevronRight class="text-muted-foreground size-3.5 shrink-0 transition-transform {openServer === s.name ? 'rotate-90' : ''}" />
										</button>
										{#if s.status !== "disabled"}
											<Button size="sm" variant="ghost" class="h-7 px-2 text-xs" onclick={() => mcpAction({ subtype: "mcp_reconnect", serverName: s.name }, `Reconnected ${s.name}`)}
												>Reconnect</Button
											>
										{/if}
										<Switch
											checked={s.status !== "disabled"}
											aria-label="Enable {s.name}"
											onCheckedChange={(on) => mcpAction({ subtype: "mcp_toggle", serverName: s.name, enabled: on }, `${on ? "Enabled" : "Disabled"} ${s.name}`)}
										/>
									</div>
									{#if openServer === s.name}
										<div class="border-border text-muted-foreground flex flex-col gap-1 border-t px-2.5 py-2 text-xs">
											{#if s.error}<p class="text-destructive whitespace-pre-wrap">{s.error}</p>{/if}
											<p>
												{[s.serverInfo?.name && `${s.serverInfo.name}${s.serverInfo.version ? ` ${s.serverInfo.version}` : ""}`, s.transport, s.scope ?? s.source]
													.filter(Boolean)
													.join(" · ")}
											</p>
											{#if s.tools?.length}
												<div class="flex flex-wrap gap-1 pt-1">
													{#each s.tools as t (t.name)}<span class="bg-muted rounded px-1.5 py-px font-mono text-[11px]" title={t.description}>{t.name}</span>{/each}
												</div>
											{/if}
										</div>
									{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="skills">
					{#if errors.skills}{@render failed("skills")}
					{:else if !data.skills}{@render loadingRows()}
					{:else}
						<input
							class="border-input bg-background mb-2 h-8 w-full rounded-md border px-2.5"
							placeholder="Filter skills"
							bind:value={skillFilter}
						/>
						{#if !skills.length}
							<p class="text-muted-foreground py-8 text-center">{skillFilter ? "No skills match." : "No skills are loaded."}</p>
						{:else}
							<ul class="flex flex-col gap-1">
								{#each skills as s (s.name)}
									<li class="hover:bg-accent/40 rounded-md px-2 py-1.5">
										<div class="flex min-w-0 items-center gap-2">
											<span class="truncate font-medium">{s.display_name ?? s.name}</span>
											{#if s.state && s.state !== "on"}<span class="bg-muted rounded px-1.5 py-px text-[10px]">{s.state}</span>{/if}
											<span class="text-muted-foreground ms-auto shrink-0 text-xs">{s.source ?? ""}{s.tokens ? ` · ${fmtTokens(s.tokens)}` : ""}</span>
										</div>
										{#if s.description}<p class="text-muted-foreground line-clamp-2 text-xs" dir="auto">{s.description}</p>{/if}
									</li>
								{/each}
							</ul>
						{/if}
					{/if}
				</Tabs.Content>

				<Tabs.Content value="agents">
					{#if !conv.session.agents?.length}
						<p class="text-muted-foreground py-10 text-center">The agent list loads when the session starts.</p>
					{:else}
						<p class="text-muted-foreground pb-2 text-xs">Mention one with <span class="font-mono">@agent-name</span> in a message, or start a conversation that runs as it.</p>
						<ul class="flex flex-col gap-1">
							{#each conv.session.agents as a (a.name)}
								<li class="group/agent hover:bg-accent/40 rounded-md px-2 py-1.5">
									<div class="flex min-w-0 items-center gap-2">
										<span class="truncate font-medium">{a.name}</span>
										{#if a.model}<span class="bg-muted rounded px-1.5 py-px text-[10px]">{a.model}</span>{/if}
										<button
											class="text-muted-foreground hover:text-foreground ms-auto shrink-0 text-xs opacity-0 transition-opacity group-hover/agent:opacity-100 focus-visible:opacity-100"
											onclick={() => startWithAgent(a.name)}>New conversation</button
										>
									</div>
									{#if a.description}<p class="text-muted-foreground line-clamp-2 text-xs" dir="auto">{a.description}</p>{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="hooks">
					{#if errors.hooks}{@render failed("hooks")}
					{:else if !data.hooks}{@render loadingRows()}
					{:else}
						<h4 class="text-muted-foreground pb-1 text-xs font-medium">Recent runs</h4>
						{#if hookRuns.length}
							<ul class="flex flex-col gap-0.5 pb-3">
								{#each hookRuns.slice(-25).reverse() as r, i (i)}
									<li class="flex min-w-0 items-center gap-2 text-xs" title={r.output}>
										<span class="size-1.5 shrink-0 rounded-full {r.outcome === 'success' ? 'bg-ok' : r.outcome === 'error' || (r.exitCode ?? 0) !== 0 ? 'bg-destructive' : 'bg-muted-foreground/50'}"></span>
										<span class="shrink-0 font-medium">{r.event}</span>
										<span class="text-muted-foreground min-w-0 truncate">{r.name}</span>
										<span class="text-muted-foreground ms-auto shrink-0">{fmtAgo(r.at)}</span>
									</li>
								{/each}
							</ul>
						{:else}
							<p class="text-muted-foreground pb-3 text-xs">No hooks have run in this session yet.</p>
						{/if}
						<h4 class="text-muted-foreground pb-1 text-xs font-medium">Configured</h4>
						{#if !hookGroups.length}
							<p class="text-muted-foreground text-xs">No hooks are configured.</p>
						{:else}
							{#each hookGroups as [event, list] (event)}
								<details class="py-0.5">
									<summary class="cursor-pointer">{event} <span class="text-muted-foreground text-xs">· {list.length}</span></summary>
									<ul class="flex flex-col gap-1 py-1 ps-3">
										{#each list as h, i (i)}
											<li class="text-xs">
												<div class="text-muted-foreground">{h.matcher ? `${h.matcher} · ` : ""}{h.sourceLabel ?? h.source}</div>
												<div class="bg-code truncate rounded px-1.5 py-0.5 font-mono text-[11px]" dir="ltr" title={h.displayText}>{h.displayText}</div>
											</li>
										{/each}
									</ul>
								</details>
							{/each}
						{/if}
					{/if}
				</Tabs.Content>

				<Tabs.Content value="memory">
					{#if errors.memory}{@render failed("memory")}
					{:else if !memory}{@render loadingRows()}
					{:else}
						<ul class="flex flex-col gap-1">
							{#each memory.files as f (f.path)}
								<li class="hover:bg-accent/40 flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5">
									<div class="min-w-0">
										<div class="truncate font-medium">{f.label}</div>
										<div class="text-muted-foreground truncate font-mono text-[11px]" dir="ltr" title={f.path}>{f.path}</div>
									</div>
									{#if f.exists}
										<Button size="sm" variant="ghost" class="ms-auto h-7 shrink-0 px-2 text-xs" onclick={() => openPath(f.path)}><ExternalLink class="size-3" />Open</Button>
									{:else}
										<span class="text-muted-foreground ms-auto shrink-0 text-xs">Not created</span>
									{/if}
								</li>
							{/each}
							{#each memory.folders ?? [] as f (f.path)}
								<li class="hover:bg-accent/40 flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5">
									<div class="min-w-0">
										<div class="truncate font-medium">{f.label}</div>
										<div class="text-muted-foreground truncate font-mono text-[11px]" dir="ltr" title={f.path}>{f.path}</div>
									</div>
									<Button size="sm" variant="ghost" class="ms-auto h-7 shrink-0 px-2 text-xs" onclick={() => openPath(f.path)}><ExternalLink class="size-3" />Open</Button>
								</li>
							{/each}
						</ul>
						{#if memory.auto_memory}
							<p class="text-muted-foreground pt-3 text-xs">Auto memory is {memory.auto_memory.status}.</p>
						{/if}
					{/if}
				</Tabs.Content>

				<Tabs.Content value="style">
					{#if !conv.session.outputStyles?.length}
						<p class="text-muted-foreground py-10 text-center">Output styles load when the session starts.</p>
					{:else}
						<p class="text-muted-foreground pb-2 text-xs">How Claude writes its replies in this session.</p>
						<div class="flex flex-col gap-1" role="radiogroup" aria-label="Output style">
							{#each conv.session.outputStyles as st (st)}
								{@const on = (conv.session.outputStyle ?? "default") === st}
								<button
									class="flex items-center gap-2 rounded-md border px-3 py-2 text-start {on ? 'border-primary bg-primary/10' : 'border-border hover:bg-accent/50'}"
									role="radio"
									aria-checked={on}
									onclick={() => setStyle(st)}
								>
									<span class="font-medium capitalize">{st}</span>
									{#if on}<Check class="text-primary ms-auto size-4" />{/if}
								</button>
							{/each}
						</div>
					{/if}
				</Tabs.Content>
			</div>
		</Tabs.Root>
	{/if}
{/snippet}

<Sheet.Root bind:open={open.value}>
	<Sheet.Content
		side={narrow.current ? "bottom" : "right"}
		class="{narrow.current ? 'h-[85vh]' : 'w-[30rem] sm:max-w-[30rem]'} flex flex-col gap-3 px-4 pb-4"
	>
		<Sheet.Header class="px-0 pb-0">
			<Sheet.Title>Session</Sheet.Title>
			<Sheet.Description class="truncate" dir="auto">{conv?.title ?? ""}</Sheet.Description>
		</Sheet.Header>
		{@render panel()}
	</Sheet.Content>
</Sheet.Root>
