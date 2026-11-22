<script lang="ts">
	import { conversations } from "$lib/state/conversations.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import { convActions, permissionModes } from "$lib/state/sessions";
	import type { Item } from "$lib/types";
	import ItemView from "./ItemView.svelte";
	import Composer from "./Composer.svelte";
	import StatusLine from "./StatusLine.svelte";
	import ProgressPanel from "./ProgressPanel.svelte";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";
	import { tick } from "svelte";

	let { convId, active }: { convId: string; active: boolean } = $props();

	const conv = $derived(conversations.get(convId));
	let scroller = $state<HTMLDivElement>();
	let atBottom = $state(true);
	let composer = $state<{ focus: () => void }>();

	function onScroll() {
		if (!scroller) return;
		atBottom = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 48;
	}

	let content = $state<HTMLDivElement>();

	// Keep the view pinned to the newest message while the user is at the
	// bottom; content-visibility rows change height as they render, so a
	// one-off scroll after an update is not enough.
	$effect(() => {
		if (!content || !scroller) return;
		const s = scroller;
		const ro = new ResizeObserver(() => {
			if (atBottom) s.scrollTop = s.scrollHeight;
		});
		ro.observe(content);
		return () => ro.disconnect();
	});

	$effect(() => {
		if (active) tick().then(() => composer?.focus());
	});

	const providerLabel = $derived(conv?.provider === "codex" ? "Codex" : "Claude");
	const modes = $derived(conv ? permissionModes(conv.provider) : []);

	type Perm = Extract<Item, { kind: "permission" }>;
	function decide(item: Perm, optionId: string, message?: string) {
		if (conv) convActions.decide(conv, item, optionId, message);
	}
</script>

{#if conv}
	<div class="flex h-full min-h-0">
		<div class="flex min-w-0 flex-1 flex-col">
			<div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto" bind:this={scroller} onscroll={onScroll}>
				<div class="mx-auto flex max-w-[52rem] flex-col gap-4 px-5 pt-6 pb-8" bind:this={content}>
					{#if conv.loading}
						<div class="flex flex-col gap-3">
							{#each [0, 1, 2] as i (i)}
								<div class="bg-muted h-16 animate-pulse rounded-lg"></div>
							{/each}
						</div>
					{:else if conv.items.length === 0}
						<div class="text-muted-foreground py-16 text-center text-[13px]">
							Ask {providerLabel} anything about <span class="text-foreground font-mono" dir="ltr">{conv.cwd}</span>.
						</div>
					{/if}
					{#each conv.items as item (item.id)}
						<div class="cv-row">
							<ItemView {item} {providerLabel} onDecide={decide} />
						</div>
					{/each}
					{#if conv.turn !== "idle" && conv.turn !== "waitingPermission"}
						<div class="text-muted-foreground flex items-center gap-2 text-[13px]">
							<LoaderCircle class="text-primary size-3.5 animate-spin" />
							<span>{conv.turnDetail ?? "Working"}</span>
						</div>
					{/if}
				</div>
			</div>
			<div class="mx-auto w-full max-w-[52rem] px-4 pb-1">
				<Composer
					bind:this={composer}
					running={conv.running}
					slashCommands={conv.session.slashCommands ?? []}
					placeholder={`Message ${providerLabel}`}
					onSend={(text, attachments) => convActions.send(conv, text, attachments)}
					onStop={() => convActions.interrupt(conv)}
					onAttach={() => convActions.pickFiles()}
					onPasteImage={(blob, name) => convActions.saveImage(blob, name)}
				/>
			</div>
			<StatusLine
				{conv}
				{modes}
				onMode={(m) => convActions.setMode(conv, m)}
				onModel={(m) => convActions.setModel(conv, m)}
			/>
		</div>
		{#if prefs.progressPanel}
			<div class="hidden w-64 shrink-0 md:block">
				<ProgressPanel {conv} />
			</div>
		{/if}
	</div>
{:else}
	<div class="text-muted-foreground grid h-full place-items-center text-[13px]">This conversation is no longer open.</div>
{/if}

<style>
	.cv-row {
		content-visibility: auto;
		contain-intrinsic-size: auto 80px;
	}
</style>
