<script lang="ts">
	import type { Tab } from "$lib/state/layout.svelte";

	let { tab, active }: { tab: Tab; active: boolean } = $props();
</script>

<div class="text-muted-foreground grid h-full place-items-center text-[13px]" data-tab={tab.id} data-active={active}>
	Terminal arrives in a later build.
</div>
