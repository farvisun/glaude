<script lang="ts">
	// The result of Check: can Anthropic and OpenAI be reached, and from where.
	import type { CheckResult } from "$lib/state/proxies.svelte";
	import CircleCheck from "@lucide/svelte/icons/circle-check";
	import CircleX from "@lucide/svelte/icons/circle-x";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";

	let { result, busy = false }: { result: CheckResult | null; busy?: boolean } = $props();
	const reachable = $derived(result?.probes.filter((p) => p.target !== "Exit address") ?? []);
</script>

{#if busy}
	<p class="text-muted-foreground flex items-center gap-1.5 text-[12px]" role="status"><LoaderCircle class="size-3.5 animate-spin" /> Checking through the proxy…</p>
{:else if result}
	<ul class="flex flex-col gap-1 text-[12px]" aria-label="Check result">
		{#each reachable as p (p.target)}
			<li class="flex min-w-0 items-center gap-1.5">
				{#if p.ok}<CircleCheck class="text-ok size-3.5 shrink-0" />{:else}<CircleX class="text-destructive size-3.5 shrink-0" />{/if}
				<span class="font-medium">{p.target}</span>
				<span class="text-muted-foreground min-w-0 truncate" title={p.error ?? undefined}
					>{p.ok ? `reachable · ${p.ms} ms` : (p.error ?? "not reachable")}</span
				>
			</li>
		{/each}
		<li class="text-muted-foreground flex items-center gap-1.5 ps-5">
			{#if result.ip}Exit address <span class="text-foreground font-mono" dir="ltr">{result.ip}</span>{#if result.country}<span>· {result.country}</span>{/if}{:else}Exit address unknown{/if}
		</li>
	</ul>
{/if}
