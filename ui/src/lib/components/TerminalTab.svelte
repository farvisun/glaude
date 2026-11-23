<script lang="ts">
	import { themes } from "$lib/themes/themes.svelte";
	import type { Tab } from "$lib/state/layout.svelte";
	import { attach, detach, focus, refit } from "$lib/state/terminals";

	const termIsland = $derived(themes.terminalIsland);

	let { tab, active }: { tab: Tab; active: boolean } = $props();
	let host = $state<HTMLDivElement>();

	// The xterm instance outlives this component (see terminals.ts).
	$effect(() => {
		if (!host) return;
		const id = tab.id;
		attach(id, $state.snapshot(tab.term) ?? {}, host);
		let frame = 0;
		const ro = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => refit(id));
		});
		ro.observe(host);
		return () => {
			ro.disconnect();
			cancelAnimationFrame(frame);
			detach(id);
		};
	});

	$effect(() => {
		if (active && host) requestAnimationFrame(() => focus(tab.id));
	});
</script>

<div
	class="h-full w-full overflow-hidden bg-(--term-bg) ps-2 pt-1.5 {termIsland ? `theme-island ${termIsland.kind}` : ''}"
	style={termIsland?.style}
	data-tab={tab.id}
>
	<div bind:this={host} class="h-full w-full" dir="ltr"></div>
</div>
