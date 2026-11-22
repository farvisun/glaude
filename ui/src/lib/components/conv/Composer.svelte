<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import ArrowUp from "@lucide/svelte/icons/arrow-up";
	import Square from "@lucide/svelte/icons/square";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import X from "@lucide/svelte/icons/x";
	import { detectDir } from "$lib/bidi";
	import type { AgentInfo, Attachment } from "$lib/types";
	import Bot from "@lucide/svelte/icons/bot";
	import FileIcon from "@lucide/svelte/icons/file";
	import Folder from "@lucide/svelte/icons/folder";
	import Archive from "@lucide/svelte/icons/archive";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import { stash } from "$lib/state/stash.svelte";
	import { fmtAgo } from "$lib/format";
	import { isMac } from "$lib/state/prefs.svelte";

	let {
		running,
		disabled = false,
		slashCommands = [],
		placeholder = "Message",
		onSend,
		onStop,
		onAttach,
		onPasteImage,
		agents = [],
		onSuggestFiles,
		onCycleMode,
		modeHint,
		onSteer,
		history = [],
	}: {
		running: boolean;
		disabled?: boolean;
		slashCommands?: string[];
		placeholder?: string;
		onSend: (text: string, attachments: Attachment[]) => void;
		onStop: () => void;
		onAttach?: () => Promise<Attachment[]>;
		onPasteImage?: (blob: Blob, name: string) => Promise<Attachment | null>;
		/** For @agent-<name> mentions. */
		agents?: AgentInfo[];
		/** For @file mentions; paths relative to the project. */
		onSuggestFiles?: (query: string) => Promise<string[]>;
		/** Shift+Tab: next permission mode. */
		onCycleMode?: () => void;
		/** Shown under the input, e.g. "plan mode on". */
		modeHint?: string;
		/** Cmd/Ctrl+Enter while a turn runs: send into it now. */
		onSteer?: (text: string, attachments: Attachment[]) => void;
		/** Earlier prompts, oldest first, for ArrowUp. */
		history?: string[];
	} = $props();

	// ArrowUp/ArrowDown walk earlier prompts; the draft is kept aside.
	let recall = $state<number | null>(null);
	let kept = "";
	function walk(dir: -1 | 1): boolean {
		if (!history.length) return false;
		if (recall === null) {
			if (dir === 1) return false;
			kept = text;
			recall = history.length - 1;
		} else {
			const next = recall + dir;
			if (next < 0) return true;
			if (next >= history.length) {
				recall = null;
				text = kept;
				return true;
			}
			recall = next;
		}
		text = history[recall];
		queueMicrotask(() => {
			if (!el) return;
			el.setSelectionRange(text.length, text.length);
			caret = text.length;
		});
		return true;
	}

	/** Quote a passage from a reply into the message (`> ` lines). */
	export function insertQuote(passage: string) {
		const quoted = passage
			.trim()
			.split("\n")
			.map((l) => `> ${l}`)
			.join("\n");
		text = (text.trim() ? text.replace(/\s*$/, "\n\n") : "") + quoted + "\n\n";
		queueMicrotask(() => {
			el?.focus();
			if (el) caret = el.selectionStart = el.selectionEnd = text.length;
		});
	}

	function stashDraft() {
		if (!text.trim() && !attachments.length) return;
		stash.add(text, attachments);
		text = "";
		attachments = [];
	}

	function restore(id: string) {
		const d = stash.take(id);
		if (!d) return;
		if (text.trim()) stash.add(text, attachments);
		text = d.text;
		attachments = d.attachments;
		queueMicrotask(() => el?.focus());
	}

	let text = $state("");
	let attachments = $state<Attachment[]>([]);
	let el = $state<HTMLTextAreaElement>();
	let menuIndex = $state(0);
	let dragging = $state(false);

	const dir = $derived(detectDir(text));

	const slashQuery = $derived.by(() => {
		const m = /^\/([\w:-]*)$/.exec(text);
		return m ? m[1].toLowerCase() : null;
	});
	const slashMatches = $derived(
		slashQuery === null ? [] : slashCommands.filter((c) => c.toLowerCase().startsWith(slashQuery)).slice(0, 8),
	);

	// @ mentions: agents and files, for the word being typed at the caret.
	let caret = $state(0);
	const mention = $derived.by(() => {
		const m = /(^|\s)@([^\s@]*)$/.exec(text.slice(0, caret));
		return m ? { query: m[2], start: caret - m[2].length - 1 } : null;
	});
	let files = $state<string[]>([]);
	$effect(() => {
		const q = mention?.query;
		if (q === undefined || !onSuggestFiles || q.startsWith("agent-")) {
			files = [];
			return;
		}
		let stale = false;
		const t = setTimeout(() => {
			onSuggestFiles(q)
				.then((r) => !stale && (files = r))
				.catch(() => !stale && (files = []));
		}, 120);
		return () => {
			stale = true;
			clearTimeout(t);
		};
	});
	/** Escape closes the menu for the mention that starts here. */
	let dismissedAt = $state(-1);
	type Suggestion = { kind: "agent" | "file" | "folder"; insert: string; label: string; detail?: string };
	const mentionMatches = $derived.by((): Suggestion[] => {
		if (!mention || mention.start === dismissedAt) return [];
		const q = mention.query.toLowerCase().replace(/^agent-?/, "");
		const a = agents
			.filter((x) => !q || x.name.toLowerCase().includes(q))
			.slice(0, mention.query.startsWith("agent") ? 8 : 4)
			.map((x): Suggestion => ({ kind: "agent", insert: `agent-${x.name}`, label: `agent-${x.name}`, detail: x.description }));
		const f = files.slice(0, 8).map((p): Suggestion => ({ kind: p.endsWith("/") ? "folder" : "file", insert: p, label: p }));
		return [...a, ...f].slice(0, 10);
	});

	function pickMention(s: Suggestion) {
		if (!mention) return;
		// Folders keep the menu open to go deeper.
		const tail = s.kind === "folder" ? "" : " ";
		const before = text.slice(0, mention.start) + "@" + s.insert + tail;
		text = before + text.slice(caret);
		menuIndex = 0;
		queueMicrotask(() => {
			if (!el) return;
			el.focus();
			el.setSelectionRange(before.length, before.length);
			caret = before.length;
		});
	}

	/** Put text into the input (after a rewind). */
	export function setText(t: string) {
		text = t;
		queueMicrotask(() => {
			el?.focus();
			if (el) caret = el.selectionStart = el.selectionEnd = t.length;
		});
	}

	function autosize() {
		if (!el) return;
		el.style.height = "auto";
		el.style.height = Math.min(el.scrollHeight, 280) + "px";
	}

	$effect(() => {
		void text;
		autosize();
	});

	export function focus() {
		el?.focus();
	}

	function send(steer = false) {
		const t = text.trim();
		if ((!t && attachments.length === 0) || disabled) return;
		if (steer && onSteer) onSteer(t, attachments);
		else onSend(t, attachments);
		text = "";
		attachments = [];
		recall = null;
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === "Tab" && e.shiftKey && onCycleMode) {
			e.preventDefault();
			onCycleMode();
			return;
		}
		if (!slashMatches.length && mentionMatches.length) {
			if (e.key === "ArrowDown" || e.key === "ArrowUp") {
				e.preventDefault();
				const d = e.key === "ArrowDown" ? 1 : -1;
				menuIndex = (menuIndex + d + mentionMatches.length) % mentionMatches.length;
				return;
			}
			if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey)) {
				e.preventDefault();
				pickMention(mentionMatches[Math.min(menuIndex, mentionMatches.length - 1)]);
				return;
			}
			if (e.key === "Escape") {
				e.preventDefault();
				dismissedAt = mention?.start ?? -1;
				return;
			}
		}
		if (slashMatches.length) {
			if (e.key === "ArrowDown" || e.key === "ArrowUp") {
				e.preventDefault();
				const d = e.key === "ArrowDown" ? 1 : -1;
				menuIndex = (menuIndex + d + slashMatches.length) % slashMatches.length;
				return;
			}
			if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && slashMatches[menuIndex] !== slashQuery)) {
				e.preventDefault();
				text = "/" + slashMatches[menuIndex] + " ";
				menuIndex = 0;
				return;
			}
		}
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
			e.preventDefault();
			stashDraft();
			return;
		}
		if (e.key === "ArrowUp" && !e.shiftKey && !e.altKey && el && !text.slice(0, el.selectionStart).includes("\n")) {
			if (walk(-1)) e.preventDefault();
			return;
		}
		if (e.key === "ArrowDown" && recall !== null && el && !text.slice(el.selectionEnd).includes("\n")) {
			if (walk(1)) e.preventDefault();
			return;
		}
		if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
			e.preventDefault();
			send(running && (e.metaKey || e.ctrlKey));
		}
		if (e.key === "Escape" && running) onStop();
	}

	async function onPaste(e: ClipboardEvent) {
		if (!onPasteImage || !e.clipboardData) return;
		for (const f of Array.from(e.clipboardData.files)) {
			if (f.type.startsWith("image/")) {
				e.preventDefault();
				const a = await onPasteImage(f, f.name || "pasted.png");
				if (a) attachments = [...attachments, a];
			}
		}
	}

	async function onDrop(e: DragEvent) {
		dragging = false;
		if (!onPasteImage || !e.dataTransfer?.files.length) return;
		e.preventDefault();
		for (const f of Array.from(e.dataTransfer.files)) {
			const a = await onPasteImage(f, f.name);
			if (a) attachments = [...attachments, a];
		}
	}

	async function attach() {
		if (!onAttach) return;
		const picked = await onAttach();
		attachments = [...attachments, ...picked];
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="border-input bg-card focus-within:border-ring/60 relative rounded-xl border shadow-xs transition-colors {dragging
		? 'border-primary'
		: ''}"
	ondragover={(e) => {
		if (e.dataTransfer?.types.includes("Files")) {
			e.preventDefault();
			dragging = true;
		}
	}}
	ondragleave={() => (dragging = false)}
	ondrop={onDrop}
>
	{#if slashMatches.length}
		<div class="bg-popover border-border absolute inset-x-2 bottom-full mb-1.5 overflow-hidden rounded-lg border py-1 shadow-md">
			{#each slashMatches as c, i (c)}
				<button
					class="flex w-full px-3 py-1 text-start font-mono text-[13px] {i === menuIndex ? 'bg-accent' : ''}"
					onmousedown={(e) => {
						e.preventDefault();
						text = "/" + c + " ";
						el?.focus();
					}}>/{c}</button
				>
			{/each}
		</div>
	{/if}

	{#if !slashMatches.length && mentionMatches.length}
		<div
			class="bg-popover border-border scrollbar-thin absolute inset-x-2 bottom-full mb-1.5 max-h-72 overflow-y-auto rounded-lg border py-1 shadow-md"
			role="listbox"
		>
			{#each mentionMatches as s, i (s.kind + s.insert)}
				<button
					class="flex w-full min-w-0 items-center gap-2 px-3 py-1 text-start text-[13px] {i === menuIndex ? 'bg-accent' : ''}"
					role="option"
					aria-selected={i === menuIndex}
					onmousedown={(e) => {
						e.preventDefault();
						pickMention(s);
					}}
				>
					{#if s.kind === "agent"}<Bot class="text-muted-foreground size-3.5 shrink-0" />{:else if s.kind === "folder"}<Folder
							class="text-muted-foreground size-3.5 shrink-0"
						/>{:else}<FileIcon class="text-muted-foreground size-3.5 shrink-0" />{/if}
					<span class="shrink-0 font-mono" dir="ltr">@{s.label}</span>
					{#if s.detail}<span class="text-muted-foreground min-w-0 truncate text-xs" dir="auto">{s.detail}</span>{/if}
				</button>
			{/each}
		</div>
	{/if}

	{#if attachments.length}
		<div class="flex flex-wrap gap-1.5 px-3 pt-2.5">
			{#each attachments as a, i (a.path)}
				<span class="bg-muted inline-flex items-center gap-1 rounded-md py-0.5 ps-2 pe-1 text-xs">
					<Paperclip class="size-3" />{a.name}
					<button
						class="hover:bg-accent rounded p-0.5"
						aria-label="Remove attachment"
						onclick={() => (attachments = attachments.filter((_, j) => j !== i))}><X class="size-3" /></button
					>
				</span>
			{/each}
		</div>
	{/if}

	<textarea
		bind:this={el}
		bind:value={text}
		{dir}
		rows="1"
		placeholder={running ? `Queue a message · ${isMac ? "⌘" : "Ctrl+"}↵ to send it now` : placeholder}
		class="placeholder:text-muted-foreground block max-h-[280px] w-full resize-none bg-transparent px-3.5 pt-3 pb-1 text-[15px] leading-7 outline-none"
		oninput={() => {
			caret = el?.selectionStart ?? text.length;
			recall = null;
		}}
		onkeyup={() => (caret = el?.selectionStart ?? text.length)}
		onclick={() => (caret = el?.selectionStart ?? text.length)}
		onkeydown={onKey}
		onpaste={onPaste}
	></textarea>

	<div class="flex items-center gap-1 px-2 pb-2">
		{#if onAttach}
			<Button variant="ghost" size="icon-sm" aria-label="Attach files" onclick={attach}><Paperclip /></Button>
		{/if}
		{#if stash.items.length}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" aria-label="Stashed drafts" title="Stashed drafts ({isMac ? '⌘' : 'Ctrl+'}S stashes)"
							><Archive /></Button
						>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start" class="w-72">
					<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">Stashed drafts</DropdownMenu.Label>
					{#each stash.items as d (d.id)}
						<DropdownMenu.Item class="flex items-start gap-2" onclick={() => restore(d.id)}>
							<span class="line-clamp-2 min-w-0 flex-1 text-[13px]" dir="auto">{d.text || "(attachments)"}</span>
							<span class="text-muted-foreground shrink-0 text-[11px]">{fmtAgo(d.at)}</span>
							<button
								class="hover:text-foreground text-muted-foreground shrink-0"
								aria-label="Delete draft"
								onclick={(e) => {
									e.stopPropagation();
									stash.take(d.id);
								}}><X class="size-3.5" /></button
							>
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
		{#if modeHint}
			<button
				class="text-muted-foreground hover:text-foreground truncate text-xs"
				title="Shift+Tab to change the permission mode"
				onclick={() => onCycleMode?.()}>{modeHint}</button
			>
		{/if}
		<div class="ms-auto flex items-center gap-1">
			{#if running}
				<Button variant="secondary" size="icon-sm" aria-label="Stop" onclick={onStop}><Square class="fill-current" /></Button>
			{/if}
			<Button
				size="icon-sm"
				aria-label={running ? "Queue" : "Send"}
				title={running ? "Queue (sent when the turn ends)" : "Send"}
				disabled={disabled || (!text.trim() && !attachments.length)}
				onclick={(e) => send(running && (e.metaKey || e.ctrlKey))}
			>
				<ArrowUp />
			</Button>
		</div>
	</div>
</div>
