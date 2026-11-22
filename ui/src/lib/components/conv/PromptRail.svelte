<script lang="ts">
	// A slim outline of the user's prompts at the edge of a conversation:
	// one mark per prompt, the one in view highlighted; click to jump.
	let {
		prompts,
		activeId,
		onPick,
	}: { prompts: { id: string; text: string }[]; activeId: string | null; onPick: (id: string) => void } = $props();

	let hover = $state<{ text: string; y: number } | null>(null);
</script>

<nav class="prompt-rail absolute inset-y-6 end-1.5 z-10 flex w-4 flex-col items-end justify-center gap-0.5" aria-label="Your prompts">
	{#each prompts as p, i (p.id)}
		<button
			class="group/tick flex h-2.5 w-4 shrink items-center justify-end"
			style="min-height: 3px"
			aria-label="Prompt {i + 1}: {p.text.slice(0, 60)}"
			onclick={() => onPick(p.id)}
			onmouseenter={(e) => {
				const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
				const box = (e.currentTarget as HTMLElement).closest(".prompt-rail")!.getBoundingClientRect();
				hover = { text: p.text, y: r.top - box.top + r.height / 2 };
			}}
			onmouseleave={() => (hover = null)}
		>
			<span
				class="block h-0.5 rounded-full transition-all group-hover/tick:w-4 {p.id === activeId
					? 'bg-primary w-4'
					: 'bg-muted-foreground/40 group-hover/tick:bg-foreground w-2.5'}"
			></span>
		</button>
	{/each}
	{#if hover}
		<div
			class="bg-popover text-popover-foreground border-border pointer-events-none absolute end-6 w-64 -translate-y-1/2 rounded-md border px-2.5 py-1.5 text-[12px] shadow-md"
			style="top: {hover.y}px"
			dir="auto"
		>
			<span class="line-clamp-3">{hover.text}</span>
		</div>
	{/if}
</nav>
