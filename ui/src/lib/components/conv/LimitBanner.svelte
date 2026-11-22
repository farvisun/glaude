<script lang="ts">
	// Usage limit reached: when it resets, and continue automatically then.
	import { Button } from "$lib/components/ui/button";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import Hourglass from "@lucide/svelte/icons/hourglass";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";

	let {
		limitedUntil,
		resumeAt,
		onSchedule,
	}: { limitedUntil: string | null; resumeAt: string | null; onSchedule: (at: string | null) => void } = $props();

	const fmt = (iso: string) => {
		const d = new Date(iso);
		const today = d.toDateString() === new Date().toDateString();
		return d.toLocaleString(undefined, today ? { hour: "numeric", minute: "2-digit" } : { weekday: "short", hour: "numeric", minute: "2-digit" });
	};
	const known = $derived(!!limitedUntil && !Number.isNaN(Date.parse(limitedUntil)));
	const inHours = (h: number) => new Date(Date.now() + h * 3600e3).toISOString();
</script>

{#if limitedUntil !== null || resumeAt}
	<div class="border-attention/50 bg-attention/8 mb-1.5 flex flex-wrap items-center gap-2 rounded-lg border px-3 py-1.5 text-[13px]" role="status">
		<Hourglass class="text-attention size-4 shrink-0" />
		{#if resumeAt}
			<span class="min-w-0 flex-1">Continues automatically at <span class="font-medium">{fmt(resumeAt)}</span>, even if Glaude is closed.</span>
			<Button size="sm" variant="ghost" class="h-7" onclick={() => onSchedule(null)}>Cancel</Button>
		{:else}
			<span class="min-w-0 flex-1">
				Usage limit reached{known && limitedUntil ? ` · resets ${fmt(limitedUntil)}` : ""}.
			</span>
			<div class="flex items-center">
				<Button size="sm" class="h-7 rounded-e-none" onclick={() => onSchedule(known && limitedUntil ? limitedUntil : inHours(1))}
					>{known ? "Resume at reset" : "Resume in 1 hour"}</Button
				>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} size="sm" class="h-7 rounded-s-none border-s px-1.5" aria-label="Other times"><ChevronDown /></Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Item onclick={() => onSchedule(inHours(1))}>In 1 hour</DropdownMenu.Item>
						<DropdownMenu.Item onclick={() => onSchedule(inHours(3))}>In 3 hours</DropdownMenu.Item>
						<DropdownMenu.Item onclick={() => onSchedule(inHours(5))}>In 5 hours</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		{/if}
	</div>
{/if}
