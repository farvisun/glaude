<script lang="ts">
	// Accent on top of any theme: the theme's own, a preset, or any colour.
	import { ACCENTS } from "$lib/themes/presets";
	import { norm, isColor } from "$lib/themes/color";
	import { themes } from "$lib/themes/themes.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import * as Popover from "$lib/components/ui/popover";
	import { Input } from "$lib/components/ui/input";
	import Check from "@lucide/svelte/icons/check";
	import Plus from "@lucide/svelte/icons/plus";

	const kind = $derived(themes.app.kind);
	const custom = $derived(prefs.accent.startsWith("#") ? prefs.accent : "");
	let hex = $state("");
	$effect(() => {
		hex = custom || themes.app.accent;
	});
	function setHex(v: string) {
		if (isColor(v)) prefs.set("accent", norm(v));
	}
</script>

<div class="flex flex-wrap items-center gap-1.5" role="radiogroup" aria-label="Accent colour">
	<button
		class="border-border relative grid h-7 place-items-center rounded-full border px-2.5 text-[12px] {prefs.accent === 'theme' ? 'ring-foreground/40 ring-2' : 'hover:bg-accent'}"
		role="radio"
		aria-checked={prefs.accent === "theme"}
		onclick={() => prefs.set("accent", "theme")}
		title="The theme's own accent">Theme's</button
	>
	{#each ACCENTS as a (a.id)}
		<button
			class="border-border relative grid size-7 place-items-center rounded-full border {prefs.accent === a.id ? 'ring-foreground/40 ring-2 ring-offset-1 ring-offset-(--background)' : ''}"
			style:background={a[kind]}
			role="radio"
			aria-checked={prefs.accent === a.id}
			aria-label={a.name}
			title={a.name}
			onclick={() => prefs.set("accent", a.id)}
		>
			{#if prefs.accent === a.id}<Check class="size-3.5" style="color: {kind === 'dark' ? '#0b0c0f' : '#ffffff'}" />{/if}
		</button>
	{/each}
	<Popover.Root>
		<Popover.Trigger
			class="border-border relative grid size-7 place-items-center rounded-full border {custom ? 'ring-foreground/40 ring-2 ring-offset-1 ring-offset-(--background)' : 'hover:bg-accent'}"
			style={custom ? `background: ${custom}` : undefined}
			aria-label="Other colour"
			title="Other colour">{#if !custom}<Plus class="size-3.5" />{/if}</Popover.Trigger
		>
		<Popover.Content class="flex w-56 flex-col gap-2 p-3" align="end">
			<span class="text-[12px] font-medium">Other colour</span>
			<div class="flex items-center gap-2">
				<label class="border-border relative size-8 shrink-0 cursor-pointer overflow-hidden rounded-md border" style:background={isColor(hex) ? norm(hex) : "transparent"}>
					<input type="color" class="absolute inset-0 cursor-pointer opacity-0" value={norm(hex)} oninput={(e) => ((hex = e.currentTarget.value), setHex(hex))} aria-label="Pick a colour" />
				</label>
				<Input
					class="h-8 font-mono text-[12px]"
					bind:value={hex}
					onchange={() => setHex(hex)}
					onkeydown={(e) => e.key === "Enter" && setHex(hex)}
					aria-label="Hex colour"
					placeholder="#3e7bfa"
				/>
			</div>
		</Popover.Content>
	</Popover.Root>
</div>
