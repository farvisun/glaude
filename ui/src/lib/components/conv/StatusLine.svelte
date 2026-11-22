<script lang="ts">
	import type { Conversation } from "$lib/state/conversations.svelte";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import LimitBar from "./LimitBar.svelte";
	import { basename, fmtCost, fmtDuration, fmtTokens } from "$lib/format";
	import GitBranch from "@lucide/svelte/icons/git-branch";
	import Folder from "@lucide/svelte/icons/folder";
	import User from "@lucide/svelte/icons/user";
	import Timer from "@lucide/svelte/icons/timer";
	import PanelRight from "@lucide/svelte/icons/panel-right";
	import { prefs } from "$lib/state/prefs.svelte";

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
			<span title="Context used: {fmtTokens(s.contextUsed)} of {fmtTokens(s.contextWindow)}"
				>ctx <span class="{ctxPct >= 90 ? 'text-destructive' : ctxPct >= 70 ? 'text-attention' : 'text-foreground'}"
					>{ctxPct}%</span
				></span
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
			{#if s.gitBranch}<span class="inline-flex items-center gap-1"><GitBranch class="size-3" />{s.gitBranch}</span>{/if}
			<span class="inline-flex max-w-48 shrink-0 items-center gap-1" title={conv.cwd}><Folder class="size-3 shrink-0" /><span class="truncate"
					>{basename(conv.cwd)}</span
				></span
			>
			<span class="inline-flex min-w-0 items-center gap-1"><User class="size-3 shrink-0" /><span class="truncate">{conv.account}</span></span>
		</span>
	{/if}
	<button
		class="hover:text-foreground shrink-0 {scriptLine ? 'ms-auto' : ''} {prefs.progressPanel ? 'text-foreground' : ''}"
		aria-label="Toggle progress panel"
		onclick={() => {
			prefs.progressPanel = !prefs.progressPanel;
			prefs.save();
		}}><PanelRight class="size-3.5" /></button
	>
</div>
