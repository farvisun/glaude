<script lang="ts">
	// Claude asks (AskUserQuestion): one or more questions with options,
	// single or multi choice, an "Other" answer and optional previews.
	import type { Item } from "$lib/types";
	import { Button } from "$lib/components/ui/button";
	import { narrow } from "$lib/state/media.svelte";
	import { detectDir } from "$lib/bidi";
	import MessageCircleQuestion from "@lucide/svelte/icons/message-circle-question-mark";
	import Check from "@lucide/svelte/icons/check";

	type Q = Extract<Item, { kind: "question" }>;
	let {
		item,
		providerLabel,
		onAnswer,
		onSkip,
	}: { item: Q; providerLabel: string; onAnswer: (answers: Record<string, string>) => void; onSkip: () => void } = $props();

	const OTHER = "\u0000other";
	// Per question: chosen labels (and OTHER), other text, focused option for the preview.
	let chosen = $state<string[][]>([]);
	let other = $state<string[]>([]);
	let focus = $state<(string | null)[]>([]);
	$effect(() => {
		if (chosen.length !== item.questions.length) {
			chosen = item.questions.map(() => []);
			other = item.questions.map(() => "");
			focus = item.questions.map(() => null);
		}
	});

	const pending = $derived(item.status === "pending");

	function toggle(qi: number, label: string) {
		const q = item.questions[qi];
		const cur = chosen[qi] ?? [];
		chosen[qi] = q.multiSelect ? (cur.includes(label) ? cur.filter((l) => l !== label) : [...cur, label]) : [label];
		focus[qi] = label === OTHER ? null : label;
	}

	function answerOf(qi: number): string {
		const picks = (chosen[qi] ?? []).map((l) => (l === OTHER ? other[qi]?.trim() ?? "" : l)).filter(Boolean);
		return picks.join(", ");
	}
	const complete = $derived(item.questions.every((_, i) => answerOf(i).length > 0));

	function submit() {
		if (!complete) return;
		const answers: Record<string, string> = {};
		item.questions.forEach((q, i) => (answers[q.question] = answerOf(i)));
		onAnswer(answers);
	}

	const previewOf = (qi: number) => {
		const q = item.questions[qi];
		const label = focus[qi] ?? chosen[qi]?.find((l) => l !== OTHER) ?? null;
		return q.options.find((o) => o.label === label)?.preview ?? null;
	};
	const otherOn = (qi: number) => chosen[qi]?.includes(OTHER) ?? false;
	const hasPreviews = (qi: number) => item.questions[qi].options.some((o) => o.preview);
</script>

<div
	class="rounded-lg border text-[13px] {pending ? 'border-attention/70 bg-attention/8' : 'border-border bg-card/40'}"
	role="group"
	aria-label="Question from {providerLabel}"
>
	<div class="flex items-center gap-2 px-3 pt-2.5 pb-1.5">
		<MessageCircleQuestion class="size-4 shrink-0 {pending ? 'text-attention' : 'text-muted-foreground'}" />
		<span class="font-medium">{providerLabel} has {item.questions.length > 1 ? `${item.questions.length} questions` : "a question"}</span>
		{#if !pending}
			<span class="text-muted-foreground ms-auto text-xs">{item.status === "done" ? "Answered" : item.status === "declined" ? "Skipped" : "Cancelled"}</span>
		{/if}
	</div>

	{#if pending}
		<div class="flex flex-col gap-4 px-3 pb-3">
			{#each item.questions as q, qi (q.question)}
				<fieldset class="flex flex-col gap-2" dir={detectDir(q.question)}>
					<legend class="flex flex-wrap items-center gap-2 pb-1.5">
						{#if q.header}<span class="bg-muted rounded px-1.5 py-px text-[11px] font-medium">{q.header}</span>{/if}
						<span class="font-medium" dir="auto">{q.question}</span>
						{#if q.multiSelect}<span class="text-muted-foreground text-xs">Choose any</span>{/if}
					</legend>
					<div class="flex gap-3 {hasPreviews(qi) && !narrow.current ? 'flex-row' : 'flex-col'}">
						<div class="flex min-w-0 flex-1 flex-col gap-1.5" role={q.multiSelect ? "group" : "radiogroup"}>
							{#each q.options as o, oi (o.label)}
								{@const on = chosen[qi]?.includes(o.label)}
								<button
									class="flex items-start gap-2.5 rounded-md border px-2.5 py-2 text-start transition-colors {on
										? 'border-primary bg-primary/10'
										: 'border-border hover:bg-accent/50'}"
									role={q.multiSelect ? "checkbox" : "radio"}
									aria-checked={on}
									onclick={() => toggle(qi, o.label)}
									onfocus={() => (focus[qi] = o.label)}
									onmouseenter={() => (focus[qi] = o.label)}
								>
									<span
										class="mt-0.5 grid size-4 shrink-0 place-items-center border {q.multiSelect ? 'rounded' : 'rounded-full'} {on
											? 'border-primary bg-primary text-primary-foreground'
											: 'border-muted-foreground/50'}"
									>
										{#if on}<Check class="size-3" />{/if}
									</span>
									<span class="min-w-0">
										<span class="block font-medium" dir="auto"><bdi class="text-muted-foreground me-1 text-xs">{oi + 1}.</bdi>{o.label}</span>
										{#if o.description}<span class="text-muted-foreground block text-xs" dir="auto">{o.description}</span>{/if}
									</span>
								</button>
							{/each}
							<div class="flex items-center gap-2.5 rounded-md border px-2.5 py-1.5 {otherOn(qi) ? 'border-primary bg-primary/10' : 'border-border'}">
								<button
									class="grid size-4 shrink-0 place-items-center border {q.multiSelect ? 'rounded' : 'rounded-full'} {otherOn(qi)
										? 'border-primary bg-primary text-primary-foreground'
										: 'border-muted-foreground/50'}"
									role={q.multiSelect ? "checkbox" : "radio"}
									aria-checked={otherOn(qi)}
									aria-label="Other answer"
									onclick={() => toggle(qi, OTHER)}
								>
									{#if otherOn(qi)}<Check class="size-3" />{/if}
								</button>
								<input
									class="placeholder:text-muted-foreground h-7 min-w-0 flex-1 bg-transparent outline-none"
									dir="auto"
									placeholder="Other…"
									bind:value={other[qi]}
									oninput={() => {
										if (!chosen[qi]?.includes(OTHER)) toggle(qi, OTHER);
									}}
									onkeydown={(e) => e.key === "Enter" && submit()}
								/>
							</div>
						</div>
						{#if hasPreviews(qi)}
							<pre
								class="bg-code border-border selectable scrollbar-thin min-h-24 overflow-auto rounded-md border px-3 py-2 font-mono text-[12px] leading-snug whitespace-pre {narrow.current
									? 'max-h-60'
									: 'max-h-80 w-[46%] shrink-0'}"
								dir="ltr">{previewOf(qi) ?? "Hover an option to preview it."}</pre>
						{/if}
					</div>
				</fieldset>
			{/each}
			<div class="flex items-center gap-1.5">
				<Button size="sm" disabled={!complete} onclick={submit}>Submit answer{item.questions.length > 1 ? "s" : ""}</Button>
				<Button size="sm" variant="ghost" onclick={onSkip}>Skip</Button>
			</div>
		</div>
	{:else if item.answers && Object.keys(item.answers).length}
		<dl class="flex flex-col gap-1 px-3 pb-2.5">
			{#each Object.entries(item.answers) as [q, a] (q)}
				<div class="flex flex-wrap gap-x-2">
					<dt class="text-muted-foreground" dir="auto">{q}</dt>
					<dd class="font-medium" dir="auto">{a}</dd>
				</div>
			{/each}
		</dl>
	{:else}
		<div class="text-muted-foreground px-3 pb-2.5" dir="auto">{item.questions.map((q) => q.question).join(" · ")}</div>
	{/if}
</div>
