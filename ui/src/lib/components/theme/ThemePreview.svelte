<script lang="ts">
	// A small picture of a theme: sidebar, a reply, code and terminal colours.
	import type { Theme } from "$lib/themes/types";
	import { themeVars } from "$lib/themes/vars";
	import { prefs } from "$lib/state/prefs.svelte";

	let { theme, size = "sm", accent }: { theme: Theme; size?: "sm" | "lg"; accent?: string } = $props();
	const style = $derived(
		Object.entries(themeVars(theme, accent ?? prefs.accent))
			.map(([k, v]) => `${k}: ${v}`)
			.join("; "),
	);
</script>

{#if size === "sm"}
	<div class="flex h-[4.25rem] overflow-hidden rounded-md" {style} aria-hidden="true">
		<div class="flex w-[28%] flex-col gap-1 border-e border-(--sidebar-border) bg-(--sidebar) p-1.5">
			<span class="h-1.5 rounded-[2px] bg-(--primary)"></span>
			<span class="h-1 w-3/4 rounded-[2px] bg-(--muted-foreground) opacity-60"></span>
			<span class="h-1 w-1/2 rounded-[2px] bg-(--muted-foreground) opacity-40"></span>
		</div>
		<div class="flex min-w-0 flex-1 flex-col gap-1 bg-(--background) p-1.5">
			<span class="h-1.5 w-1/2 self-end rounded-[2px] bg-(--user-bubble)"></span>
			<span class="h-1 w-5/6 rounded-[2px] bg-(--foreground) opacity-70"></span>
			<div class="flex gap-0.5 rounded-[3px] bg-(--code-bg) p-1">
				<span class="h-1 w-3 rounded-[2px] bg-(--syn-keyword)"></span>
				<span class="h-1 w-4 rounded-[2px] bg-(--syn-function)"></span>
				<span class="h-1 w-5 rounded-[2px] bg-(--syn-string)"></span>
				<span class="h-1 w-2 rounded-[2px] bg-(--syn-number)"></span>
			</div>
			<div class="mt-auto flex gap-0.5">
				{#each [1, 2, 3, 4, 5, 6] as i (i)}<span class="size-1.5 rounded-full" style:background="var(--ansi-{i})"></span>{/each}
			</div>
		</div>
	</div>
{:else}
	<div class="border-border flex h-36 shrink-0 overflow-hidden rounded-lg border text-[11px] leading-snug sm:h-44" {style} aria-hidden="true">
		<div class="flex w-[26%] flex-col gap-1.5 border-e border-(--sidebar-border) bg-(--sidebar) p-2 text-(--sidebar-foreground)">
			<span class="rounded-[4px] bg-(--primary) px-1.5 py-0.5 font-medium text-(--primary-foreground)">New chat</span>
			<span class="text-(--muted-foreground)">Projects</span>
			<span class="truncate rounded-[4px] bg-(--sidebar-accent) px-1">Fix the build</span>
			<span class="truncate px-1">رفع مشکل جهت</span>
		</div>
		<div class="flex min-w-0 flex-1 flex-col gap-1.5 bg-(--background) p-2 text-(--foreground)">
			<span class="self-end rounded-[6px] bg-(--user-bubble) px-1.5 py-0.5">Why does the test fail?</span>
			<span>The total skips the last item. <span class="text-(--link) underline">sum.ts:12</span></span>
			<pre class="rounded-[4px] bg-(--code-bg) px-1.5 py-1 font-mono text-[10.5px]"><span class="text-(--syn-keyword)">const</span> total = <span class="text-(--syn-function)">sum</span>(items, <span class="text-(--syn-string)">"ok"</span>) <span class="text-(--syn-comment) italic">// {42}</span></pre>
			<div class="flex flex-wrap gap-1">
				<span class="rounded-full bg-(--ok)/15 px-1.5 text-(--ok)">passed</span>
				<span class="rounded-full bg-(--attention)/15 px-1.5 text-(--attention)">waiting</span>
				<span class="rounded-full bg-(--destructive)/15 px-1.5 text-(--destructive)">failed</span>
			</div>
			<pre class="mt-auto rounded-[4px] bg-(--term-bg) px-1.5 py-1 font-mono text-[10.5px]"><span style:color="var(--ansi-2)">$</span> bun test <span style:color="var(--ansi-8)">--watch</span>  <span style:color="var(--ansi-2)">12 pass</span> <span style:color="var(--ansi-1)">1 fail</span> <span style:color="var(--ansi-4)">in 2s</span></pre>
		</div>
	</div>
{/if}
