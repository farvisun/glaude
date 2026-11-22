<script lang="ts">
	import { history, type SessionSummary } from "$lib/state/history.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { openSession } from "$lib/state/sessions";
	import { fmtAgo } from "$lib/format";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Plus from "@lucide/svelte/icons/plus";
	import { ui } from "$lib/state/ui.svelte";

	const SHOW = 6;
	let expanded = $state<Record<string, boolean>>({});

	const openConv = (s: SessionSummary) => openSession(s);

	function runningIds(): Set<string> {
		const ids = new Set<string>();
		for (const c of Object.values(conversations.byId)) if (c.alive && c.session.sessionId) ids.add(c.session.sessionId);
		return ids;
	}
	const running = $derived(runningIds());
</script>

{#if history.loading}
	<div class="flex flex-col gap-2 px-1 py-1">
		{#each [0, 1, 2, 3, 4] as i (i)}
			<div class="bg-sidebar-accent h-8 animate-pulse rounded-md"></div>
		{/each}
	</div>
{:else if history.sessions.length === 0}
	<div class="text-muted-foreground rounded-md px-2 py-6 text-center text-[13px] leading-relaxed whitespace-normal">
		No sessions yet.<br />Start one with <span class="text-sidebar-foreground font-medium">New conversation</span>.
	</div>
{:else}
	<div class="flex flex-col gap-0.5">
		{#each history.groups as g (g.cwd)}
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
						{#each list as s (s.provider + s.id)}
							<li>
								<button
									class="hover:bg-sidebar-accent flex h-8 w-full min-w-0 items-center gap-2 rounded-md ps-5 pe-2 text-start text-[13px]"
									onclick={() => openConv(s)}
									title={s.title}
								>
									<span class="size-1.5 shrink-0 rounded-full {s.provider === 'claude' ? 'bg-claude' : 'bg-codex'} {running.has(s.id) ? 'ring-primary/40 animate-pulse ring-2' : ''}"></span>
									<span class="min-w-0 flex-1 truncate text-left" dir="auto">{s.title}</span>
									<span class="text-muted-foreground shrink-0 text-[11px]">{fmtAgo(s.updated)}</span>
								</button>
							</li>
						{/each}
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
	</div>
{/if}
