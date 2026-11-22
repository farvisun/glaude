<script lang="ts">
	import type { PlanStep } from "$lib/types";
	import Circle from "@lucide/svelte/icons/circle";
	import CircleDot from "@lucide/svelte/icons/circle-dot";
	import CircleCheck from "@lucide/svelte/icons/circle-check";

	let { steps, explanation, compact = false }: { steps: PlanStep[]; explanation?: string; compact?: boolean } = $props();
</script>

<div class="flex flex-col gap-1 {compact ? '' : 'border-border bg-card/40 rounded-lg border px-3 py-2.5'}">
	{#if explanation}<p class="text-muted-foreground pb-1 text-[13px]" dir="auto">{explanation}</p>{/if}
	<ul class="flex flex-col gap-1">
		{#each steps as s, i (i)}
			<li class="flex items-start gap-2 text-[13px] leading-snug">
				{#if s.status === "completed"}
					<CircleCheck class="text-ok mt-0.5 size-3.5 shrink-0" />
				{:else if s.status === "inProgress" || s.status === "in_progress"}
					<CircleDot class="text-primary mt-0.5 size-3.5 shrink-0" />
				{:else}
					<Circle class="text-muted-foreground mt-0.5 size-3.5 shrink-0" />
				{/if}
				<span
					class="min-w-0 {s.status === 'completed' ? 'text-muted-foreground line-through decoration-1' : ''} {s.status ===
						'inProgress' || s.status === 'in_progress'
						? 'font-medium'
						: ''}"
					dir="auto">{s.text}</span
				>
			</li>
		{/each}
	</ul>
</div>
