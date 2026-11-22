<script lang="ts">
	import type { Item } from "$lib/types";
	import { Button } from "$lib/components/ui/button";
	import DiffView from "./DiffView.svelte";
	import ShieldQuestion from "@lucide/svelte/icons/shield-question";

	type Perm = Extract<Item, { kind: "permission" }>;
	let {
		item,
		providerLabel,
		onDecide,
	}: { item: Perm; providerLabel: string; onDecide: (item: Perm, optionId: string, message?: string) => void } = $props();

	let denyNote = $state("");
	let showNote = $state(false);

	const command = $derived.by(() => {
		const inp = item.input as Record<string, unknown> | null;
		if (inp && typeof inp === "object" && typeof inp.command === "string") return inp.command;
		return null;
	});

	const pending = $derived(item.status === "pending");
	const verdict = $derived(
		item.status === "done" ? "Allowed" : item.status === "declined" ? "Denied" : item.status === "interrupted" ? "Cancelled" : "",
	);
</script>

<div
	class="rounded-lg border text-[13px] {pending
		? 'border-attention/70 bg-attention/8'
		: 'border-border bg-card/40'}"
	role="group"
	aria-label="Permission request"
>
	<div class="flex items-center gap-2 px-3 pt-2.5 pb-1.5">
		<ShieldQuestion class="size-4 shrink-0 {pending ? 'text-attention' : 'text-muted-foreground'}" />
		<span class="font-medium">{providerLabel} wants to use <span class="font-mono">{item.tool}</span></span>
		{#if verdict}<span class="text-muted-foreground ms-auto text-xs">{verdict}</span>{/if}
	</div>
	<div class="flex flex-col gap-2 px-3 pb-2.5">
		{#if item.title && item.title !== command}
			<div class="text-muted-foreground truncate font-mono text-[12px]" dir="ltr">{item.title}</div>
		{/if}
		{#if item.reason}<div class="text-muted-foreground" dir="auto">{item.reason}</div>{/if}
		{#if command}
			<pre class="bg-code border-border selectable scrollbar-thin max-h-48 overflow-auto rounded-md border px-2.5 py-2 font-mono text-[12px] whitespace-pre-wrap" dir="ltr">{command}</pre>
		{/if}
		{#if item.diffs?.length}
			<DiffView diffs={item.diffs} maxHeight="18rem" />
		{/if}
		{#if pending}
			{#if showNote}
				<input
					class="border-input bg-background h-8 rounded-md border px-2.5"
					dir="auto"
					placeholder="Tell the agent what to do instead (optional)"
					bind:value={denyNote}
					onkeydown={(e) => {
						if (e.key === "Enter") onDecide(item, "deny", denyNote || undefined);
					}}
				/>
			{/if}
			<div class="flex flex-wrap items-center gap-1.5 pt-0.5">
				{#each item.options.filter((o) => o.kind === "allow") as o (o.id)}
					<Button size="sm" variant={o.id === item.options[0]?.id ? "default" : "secondary"} onclick={() => onDecide(item, o.id)}
						>{o.label}</Button
					>
				{/each}
				{#each item.options.filter((o) => o.kind !== "allow") as o (o.id)}
					<Button
						size="sm"
						variant="ghost"
						onclick={() => {
							if (o.id === "deny" && !showNote) showNote = true;
							else onDecide(item, o.id, denyNote || undefined);
						}}>{o.id === "deny" && showNote ? "Deny with note" : o.label}</Button
					>
				{/each}
			</div>
		{/if}
	</div>
</div>
