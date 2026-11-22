<script lang="ts">
	// Output of a background shell or monitor, refreshed while it runs.
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Sheet from "$lib/components/ui/sheet";
	import { Button } from "$lib/components/ui/button";
	import { ui } from "$lib/state/ui.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { convActions } from "$lib/state/sessions";
	import { narrow } from "$lib/state/media.svelte";
	import { fmtDuration } from "$lib/format";
	import TaskStatus from "./TaskStatus.svelte";

	const conv = $derived(ui.taskOutput ? conversations.get(ui.taskOutput.convId) : undefined);
	const task = $derived(ui.taskOutput && conv ? conv.tasks[ui.taskOutput.taskId] : undefined);
	const running = $derived(task?.status === "running" || task?.status === "pending");

	let output = $state("");
	let error = $state<string | null>(null);
	let loading = $state(false);
	let pre = $state<HTMLPreElement | null>(null);
	let stick = true;

	async function load() {
		if (!conv || !task) return;
		try {
			const text = await convActions.taskOutput(conv, task.id, task.outputFile);
			error = null;
			if (text !== output) {
				stick = !pre || pre.scrollHeight - pre.scrollTop - pre.clientHeight < 24;
				output = text;
				queueMicrotask(() => stick && pre && (pre.scrollTop = pre.scrollHeight));
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (!ui.taskOutput) return;
		output = "";
		error = null;
		loading = true;
		void load();
	});
	$effect(() => {
		if (!ui.taskOutput || !running) return;
		const t = setInterval(load, 1000);
		return () => clearInterval(t);
	});

	const open = {
		get value() {
			return !!ui.taskOutput;
		},
		set value(v: boolean) {
			if (!v) ui.taskOutput = null;
		},
	};
	const title = $derived(task?.description || "Task output");
	const meta = $derived(
		task ? `${task.kind === "monitor" ? "Monitor" : "Background shell"} · ${task.status}${task.endedAt ? ` · ${fmtDuration(task.endedAt - task.startedAt)}` : ""}` : "",
	);
</script>

{#snippet body()}
	{#if loading && !output}
		<div class="skeleton-shimmer h-40 rounded-md"></div>
	{:else if error && !output}
		<p class="text-muted-foreground py-6 text-center text-[13px]">{error}</p>
	{:else}
		<pre
			bind:this={pre}
			class="bg-code border-border selectable scrollbar-thin max-h-[60vh] min-h-32 overflow-auto rounded-md border px-3 py-2 font-mono text-[12px] whitespace-pre-wrap"
			dir="ltr">{output || "No output yet."}</pre>
	{/if}
	<div class="flex items-center justify-end gap-1.5 pt-1">
		{#if running && conv && task}
			<Button size="sm" variant="secondary" onclick={() => convActions.stopTask(conv, task.id)}>Stop</Button>
		{/if}
		<Button size="sm" variant="ghost" onclick={() => navigator.clipboard?.writeText(output)} disabled={!output}>Copy</Button>
	</div>
{/snippet}

{#if narrow.current}
	<Sheet.Root bind:open={open.value}>
		<Sheet.Content side="bottom" class="max-h-[85vh] gap-2 px-4 pb-4">
			<Sheet.Header class="px-0">
				<Sheet.Title class="flex items-center gap-2" dir="auto">{#if task}<TaskStatus status={task.status} color="var(--muted-foreground)" />{/if}{title}</Sheet.Title>
				<Sheet.Description>{meta}</Sheet.Description>
			</Sheet.Header>
			{@render body()}
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Dialog.Root bind:open={open.value}>
		<Dialog.Content class="sm:max-w-3xl">
			<Dialog.Header>
				<Dialog.Title class="flex items-center gap-2" dir="auto">{#if task}<TaskStatus status={task.status} color="var(--muted-foreground)" />{/if}{title}</Dialog.Title>
				<Dialog.Description>{meta}</Dialog.Description>
			</Dialog.Header>
			{@render body()}
		</Dialog.Content>
	</Dialog.Root>
{/if}
