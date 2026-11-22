<script lang="ts">
	import { layout, MAX_PANES, type Edge, type Leaf } from "$lib/state/layout.svelte";
	import { drag } from "$lib/state/drag.svelte";
	import TabStrip from "./TabStrip.svelte";
	import TabBody from "./TabBody.svelte";
	import PaneMenu from "./PaneMenu.svelte";

	let { pane, showExpand, onExpandSidebar }: { pane: Leaf; showExpand: boolean; onExpandSidebar?: () => void } =
		$props();

	let hoverEdge = $state<Edge | "center" | null>(null);

	const canSplit = $derived(layout.leaves().length < MAX_PANES);
	const focused = $derived(layout.focused === pane.id);
	const multi = $derived(layout.leaves().length > 1);

	function edgeAt(e: DragEvent, el: HTMLElement): Edge | "center" {
		const r = el.getBoundingClientRect();
		const x = (e.clientX - r.left) / r.width;
		const y = (e.clientY - r.top) / r.height;
		if (!canSplit) return "center";
		if (x < 0.2) return "left";
		if (x > 0.8) return "right";
		if (y < 0.2) return "top";
		if (y > 0.8) return "bottom";
		return "center";
	}

	function onDragOver(e: DragEvent) {
		if (!drag.tabId) return;
		e.preventDefault();
		hoverEdge = edgeAt(e, e.currentTarget as HTMLElement);
	}

	function onDrop(e: DragEvent) {
		const tabId = drag.tabId;
		const edge = hoverEdge;
		hoverEdge = null;
		drag.tabId = null;
		if (!tabId || !edge) return;
		e.preventDefault();
		if (edge === "center") layout.moveTab(tabId, pane.id);
		else layout.splitWith(pane.id, edge, tabId);
	}

	const overlay: Record<Edge | "center", string> = {
		left: "inset-y-0 start-0 w-1/2",
		right: "inset-y-0 end-0 w-1/2",
		top: "inset-x-0 top-0 h-1/2",
		bottom: "inset-x-0 bottom-0 h-1/2",
		center: "inset-0",
	};
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<section
	class="relative flex h-full min-h-0 min-w-0 flex-col {multi && focused ? 'pane-focused' : ''}"
	onpointerdown={() => layout.focus(pane.id)}
>
	<TabStrip {pane} {showExpand} {onExpandSidebar} />
	<PaneMenu paneId={pane.id}>
		{#snippet trigger(props)}
	<div
		{...props}
		class="relative min-h-0 flex-1"
		ondragover={onDragOver}
		ondragleave={() => (hoverEdge = null)}
		ondrop={onDrop}
	>
		{#each pane.tabs as tabId (tabId)}
			{@const tab = layout.tabs[tabId]}
			{#if tab && (tab.kind === "terminal" || tabId === pane.active)}
				<div class="absolute inset-0 {tabId === pane.active ? 'motion-fade' : 'invisible'}">
					<TabBody {tab} active={tabId === pane.active} />
				</div>
			{/if}
		{/each}
		{#if drag.tabId && hoverEdge}
			<div class="bg-primary/15 border-primary/60 pointer-events-none absolute z-20 rounded-md border-2 {overlay[hoverEdge]}"></div>
		{/if}
	</div>
		{/snippet}
	</PaneMenu>
</section>

<style>
	.pane-focused::after {
		content: "";
		position: absolute;
		inset-inline: 0;
		top: 0;
		height: 2px;
		background: var(--primary);
		opacity: 0.7;
		pointer-events: none;
	}
</style>
