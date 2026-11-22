<script lang="ts">
	// Rewind to before a user message: undo Claude's file edits since then,
	// fork the conversation from there, or both (like /rewind).
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Sheet from "$lib/components/ui/sheet";
	import { Button } from "$lib/components/ui/button";
	import { ui } from "$lib/state/ui.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { convActions, showError } from "$lib/state/sessions";
	import { narrow } from "$lib/state/media.svelte";
	import { toast } from "svelte-sonner";
	import FilePen from "@lucide/svelte/icons/file-pen";

	type Choice = "both" | "conversation" | "code";

	const conv = $derived(ui.rewind ? conversations.get(ui.rewind.convId) : undefined);
	const item = $derived(conv && ui.rewind ? conv.items.find((i) => i.id === ui.rewind?.itemId) : undefined);
	const text = $derived.by(() => {
		if (item?.kind !== "user") return "";
		const div = document.createElement("div");
		div.innerHTML = item.blocks.map((b) => b.html).join("\n");
		return (div.innerText || div.textContent || "").trim();
	});

	let preview = $state<{ canRewind: boolean; error?: string; filesChanged?: string[]; insertions?: number; deletions?: number } | null>(null);
	let loading = $state(false);
	let busy = $state(false);
	let choice = $state<Choice>("conversation");

	$effect(() => {
		const r = ui.rewind;
		if (!r || !conv) return;
		preview = null;
		loading = true;
		choice = "conversation";
		convActions
			.rewindFiles(conv, r.itemId, true)
			.then((p) => {
				preview = p;
				if (p.canRewind && p.filesChanged?.length) choice = "both";
			})
			.catch((e) => (preview = { canRewind: false, error: e instanceof Error ? e.message : String(e) }))
			.finally(() => (loading = false));
	});

	const rel = (f: string) => (conv && f.startsWith(conv.cwd + "/") ? f.slice(conv.cwd.length + 1) : f);
	const files = $derived(preview?.canRewind ? (preview.filesChanged ?? []) : []);
	const choices = $derived(
		[
			{ id: "both" as Choice, label: "Restore code and conversation", hint: "Undo file edits and continue from before this message.", needsFiles: true },
			{ id: "conversation" as Choice, label: "Restore conversation", hint: "Fork the conversation from before this message; files stay as they are.", needsFiles: false },
			{ id: "code" as Choice, label: "Restore code", hint: "Undo file edits made since this message; keep the conversation.", needsFiles: true },
		].filter((c) => !c.needsFiles || files.length > 0),
	);

	const open = {
		get value() {
			return !!ui.rewind;
		},
		set value(v: boolean) {
			if (!v && !busy) ui.rewind = null;
		},
	};

	async function run() {
		if (!conv || !ui.rewind) return;
		const id = ui.rewind.itemId;
		busy = true;
		try {
			if (conv.running) await convActions.interrupt(conv);
			if (choice !== "conversation") {
				const r = await convActions.rewindFiles(conv, id, false);
				if (!r.canRewind) throw new Error(r.error ?? "Claude could not restore the files");
			}
			if (choice !== "code") {
				const ok = await convActions.rewindConversation(conv, id, text);
				if (!ok) return;
			}
			toast.success(choice === "code" ? "Files restored" : choice === "conversation" ? "Conversation rewound" : "Code and conversation restored");
			busy = false;
			ui.rewind = null;
		} catch (e) {
			showError("Could not rewind", e);
		} finally {
			busy = false;
		}
	}
</script>

{#snippet body()}
	{#if text}
		<blockquote class="bg-muted/50 border-border line-clamp-3 rounded-md border-s-2 px-3 py-1.5 text-[13px]" dir="auto">{text}</blockquote>
	{/if}
	<div class="flex flex-col gap-1.5" role="radiogroup" aria-label="What to restore">
		{#each choices as c (c.id)}
			<button
				class="flex flex-col rounded-md border px-3 py-2 text-start text-[13px] transition-colors {choice === c.id
					? 'border-primary bg-primary/10'
					: 'border-border hover:bg-accent/50'}"
				role="radio"
				aria-checked={choice === c.id}
				onclick={() => (choice = c.id)}
			>
				<span class="font-medium">{c.label}</span>
				<span class="text-muted-foreground text-xs">{c.hint}</span>
			</button>
		{/each}
	</div>
	<div class="text-[13px]">
		{#if loading}
			<div class="skeleton-shimmer h-10 rounded-md"></div>
		{:else if files.length}
			<div class="text-muted-foreground pb-1 text-xs">
				{files.length} file{files.length === 1 ? "" : "s"} changed since this message · <span class="text-ok">+{preview?.insertions ?? 0}</span>
				<span class="text-destructive">−{preview?.deletions ?? 0}</span>
			</div>
			<ul class="scrollbar-thin flex max-h-32 flex-col gap-0.5 overflow-y-auto">
				{#each files as f (f)}
					<li class="flex min-w-0 items-center gap-1.5 font-mono text-[12px]" dir="ltr"><FilePen class="text-muted-foreground size-3 shrink-0" /><span class="truncate" title={f}>{rel(f)}</span></li>
				{/each}
			</ul>
		{:else}
			<p class="text-muted-foreground text-xs">
				{preview?.error && !/no.*checkpoint|not found/i.test(preview.error) ? preview.error : "No file changes to undo since this message."}
			</p>
		{/if}
	</div>
{/snippet}

{#snippet actions()}
	<Button variant="ghost" onclick={() => (ui.rewind = null)} disabled={busy}>Cancel</Button>
	<Button onclick={run} disabled={busy || loading}>{busy ? "Rewinding…" : "Rewind"}</Button>
{/snippet}

{#if narrow.current}
	<Sheet.Root bind:open={open.value}>
		<Sheet.Content side="bottom" class="max-h-[90vh] gap-3 px-4 pb-4">
			<Sheet.Header class="px-0">
				<Sheet.Title>Rewind</Sheet.Title>
				<Sheet.Description>Go back to before this message.</Sheet.Description>
			</Sheet.Header>
			{@render body()}
			<div class="flex justify-end gap-2">{@render actions()}</div>
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Dialog.Root bind:open={open.value}>
		<Dialog.Content class="sm:max-w-lg">
			<Dialog.Header>
				<Dialog.Title>Rewind</Dialog.Title>
				<Dialog.Description>Go back to before this message.</Dialog.Description>
			</Dialog.Header>
			<div class="flex flex-col gap-3">{@render body()}</div>
			<Dialog.Footer>{@render actions()}</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>
{/if}
