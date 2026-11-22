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
	import ProxyEditor from "$lib/components/ProxyEditor.svelte";
	import ProjectEditor from "$lib/components/ProjectEditor.svelte";
	import SessionRename from "$lib/components/SessionRename.svelte";
	import TerminalPanel from "$lib/components/TerminalPanel.svelte";
	import { termPanel } from "$lib/state/termpanel.svelte";
	import { proxies } from "$lib/state/proxies.svelte";
	import { motionOn, slide } from "$lib/motion";
	import ProgressPanel from "$lib/components/conv/ProgressPanel.svelte";
	import { Button } from "$lib/components/ui/button";
	import PanelRightClose from "@lucide/svelte/icons/panel-right-close";
	import { convActions, hostLost, restoreConversations, startIdleReaper } from "$lib/state/sessions";
	import { startAlerts } from "$lib/state/alerts";
	import { call, inTauri } from "$lib/ipc";
	import { dispose as disposeTerminal, retheme, reconfigure as reconfigureTerminals } from "$lib/state/terminals";
	import { layout } from "$lib/state/layout.svelte";
	import { conversations } from "$lib/state/conversations.svelte";
	import { onKey as runShortcut, startCommands, currentConv, menuOwned } from "$lib/state/commands.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { prefs, isMac, widths } from "$lib/state/prefs.svelte";
	import type { Pane } from "paneforge";

	let sidebarPane = $state<Pane>();

	startIdleReaper();
	startAlerts();
	void restoreConversations().then(() => termPanel.restore());
	if (inTauri)
		import("@tauri-apps/api/event").then(({ listen }) => {
			void listen("host-lost", () => hostLost());
		});

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

	// Settings → Appearance, applied live.
	$effect(() => {
		const root = document.documentElement;
		root.dataset.density = prefs.density;
		root.dataset.motion = prefs.motion ? "on" : "off";
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

	// The sidebar slides when shown or hidden with a button or shortcut
	// (not while its edge is dragged).
	let paneAnim = $state(false);
	let paneAnimTimer: ReturnType<typeof setTimeout> | undefined;
	function animated(run: () => void) {
		if (motionOn()) {
			paneAnim = true;
			clearTimeout(paneAnimTimer);
			paneAnimTimer = setTimeout(() => (paneAnim = false), 320);
		}
		run();
	}
	const collapseSidebar = () => animated(() => sidebarPane?.collapse());
	const expandSidebar = () => animated(() => sidebarPane?.expand());
	ui.toggleSidebar = () => (layout.sidebar.collapsed ? expandSidebar() : collapseSidebar());
	startCommands();

	// New terminals in the panel start in the focused conversation's folder,
	// going out the way that conversation does.
	termPanel.defaults = () => {
		const c = currentConv();
		if (!c) return {};
		return { cwd: c.cwd, proxy: proxies.s.terminals ? (c.proxyActive ?? "none") : "none" };
	};

	// Ctrl+` toggles the terminal panel, Ctrl+Shift+` adds a terminal, also
	// while a terminal has focus (capture phase, before xterm sees it).
	function onKeyCapture(e: KeyboardEvent) {
		if (!e.ctrlKey || e.metaKey || e.altKey || (e.key !== "`" && e.key !== "~" && e.code !== "Backquote")) return;
		if (!e.shiftKey && menuOwned) return; // the app menu has Ctrl+`
		e.preventDefault();
		e.stopPropagation();
		if (e.shiftKey) termPanel.newTerminal();
		else termPanel.toggle();
	}

	// The progress panel sits right of the panes and the terminal panel,
	// for the focused conversation; it hides on narrow windows.
	let mainW = $state(0);
	// The focused conversation; while another kind of tab has focus, the last
	// focused conversation that is still on screen (so the panel doesn't jump).
	const focusedConv = $derived(currentConv());
	let lastConvId = $state<string | null>(null);
	$effect(() => {
		if (focusedConv) lastConvId = focusedConv.id;
	});
	const panelConv = $derived.by(() => {
		if (focusedConv) return focusedConv;
		const onScreen = layout
			.leaves()
			.map((l) => (l.active ? layout.tabs[l.active] : undefined))
			.filter((t) => t?.kind === "conversation" && t.ref)
			.map((t) => t!.ref!);
		const id = lastConvId && onScreen.includes(lastConvId) ? lastConvId : onScreen[0];
		return id ? conversations.get(id) : undefined;
	});
	const rightConv = $derived(prefs.progressPanel && mainW >= 860 ? panelConv : null);

	// The terminal panel's height, dragged or with the arrow keys.
	let mainEl = $state<HTMLElement>();
	function dragPanel(e: PointerEvent) {
		if (!mainEl) return;
		const el = e.currentTarget as HTMLElement;
		el.setPointerCapture(e.pointerId);
		const box = mainEl.getBoundingClientRect();
		const move = (ev: PointerEvent) => {
			termPanel.size = Math.min(85, Math.max(12, ((box.bottom - ev.clientY) / box.height) * 100));
		};
		const up = () => {
			el.removeEventListener("pointermove", move);
			el.removeEventListener("pointerup", up);
			termPanel.save();
		};
		el.addEventListener("pointermove", move);
		el.addEventListener("pointerup", up);
	}
	function panelKey(e: KeyboardEvent) {
		const step = e.shiftKey ? 10 : 3;
		if (e.key === "ArrowUp") termPanel.size = Math.min(85, termPanel.size + step);
		else if (e.key === "ArrowDown") termPanel.size = Math.max(12, termPanel.size - step);
		else return;
		e.preventDefault();
		termPanel.save();
	}

	function onKey(e: KeyboardEvent) {
		// Off macOS the shortcuts use Ctrl, which belongs to the shell
		// (Ctrl+B, Ctrl+W) while a terminal has focus.
		if (!isMac && (e.target as HTMLElement | null)?.closest?.(".xterm")) return;
		if (runShortcut(e)) e.preventDefault();
	}
</script>

<svelte:window onkeydown={onKey} onkeydowncapture={onKeyCapture} bind:innerWidth={winWidth} />

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
<ProxyEditor />
<ProjectEditor />
<SessionRename />

<Tooltip.Provider delayDuration={400}>
	<div class="h-full w-full">
		<Resizable.PaneGroup direction="horizontal" class={paneAnim ? "pane-anim" : ""}>
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
				<Sidebar onCollapse={collapseSidebar} />
			</Resizable.Pane>
			<Resizable.Handle />
			<Resizable.Pane defaultSize={100 - (layout.sidebar.collapsed ? 0 : layout.sidebar.size)}>
				<main class="bg-background flex h-full min-w-0" bind:clientWidth={mainW}>
					<!-- Center: panes, with the terminal panel under them only. -->
					<div class="flex h-full min-w-0 flex-1 flex-col" bind:this={mainEl}>
						<div class="min-h-0 flex-1 {termPanel.open && termPanel.maximized ? 'hidden' : ''}">
							<PaneView node={layout.root} sidebarCollapsed={layout.sidebar.collapsed} onExpandSidebar={expandSidebar} />
						</div>
						{#if termPanel.open}
							{#if !termPanel.maximized}
								<div class="relative z-20 h-0 shrink-0">
									<!-- A focusable window splitter (ARIA separator), moved with the arrow keys. -->
									<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
									<div
										class="hover:bg-primary/50 focus-visible:bg-primary/60 absolute inset-x-0 -top-[3px] h-[6px] cursor-row-resize transition-colors outline-none"
										role="separator"
										aria-orientation="horizontal"
										aria-label="Resize terminal panel"
										aria-valuenow={Math.round(termPanel.size)}
										aria-valuemin={12}
										aria-valuemax={85}
										tabindex="0"
										onpointerdown={dragPanel}
										onkeydown={panelKey}
										ondblclick={() => termPanel.toggleMaximize()}
									></div>
								</div>
							{/if}
							<div
								class="min-h-0 {termPanel.maximized ? 'flex-1' : 'shrink-0'}"
								style:height={termPanel.maximized ? undefined : `${termPanel.size}%`}
								transition:slide={{ duration: 200 }}
							>
								<TerminalPanel />
							</div>
						{/if}
					</div>
					<!-- Right: progress of the focused conversation, full height beside the terminal panel. -->
					{#if rightConv}
						<aside class="border-border bg-sidebar/40 flex h-full w-64 shrink-0 flex-col border-s" aria-label="Progress" transition:slide={{ axis: "x", duration: 200 }}>
							<header class="border-border flex h-(--tab-h) shrink-0 items-center gap-1.5 border-b ps-3 pe-1.5" data-tauri-drag-region>
								<span class="shrink-0 text-[13px] font-medium" data-tauri-drag-region>Progress</span>
								{#if layout.leaves().length > 1}<span class="text-muted-foreground min-w-0 truncate text-[12px]" dir="auto" title={rightConv.title}
										>· {rightConv.title}</span
									>{/if}
								<Button
									variant="ghost"
									size="icon-sm"
									class="text-muted-foreground hover:text-foreground ms-auto"
									aria-label="Hide progress panel"
									title="Hide progress panel ({isMac ? '⌥⌘P' : 'Ctrl+Alt+P'})"
									onclick={() => prefs.set("progressPanel", false)}><PanelRightClose /></Button
								>
							</header>
							<div class="min-h-0 flex-1">
								{#key rightConv.id}<ProgressPanel conv={rightConv} />{/key}
							</div>
						</aside>
					{/if}
				</main>
			</Resizable.Pane>
		</Resizable.PaneGroup>
	</div>
</Tooltip.Provider>
