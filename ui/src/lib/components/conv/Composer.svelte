<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import ArrowUp from "@lucide/svelte/icons/arrow-up";
	import Square from "@lucide/svelte/icons/square";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import X from "@lucide/svelte/icons/x";
	import { detectDir } from "$lib/bidi";
	import type { Attachment } from "$lib/types";

	let {
		running,
		disabled = false,
		slashCommands = [],
		placeholder = "Message",
		onSend,
		onStop,
		onAttach,
		onPasteImage,
	}: {
		running: boolean;
		disabled?: boolean;
		slashCommands?: string[];
		placeholder?: string;
		onSend: (text: string, attachments: Attachment[]) => void;
		onStop: () => void;
		onAttach?: () => Promise<Attachment[]>;
		onPasteImage?: (blob: Blob, name: string) => Promise<Attachment | null>;
	} = $props();

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

	function send() {
		const t = text.trim();
		if ((!t && attachments.length === 0) || disabled) return;
		onSend(t, attachments);
		text = "";
		attachments = [];
	}

	function onKey(e: KeyboardEvent) {
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
		if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
			e.preventDefault();
			send();
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
		{placeholder}
		class="placeholder:text-muted-foreground block max-h-[280px] w-full resize-none bg-transparent px-3.5 pt-3 pb-1 text-[15px] leading-7 outline-none"
		onkeydown={onKey}
		onpaste={onPaste}
	></textarea>

	<div class="flex items-center gap-1 px-2 pb-2">
		{#if onAttach}
			<Button variant="ghost" size="icon-sm" aria-label="Attach files" onclick={attach}><Paperclip /></Button>
		{/if}
		<div class="ms-auto flex items-center gap-1">
			{#if running}
				<Button variant="secondary" size="icon-sm" aria-label="Stop" onclick={onStop}><Square class="fill-current" /></Button>
			{/if}
			<Button size="icon-sm" aria-label="Send" disabled={disabled || (!text.trim() && !attachments.length)} onclick={send}>
				<ArrowUp />
			</Button>
		</div>
	</div>
</div>
