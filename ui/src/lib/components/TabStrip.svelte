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
	import FileText from "@lucide/svelte/icons/file-text";
	import GitCompare from "@lucide/svelte/icons/git-compare";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
	import ChartColumn from "@lucide/svelte/icons/chart-column";
	import { tick } from "svelte";
	import { layout, MAX_PANES, type Leaf, type TabKind } from "$lib/state/layout.svelte";
	import { drag } from "$lib/state/drag.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { isMac } from "$lib/state/prefs.svelte";
	import { tabBadge } from "$lib/state/badges.svelte";
	import { stepOf } from "$lib/state/activity";
	import { conversations } from "$lib/state/conversations.svelte";
	import Spark from "$lib/components/conv/Spark.svelte";
	import PaneMenu from "./PaneMenu.svelte";

	let { pane, showExpand, onExpandSidebar }: { pane: Leaf; showExpand: boolean; onExpandSidebar?: () => void } =
		$props();

	const icons: Record<TabKind, typeof MessageSquare> = {
		welcome: Sparkles,
		conversation: MessageSquare,
		terminal: SquareTerminal,
		accounts: Users,
		settings: Settings,
		search: Sparkles,
		file: FileText,
		changes: GitCompare,
		agents: LayoutDashboard,
		usage: ChartColumn,
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
		// Splitting a lone tab: a fresh Start pane on the right.
		if (pane.tabs.length === 1) layout.splitNew(pane.id, "right");
		else if (tab) layout.splitWith(pane.id, "right", tab.id);
	}

	const isFirstPane = $derived(layout.leaves()[0]?.id === pane.id);
	const focusedPane = $derived(layout.focused === pane.id);

	// Overflow: wheel scrolls sideways, the active tab stays in view, and a
	// list of all tabs appears when they don't fit.
	let scroller = $state<HTMLDivElement>();
	let overflowing = $state(false);
	$effect(() => {
		if (!scroller) return;
		const el = scroller;
		const check = () => (overflowing = el.scrollWidth > el.clientWidth + 1);
		const ro = new ResizeObserver(check);
		ro.observe(el);
		check();
		return () => ro.disconnect();
	});
	$effect(() => {
		void pane.tabs.length;
		const id = pane.active;
		if (!id || !scroller) return;
		const el = scroller;
		tick().then(() => {
			el.querySelector<HTMLElement>(`[data-tab-id="${id}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
			overflowing = el.scrollWidth > el.clientWidth + 1;
		});
	});
	function onWheel(e: WheelEvent) {
		if (!scroller || Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
		scroller.scrollLeft += e.deltaY;
		e.preventDefault();
	}

	const providerColor = (ref?: string) =>
		ref && conversations.byId[ref]?.provider === "codex" ? "var(--codex)" : ref && conversations.byId[ref] ? "var(--claude)" : undefined;
	function tooltip(tabId: string): string {
		const tab = layout.tabs[tabId];
		if (!tab) return "";
		const conv = tab.ref ? conversations.byId[tab.ref] : undefined;
		if (conv) {
			const step = stepOf(conv);
			return `${tab.title}\n${conv.provider === "codex" ? "Codex" : "Claude"} · ${conv.cwd}${step.tone === "idle" ? "" : `\n${step.text}`}`;
		}
		return tab.kind === "file" || tab.kind === "changes" ? `${tab.title}\n${tab.ref}` : tab.title;
	}
	const badgeLabel = { running: "Working", attention: "Needs you", unread: "Finished" } as const;
</script>

<PaneMenu paneId={pane.id}>
	{#snippet trigger(stripProps)}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	{...stripProps}
	class="bg-sidebar/60 border-border relative flex h-(--tab-h) shrink-0 items-stretch border-b {showExpand && isMac && isFirstPane
		? 'ps-[78px]'
		: ''}"
	data-tauri-drag-region
	ondragover={(e) => {
		if (drag.tabId) e.preventDefault();
	}}
	ondrop={onStripDrop}
>
	{#if showExpand}
		<Button variant="ghost" size="icon-sm" class="ms-1 self-center" aria-label="Show sidebar" onclick={onExpandSidebar}>
			<PanelLeftOpen />
		</Button>
	{/if}

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="tabs-scroll flex min-w-0 flex-1 items-stretch overflow-x-auto"
		bind:this={scroller}
		onwheel={onWheel}
		ondblclick={(e) => {
			if (e.target === e.currentTarget) {
				layout.focus(pane.id);
				ui.openNewConversation();
			}
		}}
		role="tablist"
		aria-label="Tabs"
		tabindex="-1"
		data-tauri-drag-region
	>
		{#each pane.tabs as tabId, i (tabId)}
			{@const tab = layout.tabs[tabId]}
			{#if tab}
				{@const Icon = icons[tab.kind]}
				{@const badge = tabBadge(tab)}
				<PaneMenu paneId={pane.id} {tabId}>
					{#snippet trigger(tabProps)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					{...tabProps}
					data-tab-id={tabId}
					title={tooltip(tabId)}
					class="group border-border/70 relative -mb-px flex max-w-56 min-w-0 shrink-0 cursor-default items-center gap-1.5 border-e ps-3 pe-1.5 text-[13px] select-none
					{tabId === pane.active
						? 'bg-background text-foreground border-b border-b-transparent'
						: 'text-muted-foreground hover:bg-accent/40 hover:text-foreground border-b border-b-transparent'}"
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
					{#if tabId === pane.active}
						<span class="absolute inset-x-0 top-0 h-0.5 {focusedPane ? 'bg-primary' : 'bg-muted-foreground/40'}"></span>
					{/if}
					{#if dropIndex === i && drag.tabId}
						<span class="bg-primary absolute inset-y-1.5 -start-px w-0.5 rounded"></span>
					{/if}
					<Icon class="size-3.5 shrink-0" style={providerColor(tab.ref) ? `color: ${providerColor(tab.ref)}` : undefined} />
					<span class="truncate" dir="auto">{tab.title}</span>
					<!-- Status in the close slot; the close button shows on hover (and on the active tab without status). -->
					<span class="relative ms-1 grid size-5 shrink-0 place-items-center">
						{#if badge}
							<span class="grid place-items-center group-hover:invisible" aria-label={badgeLabel[badge]} title={badgeLabel[badge]}>
								{#if badge === "running"}
									<span class="text-[12px]"><Spark color={providerColor(tab.ref) ?? "var(--claude)"} /></span>
								{:else if badge === "attention"}
									<span class="bg-attention ring-attention/30 size-2 rounded-full ring-2"></span>
								{:else}
									<span class="bg-primary size-2 rounded-full"></span>
								{/if}
							</span>
						{/if}
						<button
							class="hover:bg-accent text-muted-foreground hover:text-foreground absolute inset-0 grid place-items-center rounded {badge
								? 'invisible group-hover:visible'
								: tabId === pane.active
									? ''
									: 'invisible group-hover:visible'}"
							aria-label="Close tab"
							onclick={(e) => {
								e.stopPropagation();
								layout.closeTab(tabId);
							}}
						>
							<X class="size-3.5" />
						</button>
					</span>
				</div>
					{/snippet}
				</PaneMenu>
			{/if}
		{/each}
		{#if dropIndex === pane.tabs.length && drag.tabId}
			<span class="bg-primary my-2 w-0.5 rounded"></span>
		{/if}
	</div>

	<div class="flex shrink-0 items-center gap-0.5 px-1">
		{#if overflowing}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" aria-label="All tabs" title="All tabs in this pane">
							<ChevronDown />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="end" class="max-h-80 w-64 overflow-y-auto">
					{#each pane.tabs as tabId (tabId)}
						{@const t = layout.tabs[tabId]}
						{#if t}
							{@const TIcon = icons[t.kind]}
							{@const b = tabBadge(t)}
							<DropdownMenu.Item onclick={() => layout.activate(tabId)} class={tabId === pane.active ? "bg-accent/60" : ""}>
								<TIcon style={providerColor(t.ref) ? `color: ${providerColor(t.ref)}` : undefined} />
								<span class="min-w-0 flex-1 truncate" dir="auto">{t.title}</span>
								{#if b === "attention"}<span class="bg-attention size-2 rounded-full"></span>{:else if b === "running"}<Spark
										color={providerColor(t.ref) ?? "var(--claude)"}
									/>{:else if b === "unread"}<span class="bg-primary size-2 rounded-full"></span>{/if}
							</DropdownMenu.Item>
						{/if}
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
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
	{/snippet}
</PaneMenu>

<style>
	/* Tabs scroll sideways without a visible scrollbar. */
	.tabs-scroll {
		scrollbar-width: none;
	}
	.tabs-scroll::-webkit-scrollbar {
		display: none;
	}
</style>
