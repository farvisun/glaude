<script lang="ts" generics="T extends string">
	// A few mutually exclusive options as buttons.
	let { value, options, onChange, label }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string } = $props();
</script>

<div class="bg-muted inline-flex rounded-md p-0.5" role="radiogroup" aria-label={label}>
	{#each options as o (o.value)}
		<button
			role="radio"
			aria-checked={value === o.value}
			class="rounded px-2.5 py-1 text-xs transition-colors {value === o.value ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
			onclick={() => onChange(o.value)}>{o.label}</button
		>
	{/each}
</div>
