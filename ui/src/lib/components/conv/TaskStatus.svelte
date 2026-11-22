<script lang="ts">
	// Status mark for a task or workflow agent: ✻ while running, then a result icon.
	import Spark from "./Spark.svelte";
	import Check from "@lucide/svelte/icons/check";
	import XIcon from "@lucide/svelte/icons/x";
	import Ban from "@lucide/svelte/icons/ban";
	import Clock from "@lucide/svelte/icons/clock";

	let { status, color = "var(--claude)", size = 14 }: { status: string; color?: string; size?: number } = $props();
</script>

{#if status === "running" || status === "start" || status === "progress"}
	<span style:font-size="{size}px" aria-label="Running"><Spark {color} /></span>
{:else if status === "completed" || status === "done"}
	<Check class="text-ok shrink-0" style="width:{size}px;height:{size}px" aria-label="Done" />
{:else if status === "failed" || status === "error"}
	<XIcon class="text-destructive shrink-0" style="width:{size}px;height:{size}px" aria-label="Failed" />
{:else if status === "pending" || status === "paused"}
	<Clock class="text-muted-foreground shrink-0" style="width:{size}px;height:{size}px" aria-label={status} />
{:else}
	<Ban class="text-muted-foreground shrink-0" style="width:{size}px;height:{size}px" aria-label="Stopped" />
{/if}
