<script lang="ts">
	// Read-only file viewer, opened from a path in a reply.
	import type { Tab } from "$lib/state/layout.svelte";
	import { call } from "$lib/ipc";
	import { pathActions, fileManager } from "$lib/state/open";
	import { prefs } from "$lib/state/prefs.svelte";
	import { Button } from "$lib/components/ui/button";
	import FileCode from "@lucide/svelte/icons/file-code";
	import FolderSearch from "@lucide/svelte/icons/folder-search";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";

	interface FileView {
		path: string;
		text: string;
		html?: string | null;
		lines: number;
		size: number;
		truncated: boolean;
		binary: boolean;
	}

	let { tab }: { tab: Tab; active: boolean } = $props();
	let view = $state<FileView | null>(null);
	let error = $state<string | null>(null);
	let scroller = $state<HTMLDivElement>();
	const LINE = 20; // px, matches leading-5

	async function load() {
		error = null;
		try {
			view = await call<FileView>("read_text_file", { path: tab.ref });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}
	$effect(() => {
		void tab.ref;
		void load();
	});
	// Scroll to the linked line.
	$effect(() => {
		const line = tab.line;
		if (!view || !scroller || !line) return;
		requestAnimationFrame(() => {
			if (scroller) scroller.scrollTop = Math.max(0, (line - 6) * LINE);
		});
	});
	const hit = $derived({ abs: tab.ref ?? "", dir: false, line: tab.line });
	const kb = (n: number) => (n > 1 << 20 ? `${(n / (1 << 20)).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);
</script>

<div class="flex h-full min-h-0 flex-col">
	<div class="border-border flex items-center gap-2 border-b px-3 py-1.5 text-xs">
		<span class="text-muted-foreground min-w-0 flex-1 truncate font-mono" dir="ltr" title={tab.ref}>{tab.ref}</span>
		{#if view}<span class="text-muted-foreground shrink-0">{view.lines} lines · {kb(view.size)}</span>{/if}
		<Button variant="ghost" size="icon-sm" aria-label="Reload" onclick={load}><RefreshCw /></Button>
		<Button variant="ghost" size="icon-sm" aria-label="Show in {fileManager}" title="Show in {fileManager}" onclick={() => pathActions.reveal(hit)}
			><FolderSearch /></Button
		>
		{#if prefs.editor !== "default"}
			<Button variant="ghost" size="sm" class="h-7" onclick={() => pathActions.editor(hit)}><FileCode /> {pathActions.editorLabel()}</Button>
		{/if}
	</div>
	{#if error}
		<div class="text-muted-foreground grid flex-1 place-items-center text-[13px]">{error}</div>
	{:else if !view}
		<div class="flex flex-col gap-2 p-4">
			{#each [0, 1, 2, 3] as i (i)}<div class="skeleton-shimmer h-4 rounded" style="width: {90 - i * 15}%"></div>{/each}
		</div>
	{:else if view.binary}
		<div class="text-muted-foreground grid flex-1 place-items-center text-[13px]">This is a binary file. Open it with the default app.</div>
	{:else}
		<div class="scrollbar-thin relative min-h-0 flex-1 overflow-auto" bind:this={scroller}>
			{#if tab.line}
				<div class="bg-primary/10 pointer-events-none absolute inset-x-0" style="top: {(tab.line - 1) * LINE + 8}px; height: {LINE}px"></div>
			{/if}
			<div class="flex min-w-max py-2 font-mono text-[12.5px] leading-5" dir="ltr">
				<div class="text-muted-foreground/60 sticky left-0 shrink-0 bg-inherit px-3 text-right select-none" aria-hidden="true">
					{#each { length: view.lines || 1 } as _, i (i)}<div>{i + 1}</div>{/each}
				</div>
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				<pre class="code-view m-0 pe-6">{#if view.html}{@html view.html}{:else}{view.text}{/if}</pre>
			</div>
			{#if view.truncated}<div class="text-muted-foreground px-4 py-2 text-xs">Showing the first 2 MB.</div>{/if}
		</div>
	{/if}
</div>
