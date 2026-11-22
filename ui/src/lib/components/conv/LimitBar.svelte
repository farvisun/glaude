<script lang="ts">
	import type { LimitWindow } from "$lib/types";
	import { fmtUntil, leftTone } from "$lib/format";

	// resetClass: the statusline hides the reset time when it runs out of room.
	let { label, win, wide = false, resetClass = "" }: { label: string; win?: LimitWindow; wide?: boolean; resetClass?: string } = $props();

	const expired = $derived(!!win?.resetsAt && new Date(win.resetsAt).getTime() < Date.now());
	const left = $derived(win ? (expired ? 100 : Math.max(0, Math.min(100, 100 - win.used))) : null);
	const tone = $derived(left == null ? "ok" : leftTone(left));
	const toneVar = { ok: "var(--ok)", attention: "var(--attention)", destructive: "var(--destructive)" };
</script>

{#if left != null}
	<span class="inline-flex items-center gap-1.5" title={win?.resetsAt ? `Resets in ${fmtUntil(win.resetsAt)}` : undefined}>
		<span class="text-muted-foreground">{label}</span>
		<span class="bg-muted relative h-1.5 overflow-hidden rounded-full {wide ? 'w-16' : 'w-8'}">
			<span class="absolute inset-y-0 start-0 rounded-full" style:width="{left}%" style:background={toneVar[tone]}></span>
		</span>
		<span>{Math.round(left)}%</span>
		{#if win?.resetsAt && !expired}<span class="text-muted-foreground {resetClass}">↻{fmtUntil(win.resetsAt)}</span>{/if}
	</span>
{/if}
