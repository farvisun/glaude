<script lang="ts">
	import type { FileDiff, ReviewNote as Note } from "$lib/types";
	import FileDiffIcon from "@lucide/svelte/icons/file-diff";
	import Rows2 from "@lucide/svelte/icons/rows-2";
	import Columns2 from "@lucide/svelte/icons/columns-2";

	let {
		diffs,
		maxHeight = "24rem",
		onLine,
		notes = [],
		onRemoveNote,
	}: {
		diffs: FileDiff[];
		maxHeight?: string;
		/** Clicking a line number: add a review note there. */
		onLine?: (path: string, side: "a" | "b", n: number, code: string) => void;
		notes?: Note[];
		onRemoveNote?: (id: string) => void;
	} = $props();

	const lineKey = (l: { t: string; a?: number; b?: number }): ["a" | "b", number] | null =>
		l.t === "del" && l.a != null ? ["a", l.a] : l.b != null ? ["b", l.b] : null;
	function notesAt(path: string, l: { t: string; a?: number; b?: number }) {
		const k = lineKey(l);
		return k ? notes.filter((x) => x.path === path && x.side === k[0] && x.n === k[1]) : [];
	}

	const SPLIT_KEY = "glaude.diff.split";
	let split = $state(localStorage.getItem(SPLIT_KEY) === "1");

	function toggle() {
		split = !split;
		localStorage.setItem(SPLIT_KEY, split ? "1" : "0");
	}

	type Line = { t: "ctx" | "add" | "del" | "hunk"; text: string; a?: number; b?: number };

	function parse(diff: string): Line[] {
		const out: Line[] = [];
		let a = 0;
		let b = 0;
		for (const raw of diff.split("\n")) {
			if (raw.startsWith("+++") || raw.startsWith("---") || raw.startsWith("diff ") || raw.startsWith("index ")) continue;
			if (raw.startsWith("@@")) {
				const m = /@@ -(\d+)(?:,\d+)? \+(\d+)/.exec(raw);
				if (m) {
					a = +m[1];
					b = +m[2];
				}
				out.push({ t: "hunk", text: raw });
			} else if (raw.startsWith("+")) out.push({ t: "add", text: raw.slice(1), b: b++ });
			else if (raw.startsWith("-")) out.push({ t: "del", text: raw.slice(1), a: a++ });
			else if (raw.length || out.length) out.push({ t: "ctx", text: raw.slice(1), a: a++, b: b++ });
		}
		while (out.length && out[out.length - 1].t === "ctx" && out[out.length - 1].text === "") out.pop();
		return out;
	}

	type Row = { l?: Line; r?: Line; hunk?: string };

	function pair(lines: Line[]): Row[] {
		const rows: Row[] = [];
		let dels: Line[] = [];
		let adds: Line[] = [];
		const flush = () => {
			const n = Math.max(dels.length, adds.length);
			for (let i = 0; i < n; i++) rows.push({ l: dels[i], r: adds[i] });
			dels = [];
			adds = [];
		};
		for (const l of lines) {
			if (l.t === "del") dels.push(l);
			else if (l.t === "add") adds.push(l);
			else {
				flush();
				if (l.t === "hunk") rows.push({ hunk: l.text });
				else rows.push({ l, r: l });
			}
		}
		flush();
		return rows;
	}

	const cls = { add: "diff-add", del: "diff-del", ctx: "", hunk: "diff-hunk" } as const;
</script>

{#each diffs as d (d.path)}
	{@const lines = parse(d.diff)}
	<div class="border-border overflow-hidden rounded-md border">
		<div class="bg-muted/60 border-border flex items-center gap-2 border-b px-2.5 py-1.5 text-xs">
			<FileDiffIcon class="text-muted-foreground size-3.5" />
			<span class="truncate font-mono" dir="ltr">{d.path}</span>
			{#if d.kind === "add"}<span class="text-ok">new</span>{:else if d.kind === "delete"}<span class="text-destructive"
					>deleted</span
				>{/if}
			<span class="text-ok ms-auto font-mono">+{d.added}</span>
			<span class="text-destructive font-mono">−{d.removed}</span>
			<button class="text-muted-foreground hover:text-foreground" aria-label="Toggle side by side" onclick={toggle}>
				{#if split && !onLine}<Rows2 class="size-3.5" />{:else}<Columns2 class="size-3.5" />{/if}
			</button>
		</div>
		<div class="scrollbar-thin selectable overflow-auto font-mono text-[12px] leading-[1.55]" style:max-height={maxHeight} dir="ltr">
			{#if split}
				<table class="w-full border-collapse">
					<tbody>
						{#each pair(lines) as row, i (i)}
							{#if row.hunk}
								<tr class="diff-hunk"><td colspan="4" class="px-2">{row.hunk}</td></tr>
							{:else}
								<tr>
									<td class="ln">{row.l?.a ?? ""}</td>
									<td class="w-1/2 px-2 whitespace-pre-wrap {row.l ? cls[row.l.t] : ''}">{row.l?.text ?? ""}</td>
									<td class="ln border-border border-s">{row.r?.b ?? ""}</td>
									<td class="w-1/2 px-2 whitespace-pre-wrap {row.r ? cls[row.r.t] : ''}">{row.r?.text ?? ""}</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			{:else}
				<table class="w-full border-collapse">
					<tbody>
						{#each lines as l, i (i)}
							<tr class="{cls[l.t]} group/ln">
								{#if l.t === "hunk"}
									<td colspan="3" class="px-2">{l.text}</td>
								{:else}
									{#if onLine}
										<td class="ln p-0" colspan="2">
											<button
												class="hover:bg-primary/15 hover:text-primary relative flex w-full justify-end gap-2 px-2"
												aria-label="Comment on line {l.b ?? l.a}"
												title="Comment on this line"
												onclick={() => {
													const k = lineKey(l);
													if (k) onLine(d.path, k[0], k[1], l.text);
												}}
											>
												<span class="text-primary absolute start-1 hidden group-hover/ln:inline">+</span>
												<span class="w-[3ch]">{l.a ?? ""}</span><span class="w-[3ch]">{l.b ?? ""}</span>
											</button>
										</td>
									{:else}
										<td class="ln">{l.a ?? ""}</td>
										<td class="ln">{l.b ?? ""}</td>
									{/if}
									<!-- The code itself (also when line numbers take review notes). -->
									<td class="px-2 whitespace-pre-wrap"
										><span class="sign">{l.t === "add" ? "+" : l.t === "del" ? "−" : " "}</span>{l.text}</td
									>
								{/if}
							</tr>
							{#each notesAt(d.path, l) as note (note.id)}
								<tr>
									<td colspan="3" class="bg-card border-border border-y px-3 py-1.5 font-sans text-[13px]">
										<div class="flex items-start gap-2">
											<span class="text-primary shrink-0 text-xs font-medium">Note</span>
											<span class="min-w-0 flex-1 whitespace-pre-wrap" dir="auto">{note.text}</span>
											{#if onRemoveNote}
												<button class="text-muted-foreground hover:text-foreground shrink-0 text-xs" onclick={() => onRemoveNote(note.id)}>Remove</button>
											{/if}
										</div>
									</td>
								</tr>
							{/each}
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	</div>
{/each}

<style>
	.ln {
		width: 1%;
		padding-inline: 0.5rem;
		text-align: end;
		color: var(--muted-foreground);
		opacity: 0.6;
		user-select: none;
		white-space: nowrap;
		vertical-align: top;
	}
	.sign {
		display: inline-block;
		width: 1.1em;
		opacity: 0.6;
		user-select: none;
	}
	:global(.diff-add) {
		background: color-mix(in oklab, var(--ok) 16%, transparent);
	}
	:global(.diff-del) {
		background: color-mix(in oklab, var(--destructive) 16%, transparent);
	}
	:global(.diff-hunk) {
		color: var(--muted-foreground);
		background: var(--muted);
	}
</style>
