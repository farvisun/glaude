<script lang="ts">
	import { conversations } from "$lib/state/conversations.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import { convActions, openLink, permissionModes } from "$lib/state/sessions";
	import type { Item } from "$lib/types";
	import ItemView from "./ItemView.svelte";
	import Composer from "./Composer.svelte";
	import StatusLine from "./StatusLine.svelte";
	import ProgressPanel from "./ProgressPanel.svelte";
	import TurnIndicator from "./TurnIndicator.svelte";
	import { tick } from "svelte";
	import { call } from "$lib/ipc";
	import { ui } from "$lib/state/ui.svelte";
	import { groupItems } from "$lib/state/group";
	import Spark from "./Spark.svelte";

	let { convId, active }: { convId: string; active: boolean } = $props();

	const conv = $derived(conversations.get(convId));

	// Optional: the user's own Claude statusline script, re-run (debounced)
	// whenever the session or limits change.
	let scriptLine = $state<string | null>(null);
	$effect(() => {
		if (!conv || conv.provider !== "claude" || !prefs.statuslineScript) {
			scriptLine = null;
			return;
		}
		const s = conv.session;
		const input = {
			account: conv.account || "default",
			cwd: conv.cwd,
			sessionId: s.sessionId ?? null,
			model: s.model ?? null,
			effort: s.effort ?? null,
			version: s.version ?? null,
			contextUsed: s.contextUsed ?? null,
			contextWindow: s.contextWindow ?? null,
			costUsd: s.costUsd ?? null,
			linesAdded: s.linesAdded ?? null,
			linesRemoved: s.linesRemoved ?? null,
			fiveHour: conv.limits.fiveHour ?? null,
			week: conv.limits.week ?? null,
		};
		const t = setTimeout(() => {
			call<string | null>("statusline_render", { input })
				.then((html) => (scriptLine = html))
				.catch(() => (scriptLine = null));
		}, 300);
		return () => clearTimeout(t);
	});
	let scroller = $state<HTMLDivElement>();
	let atBottom = $state(true);
	let composer = $state<{ focus: () => void; setText: (t: string) => void }>();

	function onClick(e: MouseEvent) {
		const a = (e.target as HTMLElement).closest("a[href]") as HTMLAnchorElement | null;
		if (!a) return;
		e.preventDefault();
		openLink(a.getAttribute("href") ?? "");
	}

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

	function decide(item: { requestId: string }, optionId: string, message?: string) {
		if (conv) convActions.decide(conv, item, optionId, message);
	}

	const view = $derived(groupItems(conv?.items ?? []));

	const taskOf = (it: Item) => (it.kind === "tool" && conv ? conv.taskFor(it.id) : undefined);

	// Text put back after a rewind.
	$effect(() => {
		if (conv?.draft != null && composer) {
			composer.setText(conv.draft);
			conv.draft = null;
		}
	});

	const canRewind = $derived(conv?.provider === "claude" && !!conv.session.sessionId);

	// Shift+Tab walks the permission modes; bypass is only chosen on purpose.
	function cycleMode() {
		if (!conv) return;
		const list = modes.map((m) => m.value).filter((v) => v !== "bypassPermissions");
		const cur = conv.session.permissionMode ?? list[0];
		const next = list[(list.indexOf(cur) + 1) % list.length];
		conv.session = { ...conv.session, permissionMode: next };
		convActions.setMode(conv, next);
	}
	const modeHint = $derived.by(() => {
		if (conv?.provider !== "claude") return undefined;
		switch (conv.session.permissionMode) {
			case "plan":
				return "⏸ plan mode on (shift+tab to cycle)";
			case "acceptEdits":
				return "⏵⏵ accept edits on (shift+tab to cycle)";
			case "auto":
				return "⏵⏵ auto mode on (shift+tab to cycle)";
			case "bypassPermissions":
				return "⏵⏵ bypass permissions on (shift+tab to cycle)";
			default:
				return undefined;
		}
	});

	async function suggestFiles(query: string): Promise<string[]> {
		if (!conv || conv.provider !== "claude" || !conv.alive) return [];
		const r = await call<{ suggestions?: { path: string }[] }>("conv_control", {
			convId: conv.id,
			request: { subtype: "file_suggestions", query },
		});
		return (r.suggestions ?? []).map((s) => s.path);
	}

	const background = $derived(conv?.runningTasks ?? []);
	const backgroundLabel = $derived.by(() => {
		const agents = background.filter((t) => t.kind === "agent").length;
		const flows = background.filter((t) => t.kind === "workflow").length;
		const other = background.length - agents - flows;
		const part = (n: number, one: string, many: string) => (n ? `${n} ${n === 1 ? one : many}` : null);
		return [part(agents, "agent", "agents"), part(flows, "workflow", "workflows"), part(other, "shell", "shells")]
			.filter(Boolean)
			.join(", ");
	});
</script>

{#if conv}
	<div class="@container flex h-full min-h-0">
		<div class="flex min-w-0 flex-1 flex-col">
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
			<div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto" bind:this={scroller} onscroll={onScroll} onclick={onClick}>
				<div class="mx-auto flex max-w-[52rem] flex-col gap-4 px-5 pt-6 pb-8" bind:this={content}>
					{#if conv.loading}
						<div class="flex flex-col gap-3">
							{#each [0, 1, 2] as i (i)}
								<div class="skeleton-shimmer h-16 rounded-lg"></div>
							{/each}
						</div>
					{:else if conv.items.length === 0}
						<div class="text-muted-foreground py-16 text-center text-[13px]">
							Ask {providerLabel} anything about <span class="text-foreground font-mono" dir="ltr">{conv.cwd}</span>.
						</div>
					{/if}
					{#each view.main as item (item.id)}
						<div class="cv-row">
							<ItemView
								{item}
								{providerLabel}
								provider={conv.provider}
								thoughtFor={(id) => conv.thoughtFor(id)}
								onDecide={decide}
								task={taskOf(item)}
								steps={view.steps[item.id]}
								onStopTask={(id) => convActions.stopTask(conv, id)}
								onTaskOutput={(id) => (ui.taskOutput = { convId: conv.id, taskId: id })}
								onAnswer={(q, answers) => convActions.answerQuestion(conv, q, answers)}
								onRewind={canRewind ? (it) => (ui.rewind = { convId: conv.id, itemId: it.id }) : undefined}
							/>
						</div>
					{/each}
					{#if conv.turn !== "idle" && conv.turn !== "waitingPermission"}
						<TurnIndicator {conv} />
					{:else if background.length}
						<button
							class="text-muted-foreground hover:text-foreground flex items-center gap-1.5 self-start text-[13px]"
							onclick={() => {
								prefs.progressPanel = true;
								prefs.save();
							}}
						>
							<Spark color={conv.provider === "codex" ? "var(--codex)" : "var(--claude)"} class="text-[14px]" />
							{backgroundLabel} working in the background
						</button>
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
					agents={conv.session.agents ?? []}
					onSuggestFiles={conv.provider === "claude" ? suggestFiles : undefined}
					onCycleMode={modes.length ? cycleMode : undefined}
					{modeHint}
				/>
			</div>
			<StatusLine
				{conv}
				{modes}
				{scriptLine}
				onMode={(m) => convActions.setMode(conv, m)}
				onModel={(m) => convActions.setModel(conv, m)}
			/>
		</div>
		{#if prefs.progressPanel}
			<div class="hidden w-64 shrink-0 @3xl:block">
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
