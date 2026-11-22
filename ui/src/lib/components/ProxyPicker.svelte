<script lang="ts">
	// Which proxy a conversation uses: the default (named, with where it comes
	// from), a saved proxy, or none.
	import * as Select from "$lib/components/ui/select";
	import { proxies, sourceLabel, type ProxyTarget } from "$lib/state/proxies.svelte";

	let {
		value,
		onChange,
		target,
		class: cls = "w-full",
	}: { value: string | undefined; onChange: (v: string | undefined) => void; target: ProxyTarget; class?: string } = $props();

	const fallback = $derived(proxies.defaultFor(target));
	const defaultLabel = $derived(
		fallback.proxy ? `Default · ${fallback.proxy.name}` : "Default · No proxy",
	);
	const current = $derived(value === "none" ? "No proxy" : value ? (proxies.byId(value)?.name ?? "Removed proxy") : defaultLabel);
</script>

<Select.Root type="single" value={value ?? "default"} onValueChange={(v) => onChange(v === "default" ? undefined : v)}>
	<Select.Trigger class={cls} aria-label="Proxy">{current}</Select.Trigger>
	<Select.Content>
		<Select.Item value="default" label={defaultLabel}>
			<span class="flex min-w-0 flex-col">
				<span>{defaultLabel}</span>
				<span class="text-muted-foreground text-xs">{fallback.source ? `From the ${sourceLabel[fallback.source]}` : "No default is set"}</span>
			</span>
		</Select.Item>
		{#each proxies.list as p (p.id)}
			<Select.Item value={p.id} label={p.name}>{p.name}</Select.Item>
		{/each}
		<Select.Item value="none" label="No proxy">No proxy</Select.Item>
	</Select.Content>
</Select.Root>
