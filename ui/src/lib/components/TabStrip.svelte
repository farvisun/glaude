<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import X from "@lucide/svelte/icons/x";
	import Plus from "@lucide/svelte/icons/plus";
	import PanelLeftOpen from "@lucide/svelte/icons/panel-left-open";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import Users from "@lucide/svelte/icons/users";
	import Settings from "@lucide/svelte/icons/settings";
	import Sparkles from "@lucide/svelte/icons/sparkles";
	import Columns2 from "@lucide/svelte/icons/columns-2";
	import { layout, MAX_PANES, type Leaf, type TabKind } from "$lib/state/layout.svelte";
	import { drag } from "$lib/state/drag.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { isMac } from "$lib/state/prefs.svelte";
	import { tabBadge } from "$lib/state/badges.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import Spark from "$lib/components/conv/Spark.svelte";

	let { pane, showExpand, onExpandSidebar }: { pane: Leaf; showExpand: boolean; onExpandSidebar?: () => void } =
		$props();

	const icons: Record<TabKind, typeof MessageSquare> = {
		welcome: Sparkles,
		conversation: MessageSquare,
		terminal: SquareTerminal,
		accounts: Users,
		settings: Settings,
		search: Sparkles,
	};

	let dropIndex = $state<number | null>(null);

	function onDragStart(e: DragEvent, tabId: string) {
		drag.tabId = tabId;
		e.dataTransfer?.setData("text/plain", tabId);
		if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
	}

	function onStripDrop(e: DragEvent) {
		e.preventDefault();
		const id = drag.tabId;
		drag.tabId = null;
		const at = dropIndex ?? pane.tabs.length;
		dropIndex = null;
		if (id) layout.moveTab(id, pane.id, at);
	}

	function splitActive() {
		if (!pane.active) return;
		const tab = layout.tabs[pane.active];
		// Splitting a lone tab: open a fresh welcome tab on the right.
		if (pane.tabs.length === 1) {
			const id = layout.openTab({ kind: "welcome", title: "Start" }, pane.id);
			layout.splitWith(pane.id, "right", id);
		} else if (tab) layout.splitWith(pane.id, "right", tab.id);
	}

	const isFirstPane = $derived(layout.leaves()[0]?.id === pane.id);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="bg-sidebar/60 border-border flex h-11 shrink-0 items-end gap-0.5 border-b px-1.5 {showExpand && isMac && isFirstPane
		? 'ps-[78px]'
		: ''}"
	data-tauri-drag-region
	ondragover={(e) => {
		if (drag.tabId) e.preventDefault();
	}}
	ondrop={onStripDrop}
>
	{#if showExpand}
		<Button variant="ghost" size="icon-sm" class="mb-1.5" aria-label="Show sidebar" onclick={onExpandSidebar}>
			<PanelLeftOpen />
		</Button>
	{/if}

	<div class="scrollbar-thin flex min-w-0 flex-1 items-end gap-0.5 overflow-x-auto" data-tauri-drag-region>
		{#each pane.tabs as tabId, i (tabId)}
			{@const tab = layout.tabs[tabId]}
			{#if tab}
				{@const Icon = icons[tab.kind]}
				{@const badge = tabBadge(tab)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="group relative flex h-8 max-w-56 min-w-0 shrink-0 cursor-default items-center gap-1.5 rounded-t-md ps-2.5 pe-1 text-[13px]
					{tabId === pane.active ? 'bg-background text-foreground' : 'text-muted-foreground hover:bg-accent/50'}"
					draggable="true"
					ondragstart={(e) => onDragStart(e, tabId)}
					ondragend={() => {
						drag.tabId = null;
						dropIndex = null;
					}}
					ondragover={(e) => {
						if (!drag.tabId) return;
						e.preventDefault();
						const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
						dropIndex = e.clientX < r.left + r.width / 2 ? i : i + 1;
					}}
					onmousedown={(e) => {
						if (e.button === 1) {
							e.preventDefault();
							layout.closeTab(tabId);
						}
					}}
					onclick={() => layout.activate(tabId)}
					role="tab"
					tabindex="0"
					aria-selected={tabId === pane.active}
					onkeydown={(e) => e.key === "Enter" && layout.activate(tabId)}
				>
					{#if dropIndex === i && drag.tabId}
						<span class="bg-primary absolute inset-y-1 -start-px w-0.5 rounded"></span>
					{/if}
					<Icon class="size-3.5 shrink-0 {badge === 'attention' ? 'text-attention' : ''}" />
					<span class="truncate" dir="auto">{tab.title}</span>
					{#if badge === "running"}
						<span class="text-[12px]" aria-label="Running"><Spark color={tab.ref && conversations.byId[tab.ref]?.provider === "codex" ? "var(--codex)" : "var(--claude)"} /></span>
					{:else if badge === "attention"}
						<span class="bg-attention size-1.5 shrink-0 rounded-full" aria-label="Needs approval"></span>
					{/if}
					<button
						class="hover:bg-accent ms-auto grid size-5 shrink-0 place-items-center rounded opacity-0 group-hover:opacity-100 {tabId ===
						pane.active
							? 'opacity-60'
							: ''}"
						aria-label="Close tab"
						onclick={(e) => {
							e.stopPropagation();
							layout.closeTab(tabId);
						}}
					>
						<X class="size-3" />
					</button>
				</div>
			{/if}
		{/each}
		{#if dropIndex === pane.tabs.length && drag.tabId}
			<span class="bg-primary mb-1.5 h-5 w-0.5 rounded"></span>
		{/if}
	</div>

	<div class="mb-1.5 flex shrink-0 items-center">
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="ghost" size="icon-sm" aria-label="New tab">
						<Plus />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="end">
				<DropdownMenu.Item
					onclick={() => {
						layout.focus(pane.id);
						ui.openNewConversation();
					}}
				>
					<MessageSquare /> New conversation
				</DropdownMenu.Item>
				<DropdownMenu.Item
					onclick={() => {
						layout.focus(pane.id);
						ui.openTerminal();
					}}
				>
					<SquareTerminal /> New terminal
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
		{#if layout.leaves().length < MAX_PANES}
			<Button variant="ghost" size="icon-sm" aria-label="Split right" onclick={splitActive}>
				<Columns2 />
			</Button>
		{/if}
	</div>
</div>
