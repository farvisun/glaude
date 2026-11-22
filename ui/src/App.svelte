<script lang="ts">
	import { ModeWatcher, mode } from "mode-watcher";
	import { Toaster } from "$lib/components/ui/sonner";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import * as Resizable from "$lib/components/ui/resizable";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import PaneView from "$lib/components/PaneView.svelte";
	import NewConversationDialog from "$lib/components/NewConversationDialog.svelte";
	import SearchPalette from "$lib/components/SearchPalette.svelte";
	import CommandPalette from "$lib/components/CommandPalette.svelte";
	import PromptEditor from "$lib/components/PromptEditor.svelte";
	import PromptFill from "$lib/components/PromptFill.svelte";
	import TaskOutputDialog from "$lib/components/conv/TaskOutputDialog.svelte";
	import RewindDialog from "$lib/components/conv/RewindDialog.svelte";
	import SessionPanel from "$lib/components/conv/SessionPanel.svelte";
	import PathMenu from "$lib/components/PathMenu.svelte";
	import { convActions, hostLost, restoreConversations, startIdleReaper } from "$lib/state/sessions";
	import { startAlerts } from "$lib/state/alerts";
	import { call, inTauri } from "$lib/ipc";
	import { dispose as disposeTerminal, retheme, reconfigure as reconfigureTerminals } from "$lib/state/terminals";
	import { layout } from "$lib/state/layout.svelte";
	import { onKey as runShortcut, startCommands } from "$lib/state/commands.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { prefs, isMac, widths } from "$lib/state/prefs.svelte";
	import type { Pane } from "paneforge";

	let sidebarPane = $state<Pane>();

	startIdleReaper();
	startAlerts();
	void restoreConversations();
	if (inTauri)
		import("@tauri-apps/api/event").then(({ listen }) => {
			void listen("host-lost", () => hostLost());
		});

	layout.onClose = (tab) => {
		if (tab.kind === "conversation" && tab.ref) {
			convActions.close(tab.ref);
			disposeTerminal(`drawer:${tab.ref}`);
		}
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

	// Settings → Appearance, applied live.
	$effect(() => {
		const root = document.documentElement;
		root.dataset.density = prefs.density;
		root.style.setProperty("--code-font", prefs.fontStack(prefs.codeFont, prefs.codeFontCustom));
		root.style.setProperty("--msg-w", widths[prefs.messageWidth]);
	});
	$effect(() => {
		const z = prefs.uiScale;
		if (inTauri)
			import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
				getCurrentWebview()
					.setZoom(z)
					.catch(() => {}),
			);
		else (document.documentElement.style as CSSStyleDeclaration & { zoom: string }).zoom = String(z);
	});
	// The background host reads these (quit behaviour, notifications while closed).
	$effect(() => {
		const keepAgents = prefs.keepAgents;
		const notifyClosed = prefs.notifyClosed;
		call("app_prefs_set", { keepAgents, notifyClosed }).catch(() => {});
	});
	// Terminal font, size and cursor.
	$effect(() => {
		void [prefs.termFont, prefs.termFontCustom, prefs.termFontSize, prefs.termCursor, prefs.termCursorBlink];
		requestAnimationFrame(reconfigureTerminals);
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

	ui.toggleSidebar = () => {
		if (layout.sidebar.collapsed) sidebarPane?.expand();
		else sidebarPane?.collapse();
	};
	startCommands();

	function onKey(e: KeyboardEvent) {
		// Off macOS the shortcuts use Ctrl, which belongs to the shell
		// (Ctrl+B, Ctrl+W) while a terminal has focus.
		if (!isMac && (e.target as HTMLElement | null)?.closest?.(".xterm")) return;
		if (runShortcut(e)) e.preventDefault();
	}
</script>

<svelte:window onkeydown={onKey} bind:innerWidth={winWidth} />

<ModeWatcher defaultMode="dark" />
<Toaster position="bottom-right" richColors closeButton />
<NewConversationDialog />
<SearchPalette />
<CommandPalette />
<PromptEditor />
<PromptFill />
<TaskOutputDialog />
<RewindDialog />
<SessionPanel />
<PathMenu />

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
