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
	import PanelRight from "@lucide/svelte/icons/panel-right";
	import PanelBottom from "@lucide/svelte/icons/panel-bottom";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import { openInTerminal } from "$lib/state/sessions";
	import { prefs, isMac } from "$lib/state/prefs.svelte";
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
</script>

<div class="text-muted-foreground flex h-7 shrink-0 items-center gap-3 overflow-hidden px-3 text-[12px] whitespace-nowrap">
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

		<LimitBar label="5h" win={conv.limits.fiveHour} />
		<LimitBar label="7d" win={conv.limits.week} />

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
			<ProjectActions cwd={conv.cwd} />
			<span class="inline-flex max-w-48 shrink-0 items-center gap-1" title={conv.cwd}><Folder class="size-3 shrink-0" /><span class="truncate"
					>{basename(conv.cwd)}</span
				></span
			>
			<span class="inline-flex min-w-0 items-center gap-1"><User class="size-3 shrink-0" /><span class="truncate">{conv.account}</span></span>
		</span>
	{/if}
	{#if conv.provider === "claude"}
		<button
			class="hover:text-foreground shrink-0 {scriptLine ? 'ms-auto' : ''}"
			aria-label="Session details"
			title="Session: context, MCP, skills, agents, hooks, memory, style"
			onclick={() => (ui.sessionPanel = { convId: conv.id })}><Layers class="size-3.5" /></button
		>
	{/if}
	<button
		class="hover:text-foreground shrink-0 {scriptLine && conv.provider !== 'claude' ? 'ms-auto' : ''} disabled:opacity-40"
		aria-label="Open in the {conv.provider === 'claude' ? 'Claude' : 'Codex'} terminal UI"
		title="Open in the {conv.provider === 'claude' ? 'Claude' : 'Codex'} terminal UI"
		disabled={!s.sessionId}
		onclick={() => openInTerminal(conv)}><SquareTerminal class="size-3.5" /></button
	>
	<button
		class="hover:text-foreground shrink-0 {conv.drawer ? 'text-foreground' : ''}"
		aria-label="Toggle terminal"
		title="Terminal in {basename(conv.cwd)} (Ctrl+&#96;)"
		onclick={() => (conv.drawer = !conv.drawer)}><PanelBottom class="size-3.5" /></button
	>
	<button
		class="hover:text-foreground shrink-0 {prefs.progressPanel ? 'text-foreground' : ''}"
		aria-label="Toggle progress panel"
		onclick={() => {
			prefs.progressPanel = !prefs.progressPanel;
			prefs.save();
		}}><PanelRight class="size-3.5" /></button
	>
</div>
