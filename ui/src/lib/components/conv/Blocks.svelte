<script lang="ts">
	import type { Block } from "$lib/types";

	let { blocks, streaming = false }: { blocks: Block[]; streaming?: boolean } = $props();
</script>

<!-- Blocks are sanitized HTML rendered in Rust (pulldown-cmark + ammonia). -->
{#each blocks as b, i (i)}
	<div class="prose-g" dir={b.dir}>
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html b.html}
		{#if streaming && i === blocks.length - 1}<span class="caret" aria-hidden="true"></span>{/if}
	</div>
{/each}

<style>
	.prose-g + .prose-g {
		margin-block-start: 0.7em;
	}
	.caret {
		display: inline-block;
		width: 0.5em;
		height: 1em;
		margin-inline-start: 2px;
		vertical-align: text-bottom;
		background: var(--primary);
		opacity: 0.7;
		animation: blink 1s steps(2, start) infinite;
	}
	@keyframes blink {
		to {
			visibility: hidden;
		}
	}
</style>
