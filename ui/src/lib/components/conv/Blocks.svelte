<script lang="ts">
	import type { Block } from "$lib/types";
	import { codeBlocks } from "$lib/actions/codeblocks";

	let { blocks, streaming = false }: { blocks: Block[]; streaming?: boolean } = $props();
</script>

<!-- Blocks are sanitized HTML rendered in Rust (pulldown-cmark + ammonia). -->
{#each blocks as b, i (i)}
	<div class="prose-g" dir={b.dir} use:codeBlocks={b.html}>
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html b.html}
	</div>
{/each}

<style>
	.prose-g + .prose-g {
		margin-block-start: 0.7em;
	}
</style>
