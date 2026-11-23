<script lang="ts">
	// The terminal panel (Ctrl+`): terminals in groups, split side by side,
	// with a list of all of them when there is more than one.
	import { themes } from "$lib/themes/themes.svelte";
	import { termPanel } from "$lib/state/termpanel.svelte";
	import { basename } from "$lib/format";
	import { isMac } from "$lib/state/prefs.svelte";
	import { layout } from "$lib/state/layout.svelte";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import ChevronsUpDown from "@lucide/svelte/icons/chevrons-up-down";
	import * as Resizable from "$lib/components/ui/resizable";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import TermHost from "./TermHost.svelte";
	import Plus from "@lucide/svelte/icons/plus";
	import Columns2 from "@lucide/svelte/icons/columns-2";
	import Trash from "@lucide/svelte/icons/trash-2";
	import ChevronUp from "@lucide/svelte/icons/chevron-up";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import X from "@lucide/svelte/icons/x";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import Shield from "@lucide/svelte/icons/shield";
	import { proxies } from "$lib/state/proxies.svelte";

	// Settings → Appearance → Terminal theme, when it differs from the window's.
	const termIsland = $derived(themes.terminalIsland);

	const group = $derived(termPanel.groups.find((g) => g.id === termPanel.active) ?? termPanel.groups[0]);
	const baseLabel = (id: string) => {
		const t = termPanel.titles[id]?.trim();
		if (t) return t.replace(/^[^@\s]+@[^:\s]+:\s*/, "");
		return basename(termPanel.specs[id]?.cwd) || "Terminal";
	};
	// Same names get a number: "glaude", "glaude 2".
	const labels = $derived.by(() => {
		const out: Record<string, string> = {};
		const seen: Record<string, number> = {};
		for (const g of termPanel.groups)
			for (const id of g.terms) {
				const b = baseLabel(id);
				seen[b] = (seen[b] ?? 0) + 1;
				out[id] = seen[b] > 1 ? `${b} ${seen[b]}` : b;
			}
		return out;
	});
	const label = (id: string) => labels[id] ?? baseLabel(id);
	const focusedSpec = $derived(termPanel.focused ? termPanel.specs[termPanel.focused] : undefined);
	const focusedProxy = $derived(
		focusedSpec ? proxies.byId(focusedSpec.proxy === undefined ? proxies.forShell(focusedSpec.cwd) : focusedSpec.proxy) : null,
	);
	const mod = isMac ? "⌘" : "Ctrl+";
</script>

{#snippet action(text: string, Icon: typeof Plus, run: () => void, keys?: string)}
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<button {...props} class="hover:bg-accent hover:text-foreground grid size-6 place-items-center rounded-md" aria-label={text} onclick={run}>
					<Icon class="size-3.5" />
				</button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content>{text}{keys ? ` (${keys})` : ""}</Tooltip.Content>
	</Tooltip.Root>
{/snippet}

<section
	class="terminal-panel @container/term border-border flex h-full min-h-0 flex-col border-t bg-(--term-bg) {termIsland ? `theme-island ${termIsland.kind}` : ''}"
	style={termIsland?.style}
	aria-label="Terminal"
>
	<!-- Maximized with the sidebar hidden, the macOS window buttons sit over this row. -->
	<header
		class="text-muted-foreground flex h-8 shrink-0 items-center gap-2 pe-1.5 text-[11px] {isMac && termPanel.maximized && layout.sidebar.collapsed
			? 'ps-[78px]'
			: 'ps-3'}"
		data-tauri-drag-region={termPanel.maximized ? true : undefined}
	>
		<span class="text-foreground/80 font-semibold tracking-wide uppercase">Terminal</span>
		{#if termPanel.count > 1}
			<!-- Narrow panels list the terminals here instead of on the side. -->
			<DropdownMenu.Root>
				<DropdownMenu.Trigger class="hover:text-foreground inline-flex min-w-0 items-center gap-1 @xl/term:hidden"
					><span class="truncate">{termPanel.focused ? label(termPanel.focused) : ""}</span><ChevronsUpDown class="size-3 shrink-0" /></DropdownMenu.Trigger
				>
				<DropdownMenu.Content align="start" class="w-52">
					{#each termPanel.groups as g (g.id)}
						{#each g.terms as id (id)}
							<DropdownMenu.Item onclick={() => termPanel.activate(g.id, id)}><SquareTerminal />{label(id)}</DropdownMenu.Item>
						{/each}
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
		{#if focusedSpec?.cwd}<span class="hidden truncate font-mono @xl/term:inline" dir="ltr" title={focusedSpec.cwd}>{focusedSpec.cwd}</span>{/if}
		{#if focusedProxy}<span class="inline-flex shrink-0 items-center gap-1" title="This terminal goes out through {focusedProxy.name}"
				><Shield class="size-3" />{focusedProxy.name}</span
			>{/if}
		<span class="ms-auto flex items-center gap-0.5">
			{@render action("New terminal", Plus, () => termPanel.newTerminal(), `${mod}T`)}
			{@render action("Split terminal", Columns2, () => termPanel.split(), `${mod}\\`)}
			{@render action("Kill terminal", Trash, () => termPanel.kill())}
			{#if termPanel.maximized}
				{@render action("Restore panel size", ChevronDown, () => termPanel.toggleMaximize())}
			{:else}
				{@render action("Maximize panel", ChevronUp, () => termPanel.toggleMaximize())}
			{/if}
			{@render action("Hide panel", X, () => termPanel.hide(), "Ctrl+`")}
		</span>
	</header>
	<div class="flex min-h-0 flex-1">
		<div class="min-w-0 flex-1">
			{#if group}
				{#key group.id + ":" + group.terms.length}
					<Resizable.PaneGroup direction="horizontal">
						{#each group.terms as id, i (id)}
							{#if i > 0}<Resizable.Handle />{/if}
							<Resizable.Pane id={id} order={i} minSize={10}>
								<div class="h-full {group.terms.length > 1 && termPanel.focused === id ? 'ring-primary/50 ring-1 ring-inset' : ''}">
									<TermHost {id} spec={termPanel.specs[id]} autofocus={termPanel.focused === id} onFocus={() => (termPanel.focused = id)} />
								</div>
							</Resizable.Pane>
						{/each}
					</Resizable.PaneGroup>
				{/key}
			{/if}
		</div>
		{#if termPanel.count > 1}
			<ul class="border-border scrollbar-thin hidden w-48 shrink-0 overflow-y-auto border-s py-1 text-[12px] @xl/term:block" aria-label="Terminals">
				{#each termPanel.groups as g (g.id)}
					{#each g.terms as id, i (id)}
						<li class="group/term">
							<button
								class="hover:bg-accent/60 flex h-6 w-full min-w-0 items-center gap-1.5 pe-1 text-start {termPanel.focused === id
									? 'bg-accent text-foreground'
									: g.id === group?.id
										? 'text-foreground'
										: 'text-muted-foreground'}"
								style="padding-inline-start: {g.terms.length > 1 ? 1.4 : 0.6}rem"
								onclick={() => termPanel.activate(g.id, id)}
								title={termPanel.specs[id]?.cwd}
							>
								{#if g.terms.length > 1}<span class="text-muted-foreground -ms-3 w-2.5 font-mono">{i === g.terms.length - 1 ? "└" : "├"}</span>{/if}
								<SquareTerminal class="size-3.5 shrink-0" />
								<span class="min-w-0 flex-1 truncate" dir="auto">{label(id)}</span>
								<span
									role="button"
									tabindex="0"
									class="hover:text-foreground invisible grid size-5 place-items-center rounded group-hover/term:visible"
									aria-label="Kill {label(id)}"
									onclick={(e) => {
										e.stopPropagation();
										termPanel.kill(id);
									}}
									onkeydown={(e) => e.key === "Enter" && termPanel.kill(id)}><Trash class="size-3" /></span
								>
							</button>
						</li>
					{/each}
				{/each}
			</ul>
		{/if}
	</div>
</section>
