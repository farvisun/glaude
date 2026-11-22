<script lang="ts">
	import type { Item } from "$lib/types";
	import DiffView from "./DiffView.svelte";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import FileText from "@lucide/svelte/icons/file-text";
	import FilePen from "@lucide/svelte/icons/file-pen";
	import FilePlus from "@lucide/svelte/icons/file-plus";
	import Terminal from "@lucide/svelte/icons/terminal";
	import Search from "@lucide/svelte/icons/search";
	import Globe from "@lucide/svelte/icons/globe";
	import Bot from "@lucide/svelte/icons/bot";
	import ListTodo from "@lucide/svelte/icons/list-todo";
	import Wrench from "@lucide/svelte/icons/wrench";
	import Plug from "@lucide/svelte/icons/plug";
	import Check from "@lucide/svelte/icons/check";
	import XIcon from "@lucide/svelte/icons/x";
	import Ban from "@lucide/svelte/icons/ban";

	type ToolLike = Extract<Item, { kind: "tool" | "command" | "fileChange" }>;
	let { item }: { item: ToolLike } = $props();

	let open = $state(false);

	function iconFor(name: string) {
		const n = name.toLowerCase();
		if (n === "read" || n === "notebookread") return FileText;
		if (n === "edit" || n === "multiedit" || n === "notebookedit") return FilePen;
		if (n === "write") return FilePlus;
		if (n === "bash" || n === "bashoutput" || n === "killshell") return Terminal;
		if (n === "grep" || n === "glob" || n === "ls" || n === "websearch") return Search;
		if (n === "webfetch") return Globe;
		if (n === "task" || n === "agent") return Bot;
		if (n === "todowrite") return ListTodo;
		if (n.startsWith("mcp__")) return Plug;
		return Wrench;
	}

	const view = $derived.by(() => {
		if (item.kind === "command")
			return { name: "Shell", title: item.command, Icon: Terminal, output: item.output, diffs: [], input: null };
		if (item.kind === "fileChange")
			return {
				name: "Edit",
				title: item.diffs.map((d) => d.path).join(", "),
				Icon: FilePen,
				output: "",
				diffs: item.diffs,
				input: null,
			};
		const name = item.name.startsWith("mcp__") ? item.name.split("__").slice(1).join(" · ") : item.name;
		return { name, title: item.title, Icon: iconFor(item.name), output: item.output ?? "", diffs: item.diffs ?? [], input: item.input };
	});

	const hasBody = $derived(!!view.output || view.diffs.length > 0 || view.input != null);
	const exitCode = $derived(item.kind === "command" ? item.exitCode : undefined);
</script>

<div class="group/tool text-[13px]">
	<button
		class="hover:bg-accent/50 flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-1 text-start disabled:cursor-default"
		onclick={() => (open = !open)}
		disabled={!hasBody}
	>
		<view.Icon class="text-muted-foreground size-3.5 shrink-0" />
		<span class="shrink-0 font-medium">{view.name}</span>
		<span class="text-muted-foreground min-w-0 truncate font-mono text-[12px]" dir="ltr">{view.title}</span>
		<span class="ms-auto flex shrink-0 items-center gap-1.5">
			{#if item.status === "running" || item.status === "pending"}
				<span class="tool-running" aria-label="Running">⏺&#xFE0E;</span>
			{:else if item.status === "done"}
				{#if exitCode !== undefined && exitCode !== 0}
					<span class="text-destructive font-mono text-[11px]">exit {exitCode}</span>
				{:else}
					<Check class="text-ok size-3.5" />
				{/if}
			{:else if item.status === "declined" || item.status === "interrupted"}
				<Ban class="text-muted-foreground size-3.5" />
			{:else}
				<XIcon class="text-destructive size-3.5" />
			{/if}
			{#if hasBody}
				<ChevronRight class="text-muted-foreground size-3.5 transition-transform {open ? 'rotate-90' : ''}" />
			{/if}
		</span>
	</button>
	{#if open}
		<div class="flex flex-col gap-2 ps-7 pe-1 pt-1 pb-2">
			{#if view.diffs.length}
				<DiffView diffs={view.diffs} />
			{:else if view.input != null}
				<pre class="bg-code border-border selectable scrollbar-thin max-h-60 overflow-auto rounded-md border px-2.5 py-2 font-mono text-[12px] whitespace-pre-wrap" dir="ltr">{JSON.stringify(view.input, null, 2)}</pre>
			{/if}
			{#if view.output}
				<pre class="bg-code border-border selectable scrollbar-thin max-h-72 overflow-auto rounded-md border px-2.5 py-2 font-mono text-[12px] whitespace-pre-wrap" dir="auto">{view.output}</pre>
			{/if}
		</div>
	{/if}
</div>

<style>
	.tool-running {
		color: var(--muted-foreground);
		font-size: 11px;
		line-height: 1;
		font-variant-emoji: text;
		animation: tool-blink 1s steps(1, end) infinite;
	}
	@keyframes tool-blink {
		50% {
			opacity: 0.15;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tool-running {
			animation: none;
		}
	}
</style>
