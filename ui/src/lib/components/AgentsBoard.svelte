<script lang="ts">
	// Every open agent in one place, grouped by what it needs: approvals and
	// questions can be answered right here; ⇧⌘A jumps to the next one.
	import { conversations, type Conversation } from "$lib/state/conversations.svelte";
	import { stepOf, byUrgency, fmtElapsed, pendingOf, lastReply, type Tone } from "$lib/state/activity";
	import { clock } from "$lib/state/clock.svelte";
	import { showConversation } from "$lib/state/commands.svelte";
	import { convActions } from "$lib/state/sessions";
	import { ui } from "$lib/state/ui.svelte";
	import { basename, fmtAgo, fmtCost } from "$lib/format";
	import { isMac } from "$lib/state/prefs.svelte";
	import AgentGlyph from "./AgentGlyph.svelte";
	import PermissionCard from "./conv/PermissionCard.svelte";
	import QuestionCard from "./conv/QuestionCard.svelte";
	import PlanApprovalCard from "./conv/PlanApprovalCard.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Kbd } from "$lib/components/ui/kbd";
	import Plus from "@lucide/svelte/icons/plus";
	import SortMenu from "./SortMenu.svelte";
	import Square from "@lucide/svelte/icons/square";
	import ArrowUpRight from "@lucide/svelte/icons/arrow-up-right";

	type BoardSort = "urgency" | "recent" | "name";
	const SORT_KEY = "glaude.board.sort";
	let sort = $state<BoardSort>((localStorage.getItem(SORT_KEY) as BoardSort) || "urgency");
	function setSort(v: BoardSort) {
		sort = v;
		localStorage.setItem(SORT_KEY, v);
	}
	const rows = $derived.by(() => {
		const list = Object.values(conversations.byId).map((conv) => ({ conv, step: stepOf(conv) }));
		if (sort === "name") return list.sort((a, b) => a.conv.title.localeCompare(b.conv.title, undefined, { sensitivity: "base" }));
		if (sort === "recent") return list.sort((a, b) => b.conv.lastActivity - a.conv.lastActivity);
		return list.sort(byUrgency);
	});
	const columns = $derived(
		(
			[
				["Needs you", ["waiting", "error"], "Nothing is waiting for you."],
				["Working", ["working"], "No agent is working right now."],
				["Finished", ["done"], "Finished agents you haven't looked at show up here."],
			] as [string, Tone[], string][]
		).map(([label, tones, empty]) => ({ label, empty, rows: rows.filter((r) => tones.includes(r.step.tone)) })),
	);
	const idle = $derived(rows.filter((r) => r.step.tone === "idle"));
	$effect(() => {
		if (rows.some((r) => r.step.tone === "working" || r.step.tone === "waiting")) return clock.subscribe();
	});

	const label = (c: Conversation) => (c.provider === "codex" ? "Codex" : "Claude");
	function when(tone: Tone, since: number | null) {
		if (since == null) return "";
		return tone === "working" || tone === "waiting" ? fmtElapsed(clock.now - since) : fmtAgo(since);
	}
	const mod = isMac ? "⌘" : "Ctrl";
</script>

{#snippet card(conv: Conversation, step: ReturnType<typeof stepOf>)}
	{@const pending = step.tone === "waiting" ? pendingOf(conv) : undefined}
	{@const reply = pending ? "" : lastReply(conv)}
	<article
		class="bg-card border-border flex flex-col gap-2 rounded-lg border p-3 text-[13px] shadow-xs {step.tone === 'waiting'
			? 'border-attention/50'
			: ''}"
	>
		<header class="flex min-w-0 items-start gap-2">
			<AgentGlyph tone={step.tone} provider={conv.provider} class="mt-0.5" />
			<button class="min-w-0 flex-1 text-start" onclick={() => showConversation(conv.id, pending?.id)}>
				<span class="block truncate font-medium hover:underline" dir="auto">{conv.title}</span>
				<span class="text-muted-foreground block truncate text-[12px]"
					>{basename(conv.cwd)} · {label(conv)}{conv.account && conv.account !== "default" ? ` · ${conv.account}` : ""}</span
				>
			</button>
			<span class="text-muted-foreground shrink-0 text-[12px] tabular-nums">{when(step.tone, step.since)}</span>
		</header>
		<div
			class="truncate {step.tone === 'waiting' ? 'text-attention font-medium' : step.tone === 'error' ? 'text-destructive' : ''}"
			dir="auto"
		>
			{step.text}
		</div>
		{#if pending}
			<div class="scrollbar-thin max-h-96 overflow-y-auto">
				{#if pending.kind === "question"}
					<QuestionCard
						item={pending}
						providerLabel={label(conv)}
						onAnswer={(answers) => convActions.answerQuestion(conv, pending, answers)}
						onSkip={() => convActions.decide(conv, pending, "deny", "The user skipped the question.")}
					/>
				{:else if pending.tool === "ExitPlanMode"}
					<PlanApprovalCard item={pending} onDecide={(it, opt, msg) => convActions.decide(conv, it, opt, msg)} />
				{:else}
					<PermissionCard item={pending} providerLabel={label(conv)} onDecide={(it, opt, msg) => convActions.decide(conv, it, opt, msg)} />
				{/if}
			</div>
		{:else if reply}
			<p class="text-muted-foreground line-clamp-3 text-[12.5px] leading-relaxed" dir="auto">{reply}</p>
		{/if}
		<footer class="text-muted-foreground flex items-center gap-2 text-[12px]">
			{#if conv.session.costUsd}<span>{fmtCost(conv.session.costUsd)}</span>{/if}
			{#if conv.runningTasks.length}<span>{conv.runningTasks.length} background</span>{/if}
			<span class="ms-auto flex items-center gap-1">
				{#if conv.running}
					<Button variant="ghost" size="sm" class="h-7" onclick={() => convActions.interrupt(conv)}><Square class="size-3" /> Stop</Button>
				{/if}
				<Button variant="outline" size="sm" class="h-7" onclick={() => showConversation(conv.id, pending?.id)}
					>Open <ArrowUpRight class="size-3" /></Button
				>
			</span>
		</footer>
	</article>
{/snippet}

<div class="@container scrollbar-thin h-full overflow-y-auto">
	<div class="mx-auto flex max-w-7xl flex-col gap-5 px-5 py-5">
		<header class="flex flex-wrap items-center gap-3">
			<h1 class="text-lg font-semibold">Agents</h1>
			<span class="text-muted-foreground text-[13px]"
				>{rows.length} open · <Kbd>⇧{mod}A</Kbd> jumps to the next one waiting for you</span
			>
			<span class="ms-auto">
				<SortMenu
					value={sort}
					options={[
						{ value: "urgency", label: "Most urgent" },
						{ value: "recent", label: "Recent activity" },
						{ value: "name", label: "Name" },
					]}
					onChange={setSort}
				/>
			</span>
			<Button size="sm" onclick={() => ui.openNewConversation()}><Plus /> New conversation</Button>
		</header>

		{#if rows.length === 0}
			<div class="text-muted-foreground flex flex-col items-center gap-3 py-24 text-center text-[13px]">
				<p>No agents are open. Start a conversation and it shows up here, with what it is doing and when it needs you.</p>
				<Button size="sm" onclick={() => ui.openNewConversation()}><Plus /> New conversation</Button>
			</div>
		{:else}
			<div class="grid gap-4 @4xl:grid-cols-[1.35fr_1fr_1fr]">
				{#each columns as col (col.label)}
					<section class="flex min-w-0 flex-col gap-2" aria-label={col.label}>
						<h2 class="text-muted-foreground flex items-center gap-1.5 text-[11px] font-semibold tracking-wide uppercase">
							{col.label} <span class="font-normal">· {col.rows.length}</span>
						</h2>
						{#each col.rows as r (r.conv.id)}
							{@render card(r.conv, r.step)}
						{:else}
							<p class="text-muted-foreground border-border rounded-lg border border-dashed px-3 py-6 text-center text-[12.5px]">{col.empty}</p>
						{/each}
					</section>
				{/each}
			</div>
			{#if idle.length}
				<section class="flex flex-col gap-2" aria-label="Idle">
					<h2 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Idle <span class="font-normal">· {idle.length}</span></h2>
					<ul class="border-border divide-border divide-y rounded-lg border">
						{#each idle as r (r.conv.id)}
							<li>
								<button
									class="hover:bg-accent/40 flex w-full min-w-0 items-center gap-2 px-3 py-2 text-start text-[13px]"
									onclick={() => showConversation(r.conv.id)}
								>
									<AgentGlyph tone="idle" provider={r.conv.provider} />
									<span class="min-w-0 flex-1 truncate" dir="auto">{r.conv.title}</span>
									<span class="text-muted-foreground hidden truncate text-[12px] @md:inline">{basename(r.conv.cwd)}</span>
									<span class="text-muted-foreground shrink-0 text-[12px]">{r.step.text}{r.step.since ? ` · ${fmtAgo(r.step.since)}` : ""}</span>
								</button>
							</li>
						{/each}
					</ul>
				</section>
			{/if}
		{/if}
	</div>
</div>
