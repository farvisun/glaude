<script lang="ts">
	import Layers from "@lucide/svelte/icons/layers";
	import { ui } from "$lib/state/ui.svelte";
	import type { Conversation } from "$lib/state/conversations.svelte";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import LimitBar from "./LimitBar.svelte";
	import { basename, fmtCost, fmtDuration, fmtTokens } from "$lib/format";
	import GitBranch from "@lucide/svelte/icons/git-branch";
	import Folder from "@lucide/svelte/icons/folder";
	import User from "@lucide/svelte/icons/user";
	import Timer from "@lucide/svelte/icons/timer";
	import PanelBottom from "@lucide/svelte/icons/panel-bottom";
	import Shield from "@lucide/svelte/icons/shield";
	import Check from "@lucide/svelte/icons/check";
	import { convActions } from "$lib/state/sessions";
	import { proxies, sourceLabel } from "$lib/state/proxies.svelte";
	import { termPanel } from "$lib/state/termpanel.svelte";
	import { layout } from "$lib/state/layout.svelte";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import { openInTerminal } from "$lib/state/sessions";
	import { isMac } from "$lib/state/prefs.svelte";
	import ProjectActions from "./ProjectActions.svelte";

	let {
		conv,
		modes,
		onMode,
		onModel,
		scriptLine,
	}: {
		conv: Conversation;
		modes: { value: string; label: string }[];
		onMode: (mode: string) => void;
		onModel: (model: string) => void;
		scriptLine?: string | null;
	} = $props();

	let now = $state(Date.now());
	$effect(() => {
		if (!conv.turnStartedAt) return;
		now = Date.now();
		const t = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(t);
	});

	const s = $derived(conv.session);
	const ctxPct = $derived(s.contextUsed != null && s.contextWindow ? Math.round((s.contextUsed / s.contextWindow) * 100) : null);
	const modeLabel = $derived(modes.find((m) => m.value === s.permissionMode)?.label ?? s.permissionMode ?? "");
	const elapsed = $derived(conv.turnStartedAt ? Math.max(0, now - conv.turnStartedAt) : conv.lastTurnMs);

	const active = $derived(proxies.byId(conv.proxyActive));
	const fallback = $derived(proxies.defaultFor(conv));
	const showProxy = $derived(proxies.list.length > 0 || !!conv.proxyActive);
	function networkSettings() {
		ui.settingsSection = "network";
		layout.openTab({ kind: "settings", title: "Settings" });
	}
	const iconBtn = "hover:bg-accent hover:text-foreground grid size-6 shrink-0 place-items-center rounded disabled:pointer-events-none disabled:opacity-40";
</script>

<div class="statusline @container/status border-border/70 text-muted-foreground flex h-(--bar-h) shrink-0 items-center gap-3 overflow-hidden border-t ps-3 pe-1.5 text-[12px] whitespace-nowrap">
	<!-- Overflows on the left so the buttons on the right stay visible. -->
	<div class="flex min-w-0 flex-1 items-center gap-3 overflow-hidden">
		{#if scriptLine}
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			<span class="truncate font-mono" dir="ltr">{@html scriptLine}</span>
		{:else}
			<span class="inline-flex items-center gap-1.5">
				<span class="size-2 rounded-full {conv.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
				<span class="text-foreground font-medium">{conv.provider === "claude" ? "Claude" : "Codex"}</span>
			</span>

			<DropdownMenu.Root>
				<DropdownMenu.Trigger class="hover:text-foreground">{s.model ?? "model"}{s.effort ? ` · ${s.effort}` : ""}</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start">
					{#each s.models ?? [] as m (m.value)}
						<DropdownMenu.Item onclick={() => onModel(m.value)}>{m.label}</DropdownMenu.Item>
					{:else}
						<DropdownMenu.Item disabled>Model list loads when the session starts</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			{#if modes.length}
				<DropdownMenu.Root>
					<DropdownMenu.Trigger class="hover:text-foreground {s.permissionMode === 'bypassPermissions' || s.permissionMode === 'full'
							? 'text-attention'
							: ''}">{modeLabel || "mode"}</DropdownMenu.Trigger
					>
					<DropdownMenu.Content align="start">
						{#each modes as m (m.value)}
							<DropdownMenu.Item onclick={() => onMode(m.value)}>{m.label}</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}

			{#if ctxPct != null}
				<button
					class="hover:text-foreground disabled:cursor-default"
					disabled={conv.provider !== "claude"}
					title="Context used: {fmtTokens(s.contextUsed)} of {fmtTokens(s.contextWindow)}"
					onclick={() => (ui.sessionPanel = { convId: conv.id, tab: "context" })}
					>ctx <span class="{ctxPct >= 90 ? 'text-destructive' : ctxPct >= 70 ? 'text-attention' : 'text-foreground'}"
						>{ctxPct}%</span
					></button
				>
			{/if}
			{#if s.costUsd != null}<span >{fmtCost(s.costUsd)}</span>{/if}
			{#if s.linesAdded || s.linesRemoved}
				<span ><span class="text-ok">+{s.linesAdded ?? 0}</span> <span class="text-destructive">−{s.linesRemoved ?? 0}</span></span>
			{/if}

			<LimitBar label="5h" win={conv.limits.fiveHour} resetClass="hidden @5xl/status:inline" />
			<span class="hidden @3xl/status:contents"><LimitBar label="7d" win={conv.limits.week} resetClass="hidden @5xl/status:inline" /></span>

			<span class="ms-auto inline-flex min-w-0 items-center gap-3 overflow-hidden">
				{#if elapsed}
					<span class="inline-flex items-center gap-1 {conv.turnStartedAt ? 'text-foreground' : ''}"
						><Timer class="size-3" />{fmtDuration(elapsed)}</span
					>
				{/if}
				{#if s.gitBranch}<button
						class="hover:text-foreground inline-flex items-center gap-1"
						title="Changes, commit, push and pull request ({isMac ? '⌘⇧G' : 'Ctrl+Shift+G'})"
						onclick={() => ui.openChanges(conv.cwd)}><GitBranch class="size-3" />{s.gitBranch}</button
					>{/if}
				<span class="hidden @4xl/status:contents"><ProjectActions cwd={conv.cwd} /></span>
				<span class="hidden max-w-48 shrink-0 @5xl/status:inline-flex items-center gap-1" title={conv.cwd}><Folder class="size-3 shrink-0" /><span class="truncate"
						>{basename(conv.cwd)}</span
					></span
				>
				<span class="hidden min-w-0 items-center gap-1 @4xl/status:inline-flex"><User class="size-3 shrink-0" /><span class="truncate">{conv.account}</span></span>
			</span>
		{/if}
	</div>
	<span class="flex shrink-0 items-center">
		{#if showProxy}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger
					class="hover:text-foreground me-1 inline-flex h-6 max-w-36 min-w-0 items-center gap-1 rounded px-1.5 hover:bg-accent {active ? 'text-foreground' : ''}"
					title={active ? `Through ${active.name}` : "Connects directly"}
					><Shield class="size-3 shrink-0" /><span class="truncate">{active ? active.name : "No proxy"}</span></DropdownMenu.Trigger
				>
				<DropdownMenu.Content align="end" class="w-64">
					<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">Proxy for this conversation</DropdownMenu.Label>
					<DropdownMenu.Item onclick={() => convActions.setProxy(conv, undefined)}>
						<Check class={conv.proxy === undefined ? "" : "invisible"} />
						<span class="flex min-w-0 flex-col">
							<span class="truncate">Default · {fallback.proxy?.name ?? "No proxy"}</span>
							{#if fallback.source}<span class="text-muted-foreground text-xs">From the {sourceLabel[fallback.source]}</span>{/if}
						</span>
					</DropdownMenu.Item>
					{#each proxies.list as p (p.id)}
						<DropdownMenu.Item onclick={() => convActions.setProxy(conv, p.id)}>
							<Check class={conv.proxy === p.id ? "" : "invisible"} /><span class="truncate">{p.name}</span>
						</DropdownMenu.Item>
					{/each}
					<DropdownMenu.Item onclick={() => convActions.setProxy(conv, "none")}>
						<Check class={conv.proxy === "none" ? "" : "invisible"} />No proxy
					</DropdownMenu.Item>
					<DropdownMenu.Separator />
					<DropdownMenu.Item onclick={networkSettings}>Network settings…</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}

	{#if conv.provider === "claude"}
		<button
			class={iconBtn}
			aria-label="Session details"
			title="Session: context, MCP, skills, agents, hooks, memory, style"
			onclick={() => (ui.sessionPanel = { convId: conv.id })}><Layers class="size-3.5" /></button
		>
	{/if}
	<button
		class={iconBtn}
		aria-label="Open in the {conv.provider === 'claude' ? 'Claude' : 'Codex'} terminal UI"
		title="Open in the {conv.provider === 'claude' ? 'Claude' : 'Codex'} terminal UI"
		disabled={!s.sessionId}
		onclick={() => openInTerminal(conv)}><SquareTerminal class="size-3.5" /></button
	>
	<button
		class="{iconBtn} {termPanel.open ? 'text-foreground bg-accent/60' : ''}"
		aria-label="Toggle terminal panel"
		aria-pressed={termPanel.open}
		title="Terminal panel (Ctrl+&#96;)"
		onclick={() => termPanel.toggle()}><PanelBottom class="size-3.5" /></button
	>
	</span>
</div>
