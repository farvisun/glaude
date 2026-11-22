<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import { Kbd } from "$lib/components/ui/kbd";
	import PanelLeftClose from "@lucide/svelte/icons/panel-left-close";
	import Plus from "@lucide/svelte/icons/plus";
	import Search from "@lucide/svelte/icons/search";
	import Settings from "@lucide/svelte/icons/settings";
	import Users from "@lucide/svelte/icons/users";
	import PanelBottom from "@lucide/svelte/icons/panel-bottom";
	import Sun from "@lucide/svelte/icons/sun";
	import Moon from "@lucide/svelte/icons/moon";
	import { toggleMode, mode } from "mode-watcher";
	import { layout } from "$lib/state/layout.svelte";
	import { isMac } from "$lib/state/prefs.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { history } from "$lib/state/history.svelte";
	import SessionList from "./SessionList.svelte";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import ArrowUpDown from "@lucide/svelte/icons/arrow-up-down";
	import FolderPlus from "@lucide/svelte/icons/folder-plus";
	import Check from "@lucide/svelte/icons/check";
	import { projects, type ProjectSort, type SessionSort } from "$lib/state/projects.svelte";

	const projectSorts: { value: ProjectSort; label: string }[] = [
		{ value: "manual", label: "Your order (drag)" },
		{ value: "recent", label: "Recent activity" },
		{ value: "name", label: "Name" },
	];
	const sessionSorts: { value: SessionSort; label: string }[] = [
		{ value: "recent", label: "Recent activity" },
		{ value: "created", label: "Newest first" },
		{ value: "name", label: "Name" },
	];
	import { termPanel } from "$lib/state/termpanel.svelte";
	import LiveAgents from "./LiveAgents.svelte";
	import { onMount } from "svelte";

	let { onCollapse }: { onCollapse: () => void } = $props();

	const mod = isMac ? "⌘" : "Ctrl";

	onMount(() => {
		history.refresh();
		const t = setInterval(() => history.refresh(), 60_000);
		return () => clearInterval(t);
	});
</script>

<aside class="bg-sidebar text-sidebar-foreground flex h-full flex-col whitespace-nowrap">
	<!-- On macOS the window buttons sit on the left of this row (centered on it, see tauri.conf.json). -->
	<div class="flex h-(--tab-h) shrink-0 items-center gap-1 ps-3 pe-1.5 select-none {isMac ? 'ps-[78px]' : ''}" data-tauri-drag-region>
		<span class="text-[13px] leading-none font-semibold tracking-tight" data-tauri-drag-region>Glaude</span>
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
		<LiveAgents />
		<section class="pb-2">
			<div class="group/head flex h-7 items-center gap-0.5 ps-1 pe-1">
				<h2 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Projects</h2>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger
						class="text-muted-foreground hover:text-foreground hover:bg-sidebar-accent ms-auto grid size-6 place-items-center rounded"
						aria-label="Sort projects and conversations"
						title="Sort"><ArrowUpDown class="size-3.5" /></DropdownMenu.Trigger
					>
					<DropdownMenu.Content align="end" class="w-52">
						<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">Projects</DropdownMenu.Label>
						{#each projectSorts as o (o.value)}
							<DropdownMenu.Item onclick={() => projects.setSort(o.value)}
								><Check class={projects.sort === o.value ? "" : "invisible"} /><span>{o.label}</span></DropdownMenu.Item
							>
						{/each}
						<DropdownMenu.Separator />
						<DropdownMenu.Label class="text-muted-foreground text-xs font-normal">Conversations</DropdownMenu.Label>
						{#each sessionSorts as o (o.value)}
							<DropdownMenu.Item onclick={() => projects.setSessionSort(o.value)}
								><Check class={projects.sessionSort === o.value ? "" : "invisible"} /><span>{o.label}</span></DropdownMenu.Item
							>
						{/each}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
				<button
					class="text-muted-foreground hover:text-foreground hover:bg-sidebar-accent grid size-6 place-items-center rounded"
					aria-label="New project"
					title="New project"
					onclick={() => (ui.projectEdit = {})}><FolderPlus class="size-3.5" /></button
				>
			</div>
			<SessionList />
		</section>
	</div>

	{#snippet tip(text: string, keys?: string)}
		{text}{#if keys}<Kbd>{keys}</Kbd>{/if}
	{/snippet}
	<div class="border-sidebar-border flex h-(--bar-h) shrink-0 items-center gap-0.5 border-t ps-1.5 pe-1.5">
		<Button variant="ghost" size="sm" class="text-muted-foreground hover:text-foreground h-7 px-2" onclick={() => layout.openTab({ kind: "accounts", title: "Accounts" })}>
			<Users /> Accounts
		</Button>
		<div class="ms-auto flex items-center gap-0.5">
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:text-foreground {termPanel.open ? 'bg-sidebar-accent text-foreground' : ''}"
							aria-label="Terminal panel"
							aria-pressed={termPanel.open}
							onclick={() => termPanel.toggle()}
						>
							<PanelBottom />
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>{@render tip("Terminal panel", isMac ? "⌃`" : "Ctrl+`")}</Tooltip.Content>
			</Tooltip.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" class="text-muted-foreground hover:text-foreground" aria-label="Toggle light or dark" onclick={toggleMode}>
							{#if mode.current === "dark"}<Sun />{:else}<Moon />{/if}
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>{mode.current === "dark" ? "Light theme" : "Dark theme"}</Tooltip.Content>
			</Tooltip.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:text-foreground"
							aria-label="Settings"
							onclick={() => layout.openTab({ kind: "settings", title: "Settings" })}
						>
							<Settings />
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>{@render tip("Settings", isMac ? "⌘," : "Ctrl+,")}</Tooltip.Content>
			</Tooltip.Root>
		</div>
	</div>
</aside>
