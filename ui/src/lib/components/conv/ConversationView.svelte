<script lang="ts">
	import { conversations } from "$lib/state/conversations.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import { convActions, loadHistory, openLink, permissionModes } from "$lib/state/sessions";
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
	import QueuePanel from "./QueuePanel.svelte";
	import LimitBanner from "./LimitBanner.svelte";
	import TerminalDrawer from "./TerminalDrawer.svelte";
	import { Button } from "$lib/components/ui/button";
	import MessagesSquare from "@lucide/svelte/icons/messages-square";
	import Quote from "@lucide/svelte/icons/quote";
	import { hitOf, markPaths } from "$lib/state/paths";
	import { pathActions } from "$lib/state/open";

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
	let composer = $state<{ focus: () => void; setText: (t: string) => void; insertQuote: (t: string) => void }>();

	// Earlier prompts for ArrowUp, oldest first (built only when asked for).
	function promptHistory(): string[] {
		if (!conv) return [];
		const out: string[] = [];
		const div = document.createElement("div");
		for (const it of conv.items) {
			if (it.kind !== "user") continue;
			const text = it.blocks
				.map((b) => {
					div.innerHTML = b.html;
					return div.textContent ?? "";
				})
				.join("\n\n")
				.trim();
			if (text && out[out.length - 1] !== text) out.push(text);
		}
		return out;
	}

	// Selecting text in a reply offers "Quote".
	let quoteAt = $state<{ x: number; y: number; text: string } | null>(null);
	function onSelectEnd() {
		const sel = window.getSelection();
		const text = sel?.toString().trim() ?? "";
		if (!sel || !text || !scroller || sel.rangeCount === 0) {
			quoteAt = null;
			return;
		}
		const range = sel.getRangeAt(0);
		const node = range.commonAncestorContainer;
		const el = node instanceof Element ? node : node.parentElement;
		if (!el || !scroller.contains(el) || el.closest("textarea, input")) {
			quoteAt = null;
			return;
		}
		const r = range.getBoundingClientRect();
		const box = scroller.getBoundingClientRect();
		quoteAt = { x: Math.min(Math.max(r.left + r.width / 2 - box.left, 40), box.width - 40), y: r.top - box.top + scroller.scrollTop, text };
	}

	// A side chat (or anything else) asked to quote into this composer.
	$effect(() => {
		if (conv?.quote != null && composer) {
			composer.insertQuote(conv.quote);
			conv.quote = null;
		}
	});

	function onClick(e: MouseEvent) {
		const pathEl = (e.target as HTMLElement).closest("[data-path]") as HTMLElement | null;
		const hit = pathEl ? hitOf(pathEl) : null;
		if (hit) {
			e.preventDefault();
			if (e.metaKey || e.ctrlKey) void pathActions.primary(hit);
			else ui.pathMenu = { hit, x: e.clientX, y: e.clientY };
			return;
		}
		const a = (e.target as HTMLElement).closest("a[href]") as HTMLAnchorElement | null;
		if (!a) return;
		e.preventDefault();
		openLink(a.getAttribute("href") ?? "");
	}

	function onScroll() {
		if (!scroller) return;
		atBottom = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 48;
		if (scroller.scrollTop < 1200) void showEarlier();
	}


	let content = $state<HTMLDivElement>();

	// Keep the view pinned to the newest message while the user is at the
	// bottom; rows change height as they render (code, images), so a
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

	// Paths in replies that exist become links (checked in batches).
	$effect(() => {
		if (!content || !conv) return;
		const root = content;
		const cwd = conv.cwd;
		let t: ReturnType<typeof setTimeout> | undefined;
		const scan = () => {
			clearTimeout(t);
			t = setTimeout(() => void markPaths(root, cwd), 250);
		};
		scan();
		const mo = new MutationObserver(scan);
		mo.observe(root, { childList: true, subtree: true });
		return () => {
			mo.disconnect();
			clearTimeout(t);
		};
	});

	// Ctrl+` shows or hides the terminal drawer (also from inside it).
	function onWindowKey(e: KeyboardEvent) {
		if (!active || !conv || !e.ctrlKey || e.metaKey || e.altKey || (e.key !== "`" && e.code !== "Backquote")) return;
		e.preventDefault();
		e.stopPropagation();
		conv.drawer = !conv.drawer;
		if (!conv.drawer) tick().then(() => composer?.focus());
	}

	// Seen: clear the "finished while away" dot.
	$effect(() => {
		if (active && conv?.unread) conv.markUnread(false);
	});

	// A tab restored after a restart resumes its session once it is shown.
	$effect(() => {
		if (active && conv?.needsHistory) void loadHistory(conv);
	});

	const providerLabel = $derived(conv?.provider === "codex" ? "Codex" : "Claude");
	const modes = $derived(conv ? permissionModes(conv.provider) : []);

	function decide(item: { requestId: string }, optionId: string, message?: string) {
		if (conv) convActions.decide(conv, item, optionId, message);
	}

	const view = $derived(groupItems(conv?.items ?? []));

	// Only the newest rows are rendered at first; older ones are added in
	// chunks while scrolling up, keeping what is on screen in place (WebKit
	// has no scroll anchoring, and placeholder heights made content jump).
	const FIRST_ROWS = 60;
	const MORE_ROWS = 40;
	let firstId = $state<string | null>(null);
	let growing = false;
	const startIdx = $derived.by(() => {
		const main = view.main;
		const i = firstId ? main.findIndex((x) => x.id === firstId) : -1;
		return i >= 0 ? i : Math.max(0, main.length - FIRST_ROWS);
	});
	const shown = $derived(startIdx > 0 ? view.main.slice(startIdx) : view.main);
	// Pin the window's first row so rows streaming in at the bottom never
	// push rows out at the top (a new history load starts over).
	$effect(() => {
		const main = view.main;
		if (!main.length) {
			firstId = null;
			return;
		}
		if (!firstId || !main.some((x) => x.id === firstId)) firstId = main[Math.max(0, main.length - FIRST_ROWS)].id;
	});

	async function showEarlier() {
		if (growing || !scroller || startIdx === 0) return;
		growing = true;
		const s = scroller;
		const before = s.scrollHeight - s.scrollTop;
		firstId = view.main[Math.max(0, startIdx - MORE_ROWS)].id;
		await tick();
		// Same distance from the bottom as before: the visible rows stay put.
		s.scrollTop = s.scrollHeight - before;
		growing = false;
		// Still near the top (fast scrolling, short rows): add another chunk.
		if (s.scrollTop < 1200 && startIdx > 0) requestAnimationFrame(() => void showEarlier());
	}

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

<svelte:window onkeydowncapture={onWindowKey} />

{#if conv}
	<div class="@container flex h-full min-h-0">
		<div class="flex min-w-0 flex-1 flex-col">
			{#if conv.sideOf}
				<div class="border-border bg-card/60 flex items-center gap-2 border-b px-4 py-1.5 text-xs">
					<MessagesSquare class="text-muted-foreground size-3.5 shrink-0" />
					<span class="text-muted-foreground min-w-0 flex-1 truncate"
						>Side chat · knows <span class="text-foreground" dir="auto">{conversations.get(conv.sideOf)?.title ?? "the original conversation"}</span> up to when it
						started</span
					>
					<Button variant="outline" size="sm" class="h-6 text-xs" onclick={() => convActions.addToParent(conv)}>Add to chat</Button>
				</div>
			{/if}
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
			<div
				class="scrollbar-thin relative min-h-0 flex-1 overflow-y-auto"
				bind:this={scroller}
				onscroll={onScroll}
				onclick={onClick}
				onmouseup={() => setTimeout(onSelectEnd, 0)}
				onkeyup={(e) => e.shiftKey && onSelectEnd()}
			>
				{#if quoteAt}
					<button
						class="bg-popover text-popover-foreground border-border hover:bg-accent absolute z-20 flex -translate-x-1/2 -translate-y-full items-center gap-1 rounded-md border px-2 py-1 text-xs shadow-md"
						style="left: {quoteAt.x}px; top: {quoteAt.y - 6}px"
						onmousedown={(e) => e.preventDefault()}
						onclick={(e) => {
							e.stopPropagation();
							if (quoteAt) composer?.insertQuote(quoteAt.text);
							quoteAt = null;
							window.getSelection()?.removeAllRanges();
						}}><Quote class="size-3" /> Quote</button
					>
				{/if}
				<div class="mx-auto flex max-w-(--msg-w) flex-col gap-(--row-gap) px-5 pt-6 pb-8" bind:this={content}>
					{#if conv.loading || conv.needsHistory}
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
					{#if startIdx > 0}
						<button class="text-muted-foreground hover:text-foreground self-center text-xs" onclick={() => void showEarlier()}
							>{startIdx} earlier {startIdx === 1 ? "message" : "messages"}</button
						>
					{/if}
					{#each shown as item (item.id)}
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
			<div class="mx-auto w-full max-w-(--msg-w) px-4 pb-1">
				<LimitBanner limitedUntil={conv.limitedUntil} resumeAt={conv.resumeAt} onSchedule={(at) => convActions.schedule(conv, at)} />
				<QueuePanel
					items={conv.queue}
					onChange={(items) => convActions.setQueue(conv, items)}
					onSendNow={(m) => {
						void convActions.setQueue(
							conv,
							conv.queue.filter((x) => x.id !== m.id),
						);
						void convActions.steer(conv, m.text, m.attachments ?? []);
					}}
				/>
				<Composer
					bind:this={composer}
					running={conv.running}
					slashCommands={["btw", ...(conv.session.slashCommands ?? []).filter((c) => c !== "btw")]}
					placeholder={`Message ${providerLabel}`}
					onSend={(text, attachments) =>
						/^\/btw\b/.test(text) || !(conv.running || conv.queue.length)
							? convActions.send(conv, text, attachments)
							: convActions.enqueue(conv, text, attachments)}
					onSteer={(text, attachments) => convActions.steer(conv, text, attachments)}
					getHistory={promptHistory}
					onStop={() => convActions.interrupt(conv)}
					onAttach={() => convActions.pickFiles()}
					onPasteImage={(blob, name) => convActions.saveImage(blob, name)}
					agents={conv.session.agents ?? []}
					onSuggestFiles={conv.provider === "claude" ? suggestFiles : undefined}
					onCycleMode={modes.length ? cycleMode : undefined}
					{modeHint}
				/>
			</div>
			{#if conv.drawer}
				<TerminalDrawer id="drawer:{conv.id}" cwd={conv.cwd} onClose={() => (conv.drawer = false)} />
			{/if}
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
	/* No content-visibility on rows: WebKit has no scroll anchoring, so
	   placeholder heights made the content jump while scrolling up. Older
	   rows are added in chunks instead (showEarlier). */
	.cv-row {
		min-width: 0;
	}
</style>
