<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import { Kbd } from "$lib/components/ui/kbd";
	import PanelLeftClose from "@lucide/svelte/icons/panel-left-close";
	import Plus from "@lucide/svelte/icons/plus";
	import Search from "@lucide/svelte/icons/search";
	import Settings from "@lucide/svelte/icons/settings";
	import Users from "@lucide/svelte/icons/users";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import Sun from "@lucide/svelte/icons/sun";
	import Moon from "@lucide/svelte/icons/moon";
	import { toggleMode, mode } from "mode-watcher";
	import { layout } from "$lib/state/layout.svelte";
	import { isMac } from "$lib/state/prefs.svelte";
	import { ui } from "$lib/state/ui.svelte";

	let { onCollapse }: { onCollapse: () => void } = $props();

	const mod = isMac ? "⌘" : "Ctrl";
</script>

<aside class="bg-sidebar text-sidebar-foreground flex h-full flex-col whitespace-nowrap">
	<div class="flex h-11 shrink-0 items-center gap-1 px-2 {isMac ? 'ps-[78px]' : ''}" data-tauri-drag-region>
		<span class="text-sm font-semibold tracking-tight" data-tauri-drag-region>Glaude</span>
		<div class="ms-auto flex items-center">
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" onclick={onCollapse} aria-label="Hide sidebar">
							<PanelLeftClose />
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>Hide sidebar <Kbd>{mod} B</Kbd></Tooltip.Content>
			</Tooltip.Root>
		</div>
	</div>

	<div class="flex flex-col gap-1.5 px-2 pb-2">
		<Button class="justify-start overflow-hidden" size="sm" onclick={() => ui.openNewConversation()}>
			<Plus /> <span class="truncate">New conversation</span>
		</Button>
		<button
			class="border-sidebar-border text-muted-foreground hover:bg-sidebar-accent flex h-8 items-center gap-2 rounded-md border px-2.5 text-start text-[13px]"
			onclick={() => ui.openSearch()}
		>
			<Search class="size-3.5 shrink-0" />
			<span class="truncate">Search history</span>
			<Kbd class="ms-auto">{mod} K</Kbd>
		</button>
	</div>

	<div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-2">
		<section class="py-2">
			<h2 class="text-muted-foreground px-1 pb-1 text-xs font-medium">Sessions</h2>
			{#if ui.sessionsLoading}
				<div class="flex flex-col gap-2 px-1 py-1">
					{#each [0, 1, 2, 3] as i (i)}
						<div class="bg-sidebar-accent h-9 animate-pulse rounded-md"></div>
					{/each}
				</div>
			{:else}
				<div class="text-muted-foreground rounded-md px-2 py-6 text-center text-[13px] leading-relaxed">
					No sessions yet.<br />Start one with <span class="text-sidebar-foreground font-medium">New conversation</span>.
				</div>
			{/if}
		</section>
	</div>

	<div class="border-sidebar-border flex items-center gap-0.5 border-t px-2 py-1.5">
		<Button variant="ghost" size="sm" onclick={() => layout.openTab({ kind: "accounts", title: "Accounts" })}>
			<Users /> Accounts
		</Button>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="ghost" size="icon-sm" aria-label="New terminal" onclick={() => ui.openTerminal()}>
						<SquareTerminal />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>New terminal</Tooltip.Content>
		</Tooltip.Root>
		<div class="ms-auto flex items-center">
			<Button variant="ghost" size="icon-sm" aria-label="Toggle light or dark" onclick={toggleMode}>
				{#if mode.current === "dark"}<Sun />{:else}<Moon />{/if}
			</Button>
			<Button
				variant="ghost"
				size="icon-sm"
				aria-label="Settings"
				onclick={() => layout.openTab({ kind: "settings", title: "Settings" })}
			>
				<Settings />
			</Button>
		</div>
	</div>
</aside>
