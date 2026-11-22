<script lang="ts">
	// Usage: what agents cost and did, from the turns run in Glaude, plus
	// the 5-hour and weekly limits of every account.
	import { usage, totals, byDay, groupBy, since, startOfToday, type TurnRecord } from "$lib/state/usage.svelte";
	import { accounts, accountKey } from "$lib/state/accounts.svelte";
	import { basename, fmtCost, fmtTokens } from "$lib/format";
	import { fmtElapsed } from "$lib/state/activity";
	import { showConversation } from "$lib/state/commands.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import Segmented from "./settings/Segmented.svelte";
	import LimitBar from "./conv/LimitBar.svelte";
	import { Button } from "$lib/components/ui/button";
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { onMount } from "svelte";
	import SortTh from "./SortTh.svelte";
	import { projects as userProjects } from "$lib/state/projects.svelte";

	type Range = "7" | "30" | "90";
	type Metric = "cost" | "tokens" | "time";
	const RANGE_KEY = "glaude.usage.range";
	let range = $state<Range>((localStorage.getItem(RANGE_KEY) as Range) || "30");
	let metric = $state<Metric>("cost");
	let asTable = $state(false);
	let confirmClear = $state(false);
	$effect(() => localStorage.setItem(RANGE_KEY, range));

	onMount(() => {
		if (!accounts.list.length) void accounts.load();
	});

	const days = $derived(Number(range));
	const recs = $derived(since(usage.records, days * 86_400_000));
	const sum = $derived(totals(recs));
	const today = $derived(totals(usage.records.filter((r) => r.ts >= startOfToday())));
	const daily = $derived(byDay(recs, days));
	const valueOf = (t: ReturnType<typeof totals>) => (metric === "cost" ? t.costUsd : metric === "tokens" ? t.tokens : t.durationMs);
	const fmtValue = (v: number) => (metric === "cost" ? fmtCost(v) : metric === "tokens" ? fmtTokens(v) : fmtElapsed(v));
	const max = $derived(Math.max(...daily.map((d) => valueOf(d.totals)), 0));
	// Three recessive gridlines at round-ish steps.
	const ticks = $derived(max > 0 ? [max / 3, (2 * max) / 3, max] : []);

	// Both tables sort by any column (click its header); the default is the
	// measure picked above.
	type Col = "name" | "turns" | "tokens" | "time" | "cost";
	let projSort = $state<{ col: Col | null; desc: boolean }>({ col: null, desc: true });
	let convSort = $state<{ col: Col | null; desc: boolean }>({ col: null, desc: true });
	const flip = (cur: { col: Col | null; desc: boolean }, col: Col) => ({ col, desc: cur.col === col ? !cur.desc : col !== "name" });
	type Group = { key: string; records: TurnRecord[]; totals: ReturnType<typeof totals> };
	function sorted(list: Group[], s: { col: Col | null; desc: boolean }, name: (g: Group) => string): Group[] {
		const v = (g: Group): number | string =>
			s.col === "name"
				? name(g).toLowerCase()
				: s.col === "turns"
					? g.totals.turns
					: s.col === "tokens"
						? g.totals.tokens
						: s.col === "time"
							? g.totals.durationMs
							: s.col === "cost"
								? g.totals.costUsd
								: valueOf(g.totals);
		return [...list].sort((a, b) => {
			const x = v(a);
			const y = v(b);
			const c = typeof x === "string" ? x.localeCompare(y as string) : x - (y as number);
			return s.col === null || s.desc ? -c : c;
		});
	}
	// Grouped by the user's projects; folders outside them by folder.
	const projectName = (g: Group) => userProjects.get(g.key)?.name ?? basename(g.key);
	const projects = $derived(sorted(groupBy(recs, (r) => userProjects.of(r.cwd)?.id ?? r.cwd), projSort, projectName).slice(0, 12));
	const convs = $derived(sorted(groupBy(recs, (r) => r.convId), convSort, (g) => title(g.records)).slice(0, 10));
	const first = $derived(usage.records[0]?.ts ?? null);
	const codexOnly = $derived(recs.length > 0 && recs.every((r) => r.provider === "codex"));

	let hover = $state<{ i: number; x: number } | null>(null);
	let chartEl = $state<HTMLDivElement>();
	function showTip(i: number, el: HTMLElement) {
		if (!chartEl) return;
		const a = el.getBoundingClientRect();
		const box = chartEl.getBoundingClientRect();
		// Keep the tooltip inside the chart on narrow windows.
		const x = a.left - box.left + a.width / 2;
		hover = { i, x: Math.min(Math.max(x, 80), box.width - 80) };
	}
	const dayLabel = (k: string, opts: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" }) =>
		new Date(`${k}T12:00:00`).toLocaleDateString(undefined, opts);

	const title = (r: TurnRecord[]) => r[r.length - 1]?.title ?? "";
	const accountList = $derived(
		accounts.list
			.filter((a) => !("archived" in a && (a as { archived?: boolean }).archived))
			.map((a) => accounts.withLive(a)),
	);
</script>

{#snippet tile(label: string, value: string, sub?: string)}
	<div class="bg-card border-border flex min-w-0 flex-col gap-0.5 rounded-lg border px-4 py-3">
		<span class="text-muted-foreground text-[12px]">{label}</span>
		<span class="truncate text-xl font-semibold">{value}</span>
		{#if sub}<span class="text-muted-foreground truncate text-[12px]">{sub}</span>{/if}
	</div>
{/snippet}

<div class="@container scrollbar-thin h-full overflow-y-auto">
	<div class="mx-auto flex max-w-6xl flex-col gap-5 px-5 py-5 text-[13px]">
		<header class="flex flex-wrap items-center gap-3">
			<h1 class="text-lg font-semibold">Usage</h1>
			<div class="ms-auto flex flex-wrap items-center gap-2">
				<Segmented
					label="Period"
					value={range}
					options={[
						{ value: "7", label: "7 days" },
						{ value: "30", label: "30 days" },
						{ value: "90", label: "90 days" },
					]}
					onChange={(v) => (range = v)}
				/>
				<Segmented
					label="Measure"
					value={metric}
					options={[
						{ value: "cost", label: "Cost" },
						{ value: "tokens", label: "Tokens" },
						{ value: "time", label: "Agent time" },
					]}
					onChange={(v) => (metric = v)}
				/>
			</div>
		</header>

		{#if usage.records.length === 0}
			<div class="text-muted-foreground border-border rounded-lg border border-dashed px-4 py-12 text-center">
				No turns yet. Every turn an agent finishes in Glaude is counted here: time, cost, tokens and files changed.
			</div>
		{:else}
			<div class="grid grid-cols-2 gap-3 @2xl:grid-cols-5">
				{@render tile("Cost", fmtCost(sum.costUsd), `Today ${fmtCost(today.costUsd)}`)}
				{@render tile("Tokens", fmtTokens(sum.tokens), `${fmtTokens(sum.output)} output`)}
				{@render tile("Turns", String(sum.turns), `Today ${today.turns}`)}
				{@render tile("Agent time", fmtElapsed(sum.durationMs), `Today ${fmtElapsed(today.durationMs)}`)}
				{@render tile("Files changed", String(sum.files), "Counted per turn")}
			</div>

			<section class="bg-card border-border flex flex-col gap-3 rounded-lg border p-4" aria-label="Per day">
				<div class="flex items-center gap-2">
					<h2 class="font-medium">{metric === "cost" ? "Cost" : metric === "tokens" ? "Tokens" : "Agent time"} per day</h2>
					<button class="text-muted-foreground hover:text-foreground ms-auto text-[12px] underline-offset-2 hover:underline" onclick={() => (asTable = !asTable)}
						>{asTable ? "Show chart" : "Show as table"}</button
					>
				</div>
				{#if asTable}
					<div class="scrollbar-thin max-h-72 overflow-y-auto">
						<table class="w-full text-[12.5px]">
							<thead class="text-muted-foreground text-start">
								<tr><th class="py-1 text-start font-normal">Day</th><th class="text-end font-normal">Cost</th><th class="text-end font-normal">Tokens</th><th class="text-end font-normal">Turns</th><th class="text-end font-normal">Agent time</th></tr>
							</thead>
							<tbody >
								{#each [...daily].reverse() as d (d.day)}
									<tr class="border-border border-t">
										<td class="py-1">{dayLabel(d.day, { weekday: "short", month: "short", day: "numeric" })}</td>
										<td class="text-end">{fmtCost(d.totals.costUsd)}</td>
										<td class="text-end">{fmtTokens(d.totals.tokens)}</td>
										<td class="text-end">{d.totals.turns}</td>
										<td class="text-end">{fmtElapsed(d.totals.durationMs)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{:else}
					<div class="mt-3 flex gap-2">
						<!-- y axis -->
						<div class="text-muted-foreground relative h-44 w-12 shrink-0 text-end text-[11px]">
							{#each ticks as t (t)}
								<span class="absolute end-0 -translate-y-1/2" style="bottom: {(t / max) * 100}%">{fmtValue(t)}</span>
							{/each}
							<span class="absolute end-0 bottom-0 translate-y-1/2">0</span>
						</div>
						<div class="relative min-w-0 flex-1" bind:this={chartEl} onmouseleave={() => (hover = null)} role="presentation">
							<div class="relative h-44">
								{#each ticks as t (t)}
									<div class="bg-border absolute inset-x-0 h-px" style="bottom: {(t / max) * 100}%"></div>
								{/each}
								<div class="bg-border absolute inset-x-0 bottom-0 h-px"></div>
								<div class="absolute inset-0 flex items-end">
									{#each daily as d, i (d.day)}
										{@const v = valueOf(d.totals)}
										<button
											class="group/bar flex h-full min-w-0 flex-1 items-end justify-center px-px focus-visible:outline-none"
											aria-label="{dayLabel(d.day, { weekday: 'long', month: 'long', day: 'numeric' })}: {fmtValue(v)}, {d.totals.turns} turns"
											onpointerenter={(e) => showTip(i, e.currentTarget)}
											onfocus={(e) => showTip(i, e.currentTarget)}
											onblur={() => (hover = null)}
										>
											<span
												class="bg-primary block w-full max-w-6 rounded-t-[4px] transition-opacity group-hover/bar:opacity-80 group-focus-visible/bar:ring-ring group-focus-visible/bar:ring-2 {hover &&
												hover.i !== i
													? 'opacity-60'
													: ''}"
												style="height: {max ? Math.max(v > 0 ? 2 : 0, (v / max) * 100) : 0}%"
											></span>
										</button>
									{/each}
								</div>
							</div>
							<div class="text-muted-foreground mt-1.5 flex justify-between text-[11px]">
								<span>{dayLabel(daily[0].day)}</span>
								<span>{dayLabel(daily[Math.floor(daily.length / 2)].day)}</span>
								<span>Today</span>
							</div>
							{#if hover}
								{@const d = daily[hover.i]}
								<div
									class="bg-popover text-popover-foreground border-border pointer-events-none absolute -top-2 z-10 flex -translate-x-1/2 -translate-y-full flex-col rounded-md border px-2.5 py-1.5 text-[12px] whitespace-nowrap shadow-md"
									style="left: {hover.x}px"
								>
									<span class="text-[13px] font-semibold">{fmtValue(valueOf(d.totals))}</span>
									<span class="text-muted-foreground">{dayLabel(d.day, { weekday: "short", month: "short", day: "numeric" })} · {d.totals.turns} {d.totals.turns === 1 ? "turn" : "turns"}</span>
								</div>
							{/if}
						</div>
					</div>
				{/if}
				{#if metric === "cost" && codexOnly}
					<p class="text-muted-foreground text-[12px]">Codex reports tokens, not cost; switch to Tokens.</p>
				{/if}
			</section>

			<div class="grid gap-4 @4xl:grid-cols-2">
				<section class="bg-card border-border rounded-lg border" aria-label="By project">
					<h2 class="border-border border-b px-4 py-2.5 font-medium">By project</h2>
					<table class="w-full text-[12.5px]">
						<thead class="text-muted-foreground">
							<tr>
								<SortTh label="Project" class="px-4 py-1.5" active={projSort.col === "name"} desc={projSort.desc} onclick={() => (projSort = flip(projSort, "name"))} />
								<SortTh label="Turns" end active={projSort.col === "turns"} desc={projSort.desc} onclick={() => (projSort = flip(projSort, "turns"))} />
								<SortTh label="Tokens" end active={projSort.col === "tokens"} desc={projSort.desc} onclick={() => (projSort = flip(projSort, "tokens"))} />
								<SortTh label="Cost" end class="px-4" active={projSort.col === "cost"} desc={projSort.desc} onclick={() => (projSort = flip(projSort, "cost"))} />
							</tr>
						</thead>
						<tbody >
							{#each projects as g (g.key)}
								<tr class="border-border border-t">
									<td class="max-w-0 truncate px-4 py-1.5" title={userProjects.get(g.key)?.folders.join(", ") ?? g.key}>{projectName(g)}</td>
									<td class="text-end">{g.totals.turns}</td>
									<td class="text-end">{fmtTokens(g.totals.tokens)}</td>
									<td class="px-4 text-end">{fmtCost(g.totals.costUsd)}</td>
								</tr>
							{:else}
								<tr><td colspan="4" class="text-muted-foreground px-4 py-6 text-center">No turns in this period.</td></tr>
							{/each}
						</tbody>
					</table>
				</section>
				<section class="bg-card border-border rounded-lg border" aria-label="By conversation">
					<h2 class="border-border border-b px-4 py-2.5 font-medium">Top conversations</h2>
					<table class="w-full text-[12.5px]">
						<thead class="text-muted-foreground">
							<tr>
								<SortTh label="Conversation" class="px-4 py-1.5" active={convSort.col === "name"} desc={convSort.desc} onclick={() => (convSort = flip(convSort, "name"))} />
								<SortTh label="Turns" end active={convSort.col === "turns"} desc={convSort.desc} onclick={() => (convSort = flip(convSort, "turns"))} />
								<SortTh label="Time" end active={convSort.col === "time"} desc={convSort.desc} onclick={() => (convSort = flip(convSort, "time"))} />
								<SortTh label="Cost" end class="px-4" active={convSort.col === "cost"} desc={convSort.desc} onclick={() => (convSort = flip(convSort, "cost"))} />
							</tr>
						</thead>
						<tbody >
							{#each convs as g (g.key)}
								<tr class="border-border border-t">
									<td class="max-w-0 truncate px-4 py-1.5">
										{#if conversations.get(g.key)}
											<button class="truncate hover:underline" dir="auto" onclick={() => showConversation(g.key)}>{title(g.records)}</button>
										{:else}
											<span dir="auto">{title(g.records)}</span>
										{/if}
									</td>
									<td class="text-end">{g.totals.turns}</td>
									<td class="text-end">{fmtElapsed(g.totals.durationMs)}</td>
									<td class="px-4 text-end">{fmtCost(g.totals.costUsd)}</td>
								</tr>
							{:else}
								<tr><td colspan="4" class="text-muted-foreground px-4 py-6 text-center">No turns in this period.</td></tr>
							{/each}
						</tbody>
					</table>
				</section>
			</div>
		{/if}

		<section class="bg-card border-border rounded-lg border" aria-label="Account limits">
			<h2 class="border-border border-b px-4 py-2.5 font-medium">Account limits</h2>
			{#if accountList.length}
				<table class="w-full text-[12.5px]">
					<thead class="text-muted-foreground">
						<tr><th class="px-4 py-1.5 text-start font-normal">Account</th><th class="text-start font-normal">Plan</th><th class="text-start font-normal">5 hours left</th><th class="px-4 text-start font-normal">Week left</th></tr>
					</thead>
					<tbody>
						{#each accountList as a (accountKey(a))}
							<tr class="border-border border-t">
								<td class="px-4 py-1.5"><span class="me-1.5 inline-block size-1.5 rounded-full" style="background: var(--{a.provider})"></span>{a.name}</td>
								<td class="text-muted-foreground">{a.plan || "–"}</td>
								<td>{#if a.fiveHour}<LimitBar label="" win={a.fiveHour} wide />{:else}<span class="text-muted-foreground">–</span>{/if}</td>
								<td class="px-4">{#if a.week}<LimitBar label="" win={a.week} wide />{:else}<span class="text-muted-foreground">–</span>{/if}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="text-muted-foreground px-4 py-6 text-center">No accounts found.</p>
			{/if}
		</section>

		<footer class="text-muted-foreground flex flex-wrap items-center gap-2 text-[12px]">
			<span
				>Counts turns run in Glaude{first ? ` since ${new Date(first).toLocaleDateString()}` : ""}. Cost is the estimate Claude Code reports; Codex reports
				tokens only. Tokens leave out prompt-cache reads.</span
			>
			{#if usage.records.length}
				<Button variant="ghost" size="sm" class="ms-auto h-7" onclick={() => (confirmClear = true)}>Clear history</Button>
			{/if}
		</footer>
	</div>
</div>

<AlertDialog.Root bind:open={confirmClear}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Clear usage history?</AlertDialog.Title>
			<AlertDialog.Description>The {usage.records.length} recorded turns are removed from this device. Conversations are not affected.</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action
				onclick={() => {
					usage.clear();
					confirmClear = false;
				}}>Clear</AlertDialog.Action
			>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
