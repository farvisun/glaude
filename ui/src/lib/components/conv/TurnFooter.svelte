<script lang="ts">
	// Under each finished turn: how long it took, what it cost, the tokens and
	// files, and actions (copy, retry, fork, review by the other tool).
	import type { TurnStat } from "$lib/state/conversations.svelte";
	import type { Provider } from "$lib/types";
	import { fmtCost, fmtTokens } from "$lib/format";
	import { fmtElapsed } from "$lib/state/activity";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import Copy from "@lucide/svelte/icons/copy";
	import Check from "@lucide/svelte/icons/check";
	import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
	import GitFork from "@lucide/svelte/icons/git-fork";
	import ScanEye from "@lucide/svelte/icons/scan-eye";
	import FileDiff from "@lucide/svelte/icons/file-diff";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	let {
		stat,
		last,
		provider,
		onCopy,
		onRetry,
		onFork,
		onReview,
		onFiles,
	}: {
		stat?: TurnStat;
		last: boolean;
		provider: Provider;
		onCopy: () => void;
		onRetry?: () => void;
		onFork?: () => void;
		onReview?: () => void;
		onFiles?: () => void;
	} = $props();

	let copied = $state(false);
	const other = $derived(provider === "claude" ? "Codex" : "Claude");
	const tokens = $derived(stat?.tokens ? stat.tokens.input + stat.tokens.output : null);
</script>

{#snippet action(label: string, Icon: typeof Copy, run: () => void)}
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<button {...props} class="hover:bg-accent hover:text-foreground grid size-6 place-items-center rounded-md" aria-label={label} onclick={run}>
					<Icon class="size-3.5" />
				</button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content>{label}</Tooltip.Content>
	</Tooltip.Root>
{/snippet}

<div class="turn-footer text-muted-foreground -mt-1 flex min-h-6 flex-wrap items-center gap-x-3 gap-y-1 text-[12px] {last ? 'is-last' : ''}">
	{#if stat}
		<span class="flex items-center gap-x-2.5">
			{#if stat.error}<span class="text-destructive flex items-center gap-1"><TriangleAlert class="size-3" /> Stopped with an error</span>{/if}
			{#if stat.durationMs}<span title="Time this turn took">{fmtElapsed(stat.durationMs)}</span>{/if}
			{#if stat.costUsd}<span title="Cost of this turn (estimated by the tool)">{fmtCost(stat.costUsd)}</span>{/if}
			{#if tokens}
				<span title="Input {fmtTokens(stat.tokens?.input)} · cached {fmtTokens(stat.tokens?.cached)} · output {fmtTokens(stat.tokens?.output)}"
					>{fmtTokens(tokens)} tokens</span
				>
			{/if}
			{#if stat.files.length}
				<button class="hover:text-foreground flex items-center gap-1 underline-offset-2 hover:underline" onclick={onFiles} title={stat.files.join("\n")}>
					<FileDiff class="size-3" />{stat.files.length} {stat.files.length === 1 ? "file" : "files"} changed
				</button>
			{/if}
		</span>
	{/if}
	<span class="actions flex items-center gap-0.5">
		{@render action(copied ? "Copied" : "Copy reply", copied ? Check : Copy, () => {
			onCopy();
			copied = true;
			setTimeout(() => (copied = false), 1200);
		})}
		{#if onRetry}{@render action("Retry this turn", RotateCcw, onRetry)}{/if}
		{#if onFork}{@render action("Fork from here", GitFork, onFork)}{/if}
		{#if onReview}{@render action(`Ask ${other} to review the changes`, ScanEye, onReview)}{/if}
	</span>
</div>

<style>
	.actions {
		opacity: 0;
		transition: opacity 120ms;
	}
	.is-last .actions,
	:global(.cv-row:hover) .actions,
	.actions:focus-within {
		opacity: 1;
	}
	@media (hover: none) {
		.actions {
			opacity: 1;
		}
	}
</style>
