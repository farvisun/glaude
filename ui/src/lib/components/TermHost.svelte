<script lang="ts">
	// Lends a terminal (from terminals.ts) a place on screen and keeps it fitted.
	import type { TermSpec } from "$lib/state/layout.svelte";
	import { attach, detach, focus, refit } from "$lib/state/terminals";

	let { id, spec, onFocus, autofocus = false }: { id: string; spec: TermSpec; onFocus?: () => void; autofocus?: boolean } = $props();
	let host = $state<HTMLDivElement>();

	$effect(() => {
		if (!host) return;
		const tid = id;
		attach(tid, spec, host);
		let frame = 0;
		const ro = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => refit(tid));
		});
		ro.observe(host);
		if (autofocus) requestAnimationFrame(() => focus(tid));
		return () => {
			ro.disconnect();
			cancelAnimationFrame(frame);
			detach(tid);
		};
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="h-full w-full min-w-0 ps-2 pt-1" bind:this={host} onfocusin={onFocus} onmousedown={onFocus} dir="ltr"></div>
