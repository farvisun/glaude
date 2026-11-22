<script lang="ts">
	// Right-click menu of a pane or tab: split right/down with this tab, a new
	// conversation or a new terminal; copy the selection; close the tab.
	// Text fields and the terminal keep their own menu unless Shift is held.
	import type { Snippet } from "svelte";
	import { ContextMenu as Primitive } from "bits-ui";
	import * as ContextMenu from "$lib/components/ui/context-menu";
	import { layout, MAX_PANES, type Edge } from "$lib/state/layout.svelte";
	import { canMove, canSplit, splitPane, type SplitWhat } from "$lib/state/panes";
	import Columns2 from "@lucide/svelte/icons/columns-2";
	import Rows2 from "@lucide/svelte/icons/rows-2";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import ArrowRightLeft from "@lucide/svelte/icons/arrow-right-left";
	import Copy from "@lucide/svelte/icons/copy";
	import X from "@lucide/svelte/icons/x";

	let {
		paneId,
		tabId = null,
		trigger,
	}: {
		paneId: string;
		/** The tab right-clicked; the pane's active tab when null. */
		tabId?: string | null;
		trigger: Snippet<[Record<string, unknown>]>;
	} = $props();

	const subject = $derived(tabId ?? layout.findLeaf(paneId)?.active ?? null);
	let native = $state(false);
	let selection = $state("");

	// Decided on the pointer press, before the contextmenu event.
	function onpointerdown(e: PointerEvent) {
		if (e.button !== 2 && !(e.button === 0 && e.ctrlKey)) return;
		const t = e.target as HTMLElement | null;
		native = !e.shiftKey && !!t?.closest("textarea, input, [contenteditable='true'], .xterm");
	}

	function onOpenChange(open: boolean) {
		if (open) selection = window.getSelection()?.toString() ?? "";
	}

	const splits: { edge: Edge; label: string; Icon: typeof Columns2 }[] = [
		{ edge: "right", label: "Split right", Icon: Columns2 },
		{ edge: "bottom", label: "Split down", Icon: Rows2 },
	];
	const run = (edge: Edge, what: SplitWhat) => splitPane(paneId, edge, what, subject);
</script>

<ContextMenu.Root {onOpenChange}>
	<Primitive.Trigger disabled={native}>
		{#snippet child({ props })}
			{@const theirs = props.onpointerdown as ((e: PointerEvent) => void) | undefined}
			{@render trigger({
				...props,
				onpointerdown: (e: PointerEvent) => {
					onpointerdown(e);
					theirs?.(e);
				},
			})}
		{/snippet}
	</Primitive.Trigger>
	<ContextMenu.Content class="w-56">
		{#each splits as s (s.edge)}
			<ContextMenu.Sub>
				<ContextMenu.SubTrigger disabled={!canSplit()}>
					<s.Icon />
					{s.label}
				</ContextMenu.SubTrigger>
				<ContextMenu.SubContent class="w-52">
					<ContextMenu.Item disabled={!canMove(subject)} onclick={() => run(s.edge, "move")}>
						<ArrowRightLeft /> Move this tab
					</ContextMenu.Item>
					<ContextMenu.Item onclick={() => run(s.edge, "conversation")}>
						<MessageSquare /> New conversation
					</ContextMenu.Item>
					<ContextMenu.Item onclick={() => run(s.edge, "terminal")}>
						<SquareTerminal /> New terminal
					</ContextMenu.Item>
				</ContextMenu.SubContent>
			</ContextMenu.Sub>
		{/each}
		{#if !canSplit()}
			<ContextMenu.Label class="text-muted-foreground text-xs font-normal">{MAX_PANES} panes is the most</ContextMenu.Label>
		{/if}
		{#if selection}
			<ContextMenu.Separator />
			<ContextMenu.Item onclick={() => navigator.clipboard?.writeText(selection)}>
				<Copy /> Copy
			</ContextMenu.Item>
		{/if}
		{#if tabId}
			<ContextMenu.Separator />
			<ContextMenu.Item onclick={() => layout.closeTab(tabId)}>
				<X /> Close tab
			</ContextMenu.Item>
		{/if}
	</ContextMenu.Content>
</ContextMenu.Root>
