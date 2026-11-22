<script lang="ts">
	// The latest turn as a timeline: each tool call and subagent with a bar
	// from when it started to when it ended, so you can see where time went.
	import type { Conversation } from "$lib/state/conversations.svelte";
	import type { Item } from "$lib/types";
	import { toolStep, fmtElapsed } from "$lib/state/activity";
	import { clock } from "$lib/state/clock.svelte";

	let { conv }: { conv: Conversation } = $props();

	type Row = { id: string; label: string; start: number; end: number | null; error: boolean; nested: boolean };

	const MAX_ROWS = 24;
	const rows = $derived.by((): Row[] => {
		let from = 0;
		for (let i = conv.items.length - 1; i >= 0; i--)
			if (conv.items[i].kind === "user") {
				from = i;
				break;
			}
		const out: Row[] = [];
		for (const it of conv.items.slice(from)) {
			if (it.kind !== "tool" && it.kind !== "command" && it.kind !== "fileChange") continue;
			const t = conv.timeOf(it.id);
			if (!t) continue;
			const task = it.kind === "tool" ? conv.taskFor(it.id) : undefined;
			const end = task ? (task.endedAt ?? (task.status === "running" || task.status === "pending" ? null : t.end ?? null)) : (t.end ?? null);
			out.push({
				id: it.id,
				label: toolStep(it as Item),
				start: t.start,
				end: end ?? (it.status === "running" || it.status === "pending" ? null : t.start),
				error: it.status === "error" || (it.kind === "command" && it.exitCode !== undefined && it.exitCode !== 0),
				nested: it.kind === "tool" && !!it.parent,
			});
		}
		return out;
	});
	const live = $derived(rows.some((r) => r.end == null));
	$effect(() => {
		if (live) return clock.subscribe();
	});
	const t0 = $derived(rows.length ? Math.min(...rows.map((r) => r.start)) : 0);
	const t1 = $derived(rows.length ? Math.max(...rows.map((r) => r.end ?? clock.now)) : 0);
	const span = $derived(Math.max(1000, t1 - t0));
	const shown = $derived(rows.slice(-MAX_ROWS));
	function reveal(id: string) {
		document.getElementById(`tool-${id}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
	}
</script>

{#if rows.length}
	<ul class="flex flex-col gap-1.5" aria-label="Timeline of the latest turn">
		{#if rows.length > MAX_ROWS}<li class="text-muted-foreground text-[11px]">{rows.length - MAX_ROWS} earlier calls</li>{/if}
		{#each shown as r (r.id)}
			{@const end = r.end ?? clock.now}
			<li>
				<button class="hover:bg-accent/40 flex w-full flex-col gap-0.5 rounded px-1 py-0.5 text-start" onclick={() => reveal(r.id)}>
					<span class="flex min-w-0 items-baseline gap-2 text-[12px]">
						<span class="min-w-0 flex-1 truncate {r.nested ? 'text-muted-foreground ps-2' : ''} {r.error ? 'text-destructive' : ''}" dir="auto">{r.label}</span>
						<span class="text-muted-foreground shrink-0 text-[11px] tabular-nums">{fmtElapsed(end - r.start)}</span>
					</span>
					<span class="bg-muted relative block h-1 w-full overflow-hidden rounded-full">
						<span
							class="absolute inset-y-0 rounded-full {r.error ? 'bg-destructive' : r.end == null ? 'bg-primary animate-pulse' : 'bg-primary/70'}"
							style="inset-inline-start: {((r.start - t0) / span) * 100}%; width: max(3px, {((end - r.start) / span) * 100}%)"
						></span>
					</span>
				</button>
			</li>
		{/each}
		<li class="text-muted-foreground flex justify-between text-[11px] tabular-nums"><span>0s</span><span>{fmtElapsed(span)}</span></li>
	</ul>
{:else}
	<p class="text-muted-foreground text-xs">Tool calls of the latest turn appear here with how long each took.</p>
{/if}
