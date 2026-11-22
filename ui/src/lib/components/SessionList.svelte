<script lang="ts">
	import { history, type SessionSummary } from "$lib/state/history.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { openSession, openSessionSplit, resumeInTerminal, showError } from "$lib/state/sessions";
	import * as ContextMenu from "$lib/components/ui/context-menu";
	import { call } from "$lib/ipc";
	import { prefs, isMac } from "$lib/state/prefs.svelte";
	import { basename } from "$lib/format";
	import Columns2 from "@lucide/svelte/icons/columns-2";
	import Rows2 from "@lucide/svelte/icons/rows-2";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import MessageSquarePlus from "@lucide/svelte/icons/message-square-plus";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Copy from "@lucide/svelte/icons/copy";
	import FolderOpen from "@lucide/svelte/icons/folder-open";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import GitCompare from "@lucide/svelte/icons/git-compare";
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
	import { stepOf } from "$lib/state/activity";
	import { slide } from "$lib/motion";
	import { projects, sortSessions, PROJECT_COLORS, type Project } from "$lib/state/projects.svelte";
	import ProjectIcon from "./ProjectIcon.svelte";
	import FolderPlus from "@lucide/svelte/icons/folder-plus";
	import Settings2 from "@lucide/svelte/icons/settings-2";
	import ArrowUp from "@lucide/svelte/icons/arrow-up";
	import ArrowDown from "@lucide/svelte/icons/arrow-down";
	import Trash from "@lucide/svelte/icons/trash-2";
	import Folders from "@lucide/svelte/icons/folders";

	const SHOW = 6;
	let expanded = $state<Record<string, boolean>>({});
	let showSnoozed = $state(false);
	let showArchived = $state(false);

	const openConv = (s: SessionSummary) => openSession(s);

	// Open agents per project (or folder) that are working or need you.
	const live = $derived.by(() => {
		const out: Record<string, { n: number; waiting: boolean }> = {};
		for (const c of Object.values(conversations.byId)) {
			const tone = stepOf(c).tone;
			if (tone !== "working" && tone !== "waiting") continue;
			const key = projects.of(c.cwd)?.id ?? c.cwd;
			const e = (out[key] ??= { n: 0, waiting: false });
			e.n++;
			e.waiting ||= tone === "waiting";
		}
		return out;
	});

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
	const listed = (s: SessionSummary) => !triage.hidden(sessionKey(s)) && !triage.isPinned(sessionKey(s));

	// The user's projects, in the chosen order, each with its sessions.
	const projectGroups = $derived.by(() => {
		const by = new Map<string, SessionSummary[]>();
		for (const s of history.sessions) {
			if (!listed(s)) continue;
			const p = projects.of(s.cwd);
			if (!p) continue;
			let l = by.get(p.id);
			if (!l) by.set(p.id, (l = []));
			l.push(s);
		}
		const list = projects.list.map((p) => {
			const sessions = by.get(p.id) ?? [];
			return { project: p, sessions: sortSessions(sessions, projects.sessionSort), updated: Math.max(p.created, ...sessions.map((s) => s.updated)) };
		});
		if (projects.sort === "name") list.sort((a, b) => a.project.name.localeCompare(b.project.name, undefined, { sensitivity: "base" }));
		else if (projects.sort === "recent") list.sort((a, b) => b.updated - a.updated);
		return list;
	});
	// Sessions in folders outside every project, per folder.
	const otherGroups = $derived(
		history.groups
			.map((g) => ({ ...g, sessions: sortSessions(g.sessions.filter((s) => listed(s) && !projects.of(s.cwd)), projects.sessionSort) }))
			.filter((g) => g.sessions.length > 0),
	);
	const otherCount = $derived(otherGroups.reduce((n, g) => n + g.sessions.length, 0));
	const otherOpen = $derived(!!history.collapsed["@other-open"]);
	// With no projects yet: one-click projects from recent folders.
	const suggestions = $derived(history.groups.slice(0, 4));

	function quickProject(cwd: string, name: string) {
		const used = new Set(projects.list.map((p) => p.color));
		const color = (PROJECT_COLORS.find((c) => c.id !== "gray" && !used.has(c.id)) ?? PROJECT_COLORS[6]).id;
		const p = projects.add({ name, folders: [cwd], color, defaults: {} });
		toast(`Added ${name}`, { action: { label: "Edit", onClick: () => (ui.projectEdit = p) } });
	}

	function removeProject(p: Project) {
		const at = projects.list.findIndex((x) => x.id === p.id);
		projects.remove(p.id);
		toast(`Removed ${p.name}`, {
			description: "Its conversations moved to Other folders.",
			action: { label: "Undo", onClick: () => projects.restore(p, at) },
		});
	}

	// Drag a project header to reorder (switches the order to manual).
	let dragId = $state<string | null>(null);
	let dropAt = $state<number | null>(null);
	function onProjectDragOver(e: DragEvent, i: number) {
		if (!dragId) return;
		e.preventDefault();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		dropAt = e.clientY < r.top + r.height / 2 ? i : i + 1;
	}
	function onProjectDrop(e: DragEvent) {
		e.preventDefault();
		if (dragId && dropAt != null) {
			// Positions are in the shown order; map them to the stored order.
			const shown = projectGroups.map((g) => g.project.id);
			const target = shown[dropAt];
			const to = target ? projects.list.findIndex((p) => p.id === target) : projects.list.length;
			if (projects.sort !== "manual") {
				projects.list = shown.map((id) => projects.get(id)!);
			}
			projects.move(dragId, target ? projects.list.findIndex((p) => p.id === target) : to);
		}
		dragId = null;
		dropAt = null;
	}
	function moveBy(p: Project, delta: number) {
		const shown = projectGroups.map((g) => g.project.id);
		if (projects.sort !== "manual") projects.list = shown.map((id) => projects.get(id)!);
		const i = projects.list.findIndex((x) => x.id === p.id);
		const to = Math.max(0, Math.min(projects.list.length, i + delta + (delta > 0 ? 1 : 0)));
		projects.move(p.id, to);
	}

	function withUndo(s: SessionSummary, label: string, change: () => void) {
		const k = sessionKey(s);
		const before = triage.stateOf(k);
		change();
		toast(label, { description: s.title, action: { label: "Undo", onClick: () => triage.restore(k, before) } });
	}
	const archive = (s: SessionSummary) => withUndo(s, "Archived", () => triage.archive(sessionKey(s)));
	const snooze = (s: SessionSummary, until: number, label: string) =>
		withUndo(s, `Snoozed until ${label.toLowerCase()}`, () => triage.snooze(sessionKey(s), until));

	async function copyId(s: SessionSummary) {
		try {
			await navigator.clipboard.writeText(s.id);
			toast("Session ID copied", { description: s.id });
		} catch (e) {
			showError("Could not copy", e);
		}
	}
	async function reveal(s: SessionSummary) {
		if (!s.path) return;
		try {
			await call("open_path", { path: s.path, how: "reveal", editor: prefs.editor, line: null, col: null });
		} catch (e) {
			showError("Could not show the transcript", e);
		}
	}
</script>

{#snippet row(s: SessionSummary, extra?: "snoozed" | "archived")}
	{@const k = sessionKey(s)}
	<ContextMenu.Root>
	<ContextMenu.Trigger>
	{#snippet child({ props })}
	<li {...props} class="group/row relative outline-none">
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
	</ContextMenu.Trigger>
	<ContextMenu.Content class="w-60">
		<ContextMenu.Item onclick={() => openConv(s)}><MessageSquare /> Open</ContextMenu.Item>
		<ContextMenu.Item onclick={() => openSessionSplit(s, "right")}><Columns2 /> Open in split right</ContextMenu.Item>
		<ContextMenu.Item onclick={() => openSessionSplit(s, "bottom")}><Rows2 /> Open in split down</ContextMenu.Item>
		<ContextMenu.Item onclick={() => ui.openNewConversation(s.cwd)}
			><MessageSquarePlus /> <span class="truncate">New conversation in {basename(s.cwd) || "this folder"}</span></ContextMenu.Item
		>
		<ContextMenu.Separator />
		{#if extra === "archived"}
			<ContextMenu.Item onclick={() => triage.unarchive(k)}><ArchiveRestore /> Unarchive</ContextMenu.Item>
		{:else}
			<ContextMenu.Item onclick={() => triage.togglePin(k)}
				>{#if triage.isPinned(k)}<PinOff /> Unpin{:else}<Pin /> Pin to the top{/if}</ContextMenu.Item
			>
			{#if extra === "snoozed"}
				<ContextMenu.Item onclick={() => triage.wake(k)}><AlarmClock /> Wake up now</ContextMenu.Item>
			{:else}
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger><AlarmClock /> Snooze</ContextMenu.SubTrigger>
					<ContextMenu.SubContent>
						{#each snoozeOptions(new Date(triage.now)) as o (o.label)}
							<ContextMenu.Item onclick={() => snooze(s, o.until, o.label)}>{o.label}</ContextMenu.Item>
						{/each}
					</ContextMenu.SubContent>
				</ContextMenu.Sub>
			{/if}
			<ContextMenu.Item onclick={() => archive(s)}><Archive /> Archive</ContextMenu.Item>
		{/if}
		<ContextMenu.Item onclick={() => (ui.renameSession = s)}><Pencil /> Rename…</ContextMenu.Item>
		<ContextMenu.Separator />
		<ContextMenu.Item onclick={() => copyId(s)}><Copy /> Copy session ID</ContextMenu.Item>
		<ContextMenu.Item disabled={!s.path} onclick={() => reveal(s)}><FolderOpen /> {isMac ? "Reveal transcript in Finder" : "Show transcript file"}</ContextMenu.Item>
		<ContextMenu.Item onclick={() => resumeInTerminal(s)}
			><SquareTerminal /> Resume in {s.provider === "claude" ? "Claude" : "Codex"} terminal UI</ContextMenu.Item
		>
		<ContextMenu.Item onclick={() => ui.openChanges(s.cwd)}><GitCompare /> Open Changes</ContextMenu.Item>
	</ContextMenu.Content>
	</ContextMenu.Root>
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
		{#if projects.list.length === 0}
			<div class="border-sidebar-border mx-0.5 mb-2 flex flex-col gap-2 rounded-lg border border-dashed px-3 py-3 text-[12px] whitespace-normal">
				<p class="text-muted-foreground leading-relaxed">Group your conversations into projects. Folders without one stay under Other folders.</p>
				<div class="flex flex-wrap gap-1.5">
					{#each suggestions as g (g.cwd)}
						<button
							class="border-sidebar-border hover:bg-sidebar-accent inline-flex h-7 max-w-full items-center gap-1 rounded-md border px-2"
							title="Make {g.cwd} a project"
							onclick={() => quickProject(g.cwd, g.name)}><Plus class="size-3 shrink-0" /><span class="truncate">{g.name}</span></button
						>
					{/each}
				</div>
				<button class="text-link self-start text-[12px] hover:underline" onclick={() => (ui.projectEdit = {})}>New project…</button>
			</div>
		{/if}
		{#each projectGroups as g, i (g.project.id)}
			{@const p = g.project}
			{@const key = `project:${p.id}`}
			{@const closed = history.collapsed[key]}
			{@const list = expanded[key] ? g.sessions : g.sessions.slice(0, SHOW)}
			{@const lv = live[p.id]}
			<div class="group/proj relative">
				{#if dragId && dropAt === i}<div class="bg-primary absolute inset-x-1 -top-px z-10 h-0.5 rounded"></div>{/if}
				{#if dragId && dropAt === i + 1 && i === projectGroups.length - 1}<div class="bg-primary absolute inset-x-1 -bottom-px z-10 h-0.5 rounded"></div>{/if}
				<ContextMenu.Root>
					<ContextMenu.Trigger>
						{#snippet child({ props })}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								{...props}
								class="hover:bg-sidebar-accent/60 flex h-8 items-center gap-1 rounded-md pe-1 outline-none {dragId === p.id ? 'opacity-50' : ''}"
								draggable="true"
								ondragstart={(e) => {
									dragId = p.id;
									e.dataTransfer?.setData("text/plain", p.name);
									if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
								}}
								ondragend={() => {
									dragId = null;
									dropAt = null;
								}}
								ondragover={(e) => onProjectDragOver(e, i)}
								ondrop={onProjectDrop}
							>
								<button
									class="flex min-w-0 flex-1 items-center gap-1.5 ps-1 text-start text-[13px] font-medium"
									onclick={() => history.toggle(key)}
									title={p.folders.join("\n")}
									aria-expanded={!closed}
								>
									<ChevronRight class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-(--dur-2) {closed ? '' : 'rotate-90'}" />
									<ProjectIcon project={p} />
									<span class="truncate">{p.name}</span>
									{#if lv}
										<span
											class="ms-1 inline-flex shrink-0 items-center gap-1 rounded-full px-1.5 text-[11px] font-normal {lv.waiting
												? 'bg-attention/15 text-attention'
												: 'bg-accent text-foreground'}"
											title="{lv.n} {lv.n === 1 ? 'agent' : 'agents'} {lv.waiting ? 'need you' : 'working'}"
											><span class="size-1.5 rounded-full {lv.waiting ? 'bg-attention' : 'bg-ok'}"></span>{lv.n}</span
										>
									{/if}
									{#if closed && g.sessions.length}<span class="text-muted-foreground ms-auto shrink-0 pe-1 text-[11px] font-normal">{g.sessions.length}</span>{/if}
								</button>
								<span class="flex shrink-0 items-center opacity-0 group-hover/proj:opacity-100 focus-within:opacity-100 [@media(hover:none)]:opacity-100">
									<button
										class="hover:bg-sidebar-accent text-muted-foreground hover:text-foreground grid size-6 place-items-center rounded"
										aria-label="New conversation in {p.name}"
										title="New conversation"
										onclick={() => ui.openNewConversation(p.folders[0])}><Plus class="size-3.5" /></button
									>
									<button
										class="hover:bg-sidebar-accent text-muted-foreground hover:text-foreground grid size-6 place-items-center rounded"
										aria-label="Edit {p.name}"
										title="Edit project"
										onclick={() => (ui.projectEdit = p)}><Settings2 class="size-3.5" /></button
									>
								</span>
							</div>
						{/snippet}
					</ContextMenu.Trigger>
					<ContextMenu.Content class="w-56">
						<ContextMenu.Item onclick={() => ui.openNewConversation(p.folders[0])}><MessageSquarePlus /> New conversation</ContextMenu.Item>
						<ContextMenu.Item onclick={() => (ui.projectEdit = p)}><Settings2 /> Edit project…</ContextMenu.Item>
						<ContextMenu.Item onclick={() => ui.openChanges(p.folders[0])}><GitCompare /> Open Changes</ContextMenu.Item>
						<ContextMenu.Separator />
						<ContextMenu.Item disabled={i === 0} onclick={() => moveBy(p, -1)}><ArrowUp /> Move up</ContextMenu.Item>
						<ContextMenu.Item disabled={i === projectGroups.length - 1} onclick={() => moveBy(p, 1)}><ArrowDown /> Move down</ContextMenu.Item>
						<ContextMenu.Separator />
						<ContextMenu.Item variant="destructive" onclick={() => removeProject(p)}><Trash /> Remove project</ContextMenu.Item>
					</ContextMenu.Content>
				</ContextMenu.Root>
				{#if !closed}
					<ul class="flex flex-col" transition:slide={{ duration: 180 }}>
						{#each list as s (s.provider + s.id)}{@render row(s)}{:else}
							<li class="text-muted-foreground flex h-7 items-center ps-7 text-[12px]">No conversations yet</li>
						{/each}
						{#if g.sessions.length > SHOW}
							<li>
								<button class="text-muted-foreground hover:text-sidebar-foreground h-6 ps-7 text-xs" onclick={() => (expanded[key] = !expanded[key])}
									>{expanded[key] ? "Show less" : `Show ${g.sessions.length - SHOW} more`}</button
								>
							</li>
						{/if}
					</ul>
				{/if}
			</div>
		{/each}
		{#if otherGroups.length}
			<div class="mt-1">
				<button
					class="text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent/60 flex h-8 w-full items-center gap-1.5 rounded-md ps-1 text-[13px]"
					onclick={() => history.toggle("@other-open")}
					aria-expanded={otherOpen}
				>
					<ChevronRight class="size-3.5 shrink-0 transition-transform duration-(--dur-2) {otherOpen ? 'rotate-90' : ''}" />
					<Folders class="size-3.5 shrink-0" />
					<span class="truncate">Other folders</span>
					<span class="ms-auto shrink-0 pe-2 text-[11px]">{otherCount}</span>
				</button>
				{#if otherOpen}
					<div class="flex flex-col" transition:slide={{ duration: 180 }}>
						{#each otherGroups as g (g.cwd)}
							{@const key = `other:${g.cwd}`}
							{@const closed = history.collapsed[key]}
							{@const list = expanded[key] ? g.sessions : g.sessions.slice(0, SHOW)}
							<div class="group/proj">
								<div class="hover:bg-sidebar-accent/60 flex h-7 items-center gap-1 rounded-md ps-4 pe-1">
									<button class="flex min-w-0 flex-1 items-center gap-1 text-start text-[12px]" onclick={() => history.toggle(key)} title={g.cwd}>
										<ChevronRight class="text-muted-foreground size-3 shrink-0 transition-transform {closed ? '' : 'rotate-90'}" />
										<span class="truncate">{g.name}</span>
										{#if live[g.cwd]}<span class="bg-ok ms-1 size-1.5 shrink-0 rounded-full" title="Agents working"></span>{/if}
									</button>
									<span class="flex shrink-0 items-center opacity-0 group-hover/proj:opacity-100 focus-within:opacity-100 [@media(hover:none)]:opacity-100">
										<button
											class="hover:bg-sidebar-accent text-muted-foreground hover:text-foreground grid size-6 place-items-center rounded"
											aria-label="Make {g.name} a project"
											title="Make it a project"
											onclick={() => (ui.projectEdit = { folder: g.cwd })}><FolderPlus class="size-3.5" /></button
										>
										<button
											class="hover:bg-sidebar-accent text-muted-foreground hover:text-foreground grid size-6 place-items-center rounded"
											aria-label="New conversation in {g.name}"
											onclick={() => ui.openNewConversation(g.cwd)}><Plus class="size-3.5" /></button
										>
									</span>
								</div>
								{#if !closed}
									<ul class="flex flex-col ps-3" transition:slide={{ duration: 180 }}>
										{#each list as s (s.provider + s.id)}{@render row(s)}{/each}
										{#if g.sessions.length > SHOW}
											<li>
												<button class="text-muted-foreground hover:text-sidebar-foreground h-6 ps-7 text-xs" onclick={() => (expanded[key] = !expanded[key])}
													>{expanded[key] ? "Show less" : `Show ${g.sessions.length - SHOW} more`}</button
												>
											</li>
										{/if}
									</ul>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
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
