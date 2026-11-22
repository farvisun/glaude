<script lang="ts">
	// A shell under the conversation, started in its folder (Ctrl+`).
	import { attach, detach, focus, refit } from "$lib/state/terminals";

	let { id, cwd, onClose }: { id: string; cwd: string; onClose: () => void } = $props();
	let host = $state<HTMLDivElement>();
	const KEY = "glaude.drawer.h";
	let height = $state(Number(localStorage.getItem(KEY)) || 240);

	$effect(() => {
		if (!host) return;
		const termId = id;
		attach(termId, { cwd }, host);
		let frame = 0;
		const ro = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => refit(termId));
		});
		ro.observe(host);
		requestAnimationFrame(() => focus(termId));
		return () => {
			ro.disconnect();
			cancelAnimationFrame(frame);
			detach(termId);
		};
	});

	function drag(e: PointerEvent) {
		const startY = e.clientY;
		const startH = height;
		const el = e.currentTarget as HTMLElement;
		el.setPointerCapture(e.pointerId);
		const move = (ev: PointerEvent) => {
			height = Math.min(Math.max(startH + (startY - ev.clientY), 120), window.innerHeight * 0.7);
		};
		const up = () => {
			el.removeEventListener("pointermove", move);
			el.removeEventListener("pointerup", up);
			localStorage.setItem(KEY, String(Math.round(height)));
		};
		el.addEventListener("pointermove", move);
		el.addEventListener("pointerup", up);
	}
</script>

<div class="border-border bg-code-bg flex shrink-0 flex-col border-t" style="height: {height}px" role="region" aria-label="Terminal">
	<div
		class="hover:bg-primary/40 h-1 shrink-0 cursor-row-resize"
		role="separator"
		aria-orientation="horizontal"
		aria-label="Resize terminal"
		onpointerdown={drag}
	></div>
	<div class="text-muted-foreground flex items-center gap-2 px-3 pb-0.5 text-[11px]">
		<span class="truncate font-mono" dir="ltr">{cwd}</span>
		<button class="hover:text-foreground ms-auto" onclick={onClose} title="Hide (Ctrl+&#96;). The shell keeps running.">Hide</button>
	</div>
	<div bind:this={host} class="min-h-0 flex-1 ps-2" dir="ltr"></div>
</div>
