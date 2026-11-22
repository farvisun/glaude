<script lang="ts">
	// Sidebar: agents that need you, are working, or finished while you
	// were elsewhere, each with what it is doing and for how long.
	import { conversations } from "$lib/state/conversations.svelte";
	import { stepOf, byUrgency, fmtElapsed, pendingOf, type Tone } from "$lib/state/activity";
	import { clock } from "$lib/state/clock.svelte";
	import { showConversation, actions } from "$lib/state/commands.svelte";
	import { fmtAgo } from "$lib/format";
	import AgentGlyph from "./AgentGlyph.svelte";
	import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";

	const rows = $derived(
		Object.values(conversations.byId)
			.map((conv) => ({ conv, step: stepOf(conv) }))
			.filter((r) => r.step.tone !== "idle")
			.sort(byUrgency),
	);
	const groups = $derived(
		(
			[
				["Needs you", ["waiting", "error"]],
				["Working", ["working"]],
				["Finished", ["done"]],
			] as [string, Tone[]][]
		)
			.map(([label, tones]) => ({ label, rows: rows.filter((r) => tones.includes(r.step.tone)) }))
			.filter((g) => g.rows.length),
	);
	const live = $derived(rows.some((r) => r.step.tone === "working" || r.step.tone === "waiting"));
	$effect(() => {
		if (live) return clock.subscribe();
	});

	function when(tone: Tone, since: number | null): string {
		if (since == null) return "";
		if (tone === "done" || tone === "error") return fmtAgo(since);
		return fmtElapsed(clock.now - since);
	}
</script>

{#if groups.length}
	<section class="flex flex-col gap-1 pb-2" aria-label="Live agents">
		{#each groups as g, gi (g.label)}
			<div class="text-muted-foreground flex h-6 items-center gap-1 px-1 text-[11px] font-semibold tracking-wide uppercase">
				<span>{g.label}</span>
				<span class="font-normal">· {g.rows.length}</span>
				{#if gi === 0}
					<button
						class="hover:text-sidebar-foreground hover:bg-sidebar-accent ms-auto grid size-5 place-items-center rounded normal-case"
						aria-label="Agents board"
						title="Agents board"
						onclick={() => actions.agents()}><LayoutDashboard class="size-3.5" /></button
					>
				{/if}
			</div>
			<ul class="flex flex-col">
				{#each g.rows as r (r.conv.id)}
					<li>
						<button
							class="hover:bg-sidebar-accent group/live flex w-full min-w-0 items-start gap-2 rounded-md px-1.5 py-1.5 text-start"
							onclick={() => showConversation(r.conv.id, pendingOf(r.conv)?.id)}
							title="{r.conv.title}\n{r.step.text}"
						>
							<AgentGlyph tone={r.step.tone} provider={r.conv.provider} class="mt-px" />
							<span class="flex min-w-0 flex-1 flex-col">
								<span class="flex min-w-0 items-baseline gap-2">
									<span class="min-w-0 flex-1 truncate text-[13px] {r.step.tone === 'done' ? '' : 'font-medium'}" dir="auto"
										>{r.conv.title}</span
									>
									<span class="text-muted-foreground shrink-0 text-[11px] tabular-nums">{when(r.step.tone, r.step.since)}</span>
								</span>
								<span
									class="truncate text-[12px] {r.step.tone === 'waiting'
										? 'text-attention'
										: r.step.tone === 'error'
											? 'text-destructive'
											: 'text-muted-foreground'}"
									dir="auto">{r.step.text}</span
								>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{/each}
	</section>
{/if}
