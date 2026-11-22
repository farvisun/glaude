<script lang="ts">
	// One setting: label and help on the start side, its control on the end.
	// Hidden when the Settings search does not match it.
	import { getContext, type Snippet } from "svelte";

	let {
		label,
		description,
		keywords = "",
		children,
		stacked = false,
	}: { label: string; description?: string; keywords?: string; children: Snippet; stacked?: boolean } = $props();

	const query = getContext<() => string>("settings-query");
	const shown = $derived.by(() => {
		const q = query?.().trim().toLowerCase() ?? "";
		return !q || `${label} ${description ?? ""} ${keywords}`.toLowerCase().includes(q);
	});
</script>

{#if shown}
	<div class="setting-row border-border/60 flex gap-x-6 gap-y-2 border-b py-3 text-[13px] last:border-b-0 {stacked ? 'flex-col' : 'flex-wrap items-center justify-between'}">
		<div class="min-w-0 {stacked ? '' : 'flex-1 basis-60'}">
			<div class="font-medium">{label}</div>
			{#if description}<div class="text-muted-foreground mt-0.5 text-xs leading-relaxed">{description}</div>{/if}
		</div>
		<div class="flex shrink-0 items-center gap-2">{@render children()}</div>
	</div>
{/if}
