<script lang="ts">
	import * as Command from "$lib/components/ui/command";
	import Search from "@lucide/svelte/icons/search";
	import { ui } from "$lib/state/ui.svelte";
	import { history } from "$lib/state/history.svelte";
	import { openSession, showError } from "$lib/state/sessions";
	import { call } from "$lib/ipc";
	import { detectDir } from "$lib/bidi";
	import { basename, fmtAgo } from "$lib/format";
	import type { Provider } from "$lib/types";

	interface Hit {
		provider: Provider;
		sessionId: string;
		cwd: string;
		role: string;
		ts: number;
		snippet: string;
		matches: number;
	}
	interface Status {
		ready: boolean;
		indexing: boolean;
		progress: number;
		messages: number;
		error: string | null;
	}

	let query = $state("");
	let hits = $state.raw<Hit[]>([]);
	let searching = $state(false);
	let status = $state<Status | null>(null);
	let seq = 0;

	const titles = $derived(new Map(history.sessions.map((s) => [`${s.provider}:${s.id}`, s.title])));
	const recent = $derived(history.sessions.slice(0, 12));

	// Index status while the palette is open (the index updates in the background).
	$effect(() => {
		if (!ui.searchOpen) return;
		call("search_refresh").catch(() => {});
		let stop = false;
		const poll = async () => {
			try {
				status = await call<Status>("search_status");
			} catch {
				status = null;
			}
			if (!stop && (status?.indexing || !status?.ready)) timer = setTimeout(poll, 500);
		};
		let timer = setTimeout(poll, 0);
		return () => {
			stop = true;
			clearTimeout(timer);
		};
	});

	$effect(() => {
		const q = query.trim();
		void status?.messages; // search again when the index grows
		if (!ui.searchOpen || !q) {
			hits = [];
			searching = false;
			return;
		}
		searching = true;
		const mine = ++seq;
		const t = setTimeout(async () => {
			try {
				const res = await call<Hit[]>("search_query", { query: q, limit: 40 });
				if (mine === seq) hits = res;
			} catch (e) {
				if (mine === seq) showError("Search failed", e);
			} finally {
				if (mine === seq) searching = false;
			}
		}, 140);
		return () => clearTimeout(t);
	});

	function pick(provider: Provider, id: string, cwd: string) {
		ui.searchOpen = false;
		openSession({ provider, id, cwd, title: titles.get(`${provider}:${id}`) });
	}

	const plain = (html: string) => html.replace(/<[^>]+>/g, "");
</script>

<Command.Dialog
	bind:open={ui.searchOpen}
	shouldFilter={false}
	title="Search history"
	description="Search prompts and replies in past Claude and Codex sessions"
	class="sm:max-w-2xl"
>
	<Command.Input bind:value={query} placeholder="Search past conversations…" dir="auto" />
	<Command.List class="max-h-[60vh]">
		{#if !query.trim()}
			{#if recent.length}
				<Command.Group heading="Recent">
					{#each recent as s (s.provider + s.id)}
						<Command.Item value={`${s.provider}:${s.id}`} onSelect={() => pick(s.provider, s.id, s.cwd)}>
							<span class="size-2 shrink-0 rounded-full {s.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
							<span class="min-w-0 flex-1 truncate text-left" dir="auto">{s.title}</span>
							<span class="text-muted-foreground shrink-0 text-[11px]">{basename(s.cwd)} · {fmtAgo(s.updated)}</span>
						</Command.Item>
					{/each}
				</Command.Group>
			{:else}
				<Command.Empty>Type to search your past sessions.</Command.Empty>
			{/if}
		{:else if hits.length === 0}
			<div class="text-muted-foreground px-4 py-8 text-center text-[13px]">
				{#if searching}Searching…{:else if status?.indexing && !status.ready}Building the search index…{:else}No matches for “{query.trim()}”.{/if}
			</div>
		{:else}
			<Command.Group heading="Matches">
				{#each hits as h (h.provider + h.sessionId)}
					<Command.Item value={`${h.provider}:${h.sessionId}`} onSelect={() => pick(h.provider, h.sessionId, h.cwd)} class="items-start">
						<span class="mt-1.5 size-2 shrink-0 rounded-full {h.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<span class="min-w-0 flex-1 truncate text-left font-medium" dir="auto"
									>{titles.get(`${h.provider}:${h.sessionId}`) ?? basename(h.cwd) ?? "Session"}</span
								>
								<span class="text-muted-foreground shrink-0 text-[11px]"
									>{h.role === "user" ? "you" : "reply"} · {basename(h.cwd)} · {fmtAgo(h.ts)}{h.matches > 1 ? ` · ${h.matches} matches` : ""}</span
								>
							</div>
							<p class="search-snippet text-muted-foreground mt-0.5 line-clamp-2 text-[12px] leading-relaxed" dir={detectDir(plain(h.snippet))}>
								{@html h.snippet}
							</p>
						</div>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
	<div class="text-muted-foreground flex items-center gap-2 border-t px-3 py-1.5 text-[11px]">
		<Search class="size-3" />
		{#if status?.indexing}
			Indexing history… {Math.round((status.progress ?? 0) * 100)}%
		{:else if status?.error}
			Index error: {status.error}
		{:else if status}
			{status.messages.toLocaleString()} messages indexed
		{/if}
		<span class="ms-auto">↵ open · esc close</span>
	</div>
</Command.Dialog>

<style>
	.search-snippet :global(mark) {
		background: color-mix(in oklab, var(--primary) 28%, transparent);
		color: var(--foreground);
		border-radius: 3px;
		padding-inline: 1px;
	}
</style>
