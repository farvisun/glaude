<script lang="ts">
	// Chat or terminal theme: follow the window, or any theme.
	import * as Select from "$lib/components/ui/select";
	import { themes, scopeLabel } from "$lib/themes/themes.svelte";
	import { FAMILIES } from "$lib/themes/presets";
	import type { Theme } from "$lib/themes/types";

	let { value, onChange, label }: { value: string; onChange: (v: string) => void; label: string } = $props();

	const order = (t: Theme) => (t.custom ? 99 : FAMILIES.indexOf(t.family));
	const sorted = $derived([...themes.all].sort((a, b) => order(a) - order(b)));
	const light = $derived(sorted.filter((t) => t.kind === "light"));
	const dark = $derived(sorted.filter((t) => t.kind === "dark"));
	const current = $derived(themes.get(value));
</script>

{#snippet dot(t: Theme)}
	<span class="border-border inline-flex size-3.5 shrink-0 overflow-hidden rounded-full border" aria-hidden="true">
		<span class="h-full w-1/2" style:background={t.bg}></span><span class="h-full w-1/2" style:background={t.accent}></span>
	</span>
{/snippet}

<Select.Root type="single" {value} onValueChange={(v) => v && onChange(v)}>
	<Select.Trigger class="w-60" aria-label={label}>
		<span class="flex min-w-0 items-center gap-2">
			{#if current}{@render dot(current)}{/if}
			<span class="truncate">{scopeLabel(value)}</span>
		</span>
	</Select.Trigger>
	<Select.Content class="max-h-80">
		<Select.Group>
			<Select.Item value="app" label="Same as the app">Same as the app</Select.Item>
			<Select.Item value="app-light" label="App's light theme">App's light theme · {themes.forKind("light").name}</Select.Item>
			<Select.Item value="app-dark" label="App's dark theme">App's dark theme · {themes.forKind("dark").name}</Select.Item>
		</Select.Group>
		<Select.Separator />
		<Select.Group>
			<Select.GroupHeading>Dark</Select.GroupHeading>
			{#each dark as t (t.id)}
				<Select.Item value={t.id} label={t.name}><span class="flex min-w-0 items-center gap-2">{@render dot(t)}<span class="truncate">{t.name}</span></span></Select.Item>
			{/each}
		</Select.Group>
		<Select.Separator />
		<Select.Group>
			<Select.GroupHeading>Light</Select.GroupHeading>
			{#each light as t (t.id)}
				<Select.Item value={t.id} label={t.name}><span class="flex min-w-0 items-center gap-2">{@render dot(t)}<span class="truncate">{t.name}</span></span></Select.Item>
			{/each}
		</Select.Group>
	</Select.Content>
</Select.Root>
