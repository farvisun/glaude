<script lang="ts">
	import { ModeWatcher, mode } from "mode-watcher";
	import { Toaster } from "$lib/components/ui/sonner";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import * as Resizable from "$lib/components/ui/resizable";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import PaneView from "$lib/components/PaneView.svelte";
	import NewConversationDialog from "$lib/components/NewConversationDialog.svelte";
	import SearchPalette from "$lib/components/SearchPalette.svelte";
	import TaskOutputDialog from "$lib/components/conv/TaskOutputDialog.svelte";
	import RewindDialog from "$lib/components/conv/RewindDialog.svelte";
	import SessionPanel from "$lib/components/conv/SessionPanel.svelte";
	import { convActions, startIdleReaper } from "$lib/state/sessions";
	import { dispose as disposeTerminal, retheme } from "$lib/state/terminals";
	import { layout } from "$lib/state/layout.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { prefs, isMac } from "$lib/state/prefs.svelte";
	import type { Pane } from "paneforge";

	let sidebarPane = $state<Pane>();

	startIdleReaper();

	layout.onClose = (tab) => {
		if (tab.kind === "conversation" && tab.ref) convActions.close(tab.ref);
		if (tab.kind === "terminal") disposeTerminal(tab.id);
	};

	// Terminals read their colors from the theme variables.
	$effect(() => {
		void prefs.palette;
		void mode.current;
		requestAnimationFrame(retheme);
	});

	$effect(() => {
		document.documentElement.dataset.palette = prefs.palette;
	});

	$effect(() => {
		document.documentElement.classList.toggle("platform-mac", isMac);
	});

	// paneforge sizes are percentages; keep the sidebar at least ~230px and
	// fold it away on narrow windows.
	let winWidth = $state(window.innerWidth);
	const sidebarMin = $derived(Math.min(40, Math.max(12, (230 / winWidth) * 100)));
	let autoCollapsed = false;
	$effect(() => {
		if (winWidth < 760 && !layout.sidebar.collapsed) {
			autoCollapsed = true;
			sidebarPane?.collapse();
		} else if (winWidth >= 760 && autoCollapsed && layout.sidebar.collapsed) {
			autoCollapsed = false;
			sidebarPane?.expand();
		}
	});

	function onKey(e: KeyboardEvent) {
		// Off macOS the shortcuts use Ctrl, which belongs to the shell
		// (Ctrl+B, Ctrl+W) while a terminal has focus.
		if (!isMac && (e.target as HTMLElement | null)?.closest?.(".xterm")) return;
		const mod = isMac ? e.metaKey : e.ctrlKey;
		if (mod && e.key.toLowerCase() === "b") {
			e.preventDefault();
			if (layout.sidebar.collapsed) sidebarPane?.expand();
			else sidebarPane?.collapse();
		}
		if (mod && e.key.toLowerCase() === "k") {
			e.preventDefault();
			ui.searchOpen = !ui.searchOpen;
		}
		if (mod && e.key.toLowerCase() === "w") {
			e.preventDefault();
			const active = layout.focusedLeaf.active;
			if (active) layout.closeTab(active);
		}
	}
</script>

<svelte:window onkeydown={onKey} bind:innerWidth={winWidth} />

<ModeWatcher defaultMode="dark" />
<Toaster position="bottom-right" richColors closeButton />
<NewConversationDialog />
<SearchPalette />
<TaskOutputDialog />
<RewindDialog />
<SessionPanel />

<Tooltip.Provider delayDuration={400}>
	<div class="h-full w-full">
		<Resizable.PaneGroup direction="horizontal">
			<Resizable.Pane
				bind:this={sidebarPane}
				defaultSize={layout.sidebar.collapsed ? 0 : Math.max(layout.sidebar.size, sidebarMin)}
				minSize={sidebarMin}
				maxSize={40}
				collapsible
				collapsedSize={0}
				onCollapse={() => {
					layout.sidebar.collapsed = true;
					layout.saveSidebar();
				}}
				onExpand={() => {
					layout.sidebar.collapsed = false;
					layout.saveSidebar();
				}}
				onResize={(size) => {
					if (size > 0) {
						layout.sidebar.size = size;
						layout.saveSidebar();
					}
				}}
			>
				<Sidebar onCollapse={() => sidebarPane?.collapse()} />
			</Resizable.Pane>
			<Resizable.Handle />
			<Resizable.Pane defaultSize={100 - (layout.sidebar.collapsed ? 0 : layout.sidebar.size)}>
				<main class="bg-background h-full min-w-0">
					<PaneView node={layout.root} sidebarCollapsed={layout.sidebar.collapsed} onExpandSidebar={() => sidebarPane?.expand()} />
				</main>
			</Resizable.Pane>
		</Resizable.PaneGroup>
	</div>
</Tooltip.Provider>
