<script lang="ts" module>
	// Icons a project can wear in the sidebar (Settings keeps only the name).
	import Folder from "@lucide/svelte/icons/folder";
	import Code from "@lucide/svelte/icons/code";
	import Globe from "@lucide/svelte/icons/globe";
	import Server from "@lucide/svelte/icons/server";
	import Smartphone from "@lucide/svelte/icons/smartphone";
	import Database from "@lucide/svelte/icons/database";
	import Box from "@lucide/svelte/icons/box";
	import Rocket from "@lucide/svelte/icons/rocket";
	import BookOpen from "@lucide/svelte/icons/book-open";
	import FlaskConical from "@lucide/svelte/icons/flask-conical";
	import ShoppingCart from "@lucide/svelte/icons/shopping-cart";
	import Bot from "@lucide/svelte/icons/bot";
	import Palette from "@lucide/svelte/icons/palette";
	import Wrench from "@lucide/svelte/icons/wrench";
	export const PROJECT_ICONS = {
		folder: Folder,
		code: Code,
		globe: Globe,
		server: Server,
		phone: Smartphone,
		database: Database,
		box: Box,
		rocket: Rocket,
		book: BookOpen,
		flask: FlaskConical,
		shop: ShoppingCart,
		bot: Bot,
		design: Palette,
		tools: Wrench,
	} as const;
	export type ProjectIconId = keyof typeof PROJECT_ICONS;
</script>

<script lang="ts">
	import { colorOf, type Project } from "$lib/state/projects.svelte";

	let { project, class: cls = "size-3.5" }: { project: Pick<Project, "color" | "icon">; class?: string } = $props();
	const Icon = $derived(project.icon ? PROJECT_ICONS[project.icon as ProjectIconId] : null);
	const color = $derived(colorOf(project));
</script>

{#if Icon}
	<Icon class="{cls} shrink-0" style={color ? `color: ${color}` : undefined} aria-hidden="true" />
{:else}
	<span class="grid {cls} shrink-0 place-items-center" aria-hidden="true">
		<span class="size-2 rounded-full {color ? '' : 'bg-muted-foreground/50'}" style:background={color}></span>
	</span>
{/if}
