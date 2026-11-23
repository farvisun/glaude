<script lang="ts">
	// A colour: swatch (opens the system colour picker) and its hex value.
	// Optional colours may be left empty ("auto": derived from the others).
	import { Input } from "$lib/components/ui/input";
	import { isColor, norm } from "$lib/themes/color";

	let {
		label,
		value,
		onChange,
		auto,
	}: { label: string; value: string | undefined; onChange: (v: string | undefined) => void; auto?: string } = $props();
	let text = $state("");
	$effect(() => {
		text = value ?? "";
	});
	const shown = $derived(value && isColor(value) ? norm(value) : (auto ?? "#000000"));
	function commit() {
		const t = text.trim();
		if (!t && auto !== undefined) return onChange(undefined);
		if (isColor(t)) onChange(norm(t));
		else text = value ?? "";
	}
</script>

<div class="flex min-w-0 items-center gap-2">
	<label class="border-border relative size-7 shrink-0 cursor-pointer overflow-hidden rounded-md border" style:background={shown} title="Pick {label.toLowerCase()}">
		<input type="color" class="absolute inset-0 cursor-pointer opacity-0" value={shown} oninput={(e) => onChange(e.currentTarget.value)} aria-label="{label}: pick a colour" />
	</label>
	<span class="min-w-0 flex-1 truncate text-[12px]" title={label}>{label}</span>
	<Input
		class="h-7 w-[5.75rem] shrink-0 px-2 font-mono text-[12px]"
		bind:value={text}
		placeholder={auto !== undefined ? "auto" : ""}
		onchange={commit}
		onkeydown={(e) => e.key === "Enter" && commit()}
		aria-label="{label} hex"
		dir="ltr"
	/>
</div>
