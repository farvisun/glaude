<script lang="ts">
	import { history, type SessionSummary } from "$lib/state/history.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { openSession } from "$lib/state/sessions";
	import { triage, sessionKey, snoozeOptions } from "$lib/state/triage.svelte";
	import { fmtAgo, fmtUntil } from "$lib/format";
	import { toast } from "svelte-sonner";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Plus from "@lucide/svelte/icons/plus";
	import Pin from "@lucide/svelte/icons/pin";
	import PinOff from "@lucide/svelte/icons/pin-off";
	import Ellipsis from "@lucide/svelte/icons/ellipsis";
	import AlarmClock from "@lucide/svelte/icons/alarm-clock";
	import Archive from "@lucide/svelte/icons/archive";
	import ArchiveRestore from "@lucide/svelte/icons/archive-restore";
	import { ui } from "$lib/state/ui.svelte";

	const SHOW = 6;
	let expanded = $state<Record<string, boolean>>({});
	let showSnoozed = $state(false);
	let showArchived = $state(false);

	const openConv = (s: SessionSummary) => openSession(s);

	const running = $derived.by(() => {
		const ids = new Set<string>();
		for (const c of Object.values(conversations.byId)) if (c.alive && c.session.sessionId) ids.add(c.session.sessionId);
		return ids;
	});

	const pinned = $derived(
		history.sessions
			.filter((s) => triage.isPinned(sessionKey(s)) && !triage.hidden(sessionKey(s)))
			.sort((a, b) => triage.pinned[sessionKey(b)] - triage.pinned[sessionKey(a)]),
	);
	const snoozed = $derived(history.sessions.filter((s) => !triage.isArchived(sessionKey(s)) && triage.snoozedUntil(sessionKey(s)) !== null));
	const archived = $derived(history.sessions.filter((s) => triage.isArchived(sessionKey(s))));
	const groups = $derived(
		history.groups
			.map((g) => ({ ...g, sessions: g.sessions.filter((s) => !triage.hidden(sessionKey(s)) && !triage.isPinned(sessionKey(s))) }))
			.filter((g) => g.sessions.length > 0),
	);

	function withUndo(s: SessionSummary, label: string, change: () => void) {
		const k = sessionKey(s);
		const before = triage.stateOf(k);
		change();
		toast(label, { description: s.title, action: { label: "Undo", onClick: () => triage.restore(k, before) } });
	}
	const archive = (s: SessionSummary) => withUndo(s, "Archived", () => triage.archive(sessionKey(s)));
	const snooze = (s: SessionSummary, until: number, label: string) =>
		withUndo(s, `Snoozed until ${label.toLowerCase()}`, () => triage.snooze(sessionKey(s), until));
</script>

{#snippet row(s: SessionSummary, extra?: "snoozed" | "archived")}
	{@const k = sessionKey(s)}
	<li class="group/row relative">
		<button
			class="hover:bg-sidebar-accent flex h-8 w-full min-w-0 items-center gap-2 rounded-md ps-5 pe-2 text-start text-[13px]"
			onclick={() => openConv(s)}
			title={s.title}
		>
			<span
				class="size-1.5 shrink-0 rounded-full {s.provider === 'claude' ? 'bg-claude' : 'bg-codex'} {running.has(s.id)
					? 'ring-primary/40 animate-pulse ring-2'
					: ''}"
			></span>
			<span class="min-w-0 flex-1 truncate text-left" dir="auto">{s.title}</span>
			<span class="text-muted-foreground shrink-0 text-[11px] group-focus-within/row:invisible group-hover/row:invisible"
				>{extra === "snoozed" ? fmtUntil(new Date(triage.snoozed[k]).toISOString()) : fmtAgo(s.updated)}</span
			>
		</button>
		<span
			class="bg-sidebar-accent absolute inset-y-1 end-1 flex items-center gap-0.5 rounded opacity-0 group-focus-within/row:opacity-100 group-hover/row:opacity-100 [@media(hover:none)]:opacity-100"
		>
			{#if extra === "archived"}
				<button class="hover:text-foreground text-muted-foreground grid size-6 place-items-center rounded" aria-label="Unarchive" title="Unarchive" onclick={() => triage.unarchive(k)}
					><ArchiveRestore class="size-3.5" /></button
				>
			{:else if extra === "snoozed"}
				<button class="hover:text-foreground text-muted-foreground grid size-6 place-items-center rounded" aria-label="Wake up" title="Wake up now" onclick={() => triage.wake(k)}
					><AlarmClock class="size-3.5" /></button
				>
			{:else}
				<button
					class="hover:text-foreground text-muted-foreground grid size-6 place-items-center rounded"
					aria-label={triage.isPinned(k) ? "Unpin" : "Pin"}
					title={triage.isPinned(k) ? "Unpin" : "Pin to the top"}
					onclick={() => triage.togglePin(k)}
					>{#if triage.isPinned(k)}<PinOff class="size-3.5" />{:else}<Pin class="size-3.5" />{/if}</button
				>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger class="hover:text-foreground text-muted-foreground grid size-6 place-items-center rounded" aria-label="More actions"
						><Ellipsis class="size-3.5" /></DropdownMenu.Trigger
					>
					<DropdownMenu.Content align="end" class="w-52">
						<DropdownMenu.Sub>
							<DropdownMenu.SubTrigger><AlarmClock /> Snooze</DropdownMenu.SubTrigger>
							<DropdownMenu.SubContent>
								{#each snoozeOptions(new Date(triage.now)) as o (o.label)}
									<DropdownMenu.Item onclick={() => snooze(s, o.until, o.label)}>{o.label}</DropdownMenu.Item>
								{/each}
							</DropdownMenu.SubContent>
						</DropdownMenu.Sub>
						<DropdownMenu.Item onclick={() => archive(s)}><Archive /> Archive</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}
		</span>
	</li>
{/snippet}

{#if history.loading}
	<div class="flex flex-col gap-2 px-1 py-1">
		{#each [0, 1, 2, 3, 4] as i (i)}
			<div class="skeleton-shimmer h-8 rounded-md"></div>
		{/each}
	</div>
{:else if history.sessions.length === 0}
	<div class="text-muted-foreground rounded-md px-2 py-6 text-center text-[13px] leading-relaxed whitespace-normal">
		No sessions yet.<br />Start one with <span class="text-sidebar-foreground font-medium">New conversation</span>.
	</div>
{:else}
	<div class="flex flex-col gap-0.5">
		{#if pinned.length}
			<div class="text-muted-foreground flex h-7 items-center gap-1 ps-1 text-[12px] font-medium"><Pin class="size-3" /> Pinned</div>
			<ul class="mb-1 flex flex-col">
				{#each pinned as s (s.provider + s.id)}{@render row(s)}{/each}
			</ul>
		{/if}
		{#each groups as g (g.cwd)}
			{@const closed = history.collapsed[g.cwd]}
			{@const list = expanded[g.cwd] ? g.sessions : g.sessions.slice(0, SHOW)}
			<div class="group/proj">
				<div class="hover:bg-sidebar-accent/60 flex h-7 items-center gap-1 rounded-md pe-1">
					<button class="flex min-w-0 flex-1 items-center gap-1 ps-1 text-start text-[13px] font-medium" onclick={() => history.toggle(g.cwd)} title={g.cwd}>
						<ChevronRight class="text-muted-foreground size-3.5 shrink-0 transition-transform {closed ? '' : 'rotate-90'}" />
						<span class="truncate">{g.name}</span>
					</button>
					<button
						class="hover:bg-sidebar-accent text-muted-foreground grid size-5 shrink-0 place-items-center rounded opacity-0 group-hover/proj:opacity-100"
						aria-label="New conversation in {g.name}"
						onclick={() => ui.openNewConversation(g.cwd)}><Plus class="size-3.5" /></button
					>
				</div>
				{#if !closed}
					<ul class="flex flex-col">
						{#each list as s (s.provider + s.id)}{@render row(s)}{/each}
						{#if g.sessions.length > SHOW}
							<li>
								<button
									class="text-muted-foreground hover:text-sidebar-foreground ps-5 text-xs"
									onclick={() => (expanded[g.cwd] = !expanded[g.cwd])}
									>{expanded[g.cwd] ? "Show less" : `Show ${g.sessions.length - SHOW} more`}</button
								>
							</li>
						{/if}
					</ul>
				{/if}
			</div>
		{/each}
		{#if snoozed.length || archived.length}
			<div class="text-muted-foreground mt-2 flex flex-col gap-0.5 text-[12px]">
				{#if snoozed.length}
					<button class="hover:text-sidebar-foreground flex h-7 items-center gap-1 ps-1" onclick={() => (showSnoozed = !showSnoozed)}>
						<ChevronRight class="size-3.5 transition-transform {showSnoozed ? 'rotate-90' : ''}" /><AlarmClock class="size-3" /> Snoozed · {snoozed.length}
					</button>
					{#if showSnoozed}<ul class="flex flex-col">{#each snoozed as s (s.provider + s.id)}{@render row(s, "snoozed")}{/each}</ul>{/if}
				{/if}
				{#if archived.length}
					<button class="hover:text-sidebar-foreground flex h-7 items-center gap-1 ps-1" onclick={() => (showArchived = !showArchived)}>
						<ChevronRight class="size-3.5 transition-transform {showArchived ? 'rotate-90' : ''}" /><Archive class="size-3" /> Archived · {archived.length}
					</button>
					{#if showArchived}<ul class="flex flex-col">{#each archived as s (s.provider + s.id)}{@render row(s, "archived")}{/each}</ul>{/if}
				{/if}
			</div>
		{/if}
	</div>
{/if}
