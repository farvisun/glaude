<script lang="ts">
	import * as Resizable from "$lib/components/ui/resizable";
	import type { PaneNode } from "$lib/state/layout.svelte";
	import LeafPane from "./LeafPane.svelte";
	import Self from "./PaneView.svelte";

	let {
		node,
		sidebarCollapsed = false,
		onExpandSidebar,
		first = true,
	}: { node: PaneNode; sidebarCollapsed?: boolean; onExpandSidebar?: () => void; first?: boolean } = $props();
</script>

{#if node.type === "leaf"}
	<LeafPane pane={node} showExpand={sidebarCollapsed && first} {onExpandSidebar} />
{:else}
	<Resizable.PaneGroup direction={node.dir} autoSaveId={`glaude-split-${node.id}`}>
		<Resizable.Pane defaultSize={50} minSize={15}>
			<Self node={node.a} {sidebarCollapsed} {onExpandSidebar} {first} />
		</Resizable.Pane>
		<Resizable.Handle />
		<Resizable.Pane defaultSize={50} minSize={15}>
			<Self node={node.b} {sidebarCollapsed} {onExpandSidebar} first={false} />
		</Resizable.Pane>
	</Resizable.PaneGroup>
{/if}
