<script lang="ts" generics="T extends string | number">
	// A shadcn Select for a small list of values.
	import * as Select from "$lib/components/ui/select";

	let {
		value,
		options,
		onChange,
		label,
		class: cls = "w-44",
	}: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string; class?: string } = $props();
	const current = $derived(options.find((o) => o.value === value)?.label ?? String(value));
</script>

<Select.Root
	type="single"
	value={String(value)}
	onValueChange={(v) => {
		const o = options.find((x) => String(x.value) === v);
		if (o) onChange(o.value);
	}}
>
	<Select.Trigger class={cls} aria-label={label}>{current}</Select.Trigger>
	<Select.Content>
		{#each options as o (o.value)}
			<Select.Item value={String(o.value)} label={o.label}>{o.label}</Select.Item>
		{/each}
	</Select.Content>
</Select.Root>
