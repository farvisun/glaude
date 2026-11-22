<script lang="ts">
	// Settings: sections on the side (a bottom sheet on narrow widths),
	// search across all of them on top.
	import { setContext } from "svelte";
	import { prefs, isMac, type Palette, type Editor, type Prefs } from "$lib/state/prefs.svelte";
	import { setMode, userPrefersMode } from "mode-watcher";
	import { Switch } from "$lib/components/ui/switch";
	import { Input } from "$lib/components/ui/input";
	import { Button } from "$lib/components/ui/button";
	import * as Sheet from "$lib/components/ui/sheet";
	import { call } from "$lib/ipc";
	import { editorNames } from "$lib/state/paths";
	import { permissionModes } from "$lib/state/sessions";
	import { claudeModels, efforts, loadNewConv, saveNewConv, type NewConvDefaults } from "$lib/state/newconv";
	import { narrow } from "$lib/state/media.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import type { Provider } from "$lib/types";
	import SettingRow from "./settings/SettingRow.svelte";
	import Choice from "./settings/Choice.svelte";
	import Segmented from "./settings/Segmented.svelte";
	import Palette_ from "@lucide/svelte/icons/palette";
	import MessageSquarePlus from "@lucide/svelte/icons/message-square-plus";
	import MessagesSquare from "@lucide/svelte/icons/messages-square";
	import BellRing from "@lucide/svelte/icons/bell-ring";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import Keyboard from "@lucide/svelte/icons/keyboard";
	import Search from "@lucide/svelte/icons/search";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import Minus from "@lucide/svelte/icons/minus";
	import Plus from "@lucide/svelte/icons/plus";

	const sections = [
		{ id: "appearance", title: "Appearance", icon: Palette_, keys: ["uiScale", "density", "codeFont", "codeFontCustom", "messageWidth"] },
		{ id: "new", title: "New conversations", icon: MessageSquarePlus, keys: ["sendKey"] },
		{ id: "conversations", title: "Conversations", icon: MessagesSquare, keys: ["statuslineScript", "progressPanel", "editor"] },
		{ id: "background", title: "Background & alerts", icon: BellRing, keys: ["keepAgents", "idleMinutes", "notifyDone", "notifyWaiting", "notifyErrors", "notifyClosed", "notifySound"] },
		{ id: "terminal", title: "Terminal", icon: SquareTerminal, keys: ["termFont", "termFontCustom", "termFontSize", "termCursor", "termCursorBlink", "termShell"] },
		{ id: "keys", title: "Keyboard shortcuts", icon: Keyboard, keys: [] },
	] as const;
	type SectionId = (typeof sections)[number]["id"];

	const SECTION_KEY = "glaude.settings.section";
	let active = $state<SectionId>((localStorage.getItem(SECTION_KEY) as SectionId) || "appearance");
	let query = $state("");
	let sheetOpen = $state(false);
	setContext("settings-query", () => query);

	function pick(id: SectionId) {
		active = id;
		query = "";
		sheetOpen = false;
		localStorage.setItem(SECTION_KEY, id);
	}
	const searching = $derived(query.trim().length > 0);
	// Opened from the menu at a section (Help → Keyboard Shortcuts).
	$effect(() => {
		const s = ui.settingsSection as SectionId | null;
		if (s && sections.some((x) => x.id === s)) {
			pick(s);
			ui.settingsSection = null;
		}
	});
	const visible = (id: SectionId) => searching || active === id;
	const current = $derived(sections.find((s) => s.id === active) ?? sections[0]);

	const palettes: { id: Palette; name: string; note: string; swatch: string[] }[] = [
		{ id: "lajevard", name: "Lajevard", note: "Lapis ink with a turquoise accent", swatch: ["#101a2e", "#5cc8bd", "#f2b544", "#e4eaf4"] },
		{ id: "dood", name: "Dood", note: "Quiet graphite with an indigo accent", swatch: ["#18181b", "#9aa4ff", "#f5b041", "#e7e7ea"] },
	];
	const fonts = [
		{ value: "system" as const, label: "System monospace" },
		{ value: "sf" as const, label: "SF Mono" },
		{ value: "jetbrains" as const, label: "JetBrains Mono" },
		{ value: "menlo" as const, label: "Menlo" },
		{ value: "custom" as const, label: "Other…" },
	];

	let editors = $state<Editor[]>(["default"]);
	call<Editor[]>("editors_available")
		.then((list) => (editors = ["default", ...list]))
		.catch(() => {});

	// New conversations: the same defaults the dialog remembers.
	let nc = $state<NewConvDefaults>(loadNewConv());
	function setNc(change: (d: NewConvDefaults) => void) {
		const next = $state.snapshot(nc) as NewConvDefaults;
		change(next);
		nc = next;
		saveNewConv(next);
	}
	let accounts = $state<{ provider: Provider; name: string; archived: boolean }[]>([]);
	call<typeof accounts>("accounts_brief")
		.then((a) => (accounts = a))
		.catch(() => {});
	let codexModels = $state<{ value: string; label: string }[]>([]);
	$effect(() => {
		call<{ value: string; label: string }[]>("codex_models", { account: nc.account.codex })
			.then((l) => (codexModels = l.map((m) => ({ value: m.value, label: m.label || m.value }))))
			.catch(() => (codexModels = []));
	});
	const accountOptions = (p: Provider) => {
		const names = accounts.filter((a) => a.provider === p && !a.archived).map((a) => a.name);
		if (!names.includes(nc.account[p])) names.unshift(nc.account[p]);
		return names.map((n) => ({ value: n, label: n }));
	};
	const modelOptions = (p: Provider) => (p === "claude" ? claudeModels : [{ value: "", label: "Default" }, ...codexModels]);

	const mod = isMac ? "⌘" : "Ctrl+";
	const shortcuts: [string, string][] = [
		["New conversation", `${mod}N`],
		["New terminal", `${mod}T`],
		["Close tab", `${mod}W`],
		["Next / previous tab", isMac ? "⌃Tab / ⌃⇧Tab" : "Ctrl+Tab / Ctrl+Shift+Tab"],
		["Go to tab 1–9", `${mod}1 … ${mod}9`],
		["Split right / down", `${mod}\\ / ${mod}⇧\\`.replace("⌘⇧", "⇧⌘")],
		["Search history", `${mod}K`],
		["Toggle sidebar", `${mod}B`],
		["Changes of the project", isMac ? "⇧⌘G" : "Ctrl+Shift+G"],
		["Terminal under the conversation", "Ctrl+`"],
		["Side chat", "/btw <question>"],
		["Send / queue", prefs.sendKey === "enter" ? "↵" : `${mod}↵`],
		["Send into the running turn", prefs.sendKey === "enter" ? `${mod}↵` : isMac ? "⇧⌘↵" : "Ctrl+Shift+↵"],
		["Stop the agent", "Esc"],
		["Earlier prompts", "↑ / ↓"],
		["Stash the draft", `${mod}S`],
		["Next permission mode", "⇧Tab"],
		["Zoom in / out / reset", `${mod}+ / ${mod}− / ${mod}0`],
		["Settings", `${mod},`],
	];

	const resetSection = (keys: readonly string[]) => prefs.reset(keys as (keyof Prefs)[]);
</script>

{#snippet nav()}
	<nav class="flex flex-col gap-0.5" aria-label="Settings sections">
		{#each sections as s (s.id)}
			<button
				class="flex h-8 items-center gap-2 rounded-md px-2.5 text-start text-[13px] {active === s.id && !searching
					? 'bg-accent text-foreground font-medium'
					: 'text-muted-foreground hover:bg-accent/60 hover:text-foreground'}"
				aria-current={active === s.id && !searching ? "page" : undefined}
				onclick={() => pick(s.id)}
			>
				<s.icon class="size-4 shrink-0" />
				{s.title}
			</button>
		{/each}
	</nav>
{/snippet}

{#snippet header(title: string, keys: readonly string[])}
	<div class="flex items-center gap-2 pt-1 pb-1">
		<h2 class="text-[15px] font-semibold">{title}</h2>
		{#if keys.length && !searching}
			<Button variant="ghost" size="sm" class="text-muted-foreground ms-auto h-7 text-xs" onclick={() => resetSection(keys)}>Reset to defaults</Button>
		{/if}
	</div>
{/snippet}

<div class="flex h-full min-h-0">
	{#if !narrow.current}
		<aside class="border-border flex w-56 shrink-0 flex-col gap-3 border-e px-3 py-5">
			<h1 class="px-2.5 text-lg font-bold">Settings</h1>
			{@render nav()}
		</aside>
	{/if}

	<div class="scrollbar-thin min-w-0 flex-1 overflow-y-auto">
		<div class="mx-auto flex max-w-2xl flex-col gap-6 px-5 py-5 sm:px-8">
			<div class="flex items-center gap-2">
				{#if narrow.current}
					<Button variant="outline" size="sm" class="h-9 shrink-0 gap-1.5" onclick={() => (sheetOpen = true)}>
						<current.icon class="size-4" />
						{current.title}
						<ChevronDown class="size-3.5 opacity-60" />
					</Button>
				{/if}
				<div class="relative flex-1">
					<Search class="text-muted-foreground absolute start-2.5 top-1/2 size-4 -translate-y-1/2" />
					<Input bind:value={query} placeholder="Search settings" class="h-9 ps-8" aria-label="Search settings" />
				</div>
			</div>

			{#if visible("appearance")}
				<section class="settings-section">
					{@render header("Appearance", sections[0].keys)}
					<SettingRow label="Theme" keywords="dark light mode">
						<Segmented
							label="Theme"
							value={userPrefersMode.current}
							options={[
								{ value: "system", label: "System" },
								{ value: "light", label: "Light" },
								{ value: "dark", label: "Dark" },
							]}
							onChange={(m) => setMode(m)}
						/>
					</SettingRow>
					<SettingRow label="Palette" keywords="colors accent" stacked>
						<div class="grid w-full gap-2 sm:grid-cols-2">
							{#each palettes as p (p.id)}
								<button
									class="border-border flex flex-col gap-2 rounded-lg border p-3 text-start {prefs.palette === p.id
										? 'border-primary ring-primary/30 ring-2'
										: 'hover:bg-accent/50'}"
									onclick={() => prefs.setPalette(p.id)}
								>
									<div class="flex gap-1">
										{#each p.swatch as c (c)}<span class="size-5 rounded" style:background={c}></span>{/each}
									</div>
									<div class="font-medium">{p.name}</div>
									<div class="text-muted-foreground text-xs">{p.note}</div>
								</button>
							{/each}
						</div>
					</SettingRow>
					<SettingRow label="Interface size" description="Scales the whole window ({mod}+ and {mod}− work too)." keywords="zoom font text scale">
						<div class="flex items-center gap-1">
							<Button variant="outline" size="icon-sm" aria-label="Smaller" disabled={prefs.uiScale <= 0.75} onclick={() => prefs.zoom(-0.05)}><Minus /></Button>
							<button class="w-14 text-center text-xs tabular-nums" title="Reset" onclick={() => prefs.zoom(0)}>{Math.round(prefs.uiScale * 100)}%</button>
							<Button variant="outline" size="icon-sm" aria-label="Larger" disabled={prefs.uiScale >= 1.5} onclick={() => prefs.zoom(0.05)}><Plus /></Button>
						</div>
					</SettingRow>
					<SettingRow label="Density" description="Spacing between messages, text size of replies and tab height." keywords="compact comfortable spacing">
						<Segmented
							label="Density"
							value={prefs.density}
							options={[
								{ value: "comfortable", label: "Comfortable" },
								{ value: "compact", label: "Compact" },
							]}
							onChange={(v) => prefs.set("density", v)}
						/>
					</SettingRow>
					<SettingRow label="Message width" description="How wide conversations are on large screens." keywords="column">
						<Segmented
							label="Message width"
							value={prefs.messageWidth}
							options={[
								{ value: "narrow", label: "Narrow" },
								{ value: "default", label: "Default" },
								{ value: "wide", label: "Wide" },
								{ value: "full", label: "Full" },
							]}
							onChange={(v) => prefs.set("messageWidth", v)}
						/>
					</SettingRow>
					<SettingRow label="Code font" description="Code blocks, diffs and paths." keywords="monospace">
						<Choice label="Code font" value={prefs.codeFont} options={fonts} onChange={(v) => prefs.set("codeFont", v)} />
						{#if prefs.codeFont === "custom"}
							<Input class="h-9 w-40" placeholder="Font name" value={prefs.codeFontCustom} onchange={(e) => prefs.set("codeFontCustom", e.currentTarget.value)} />
						{/if}
					</SettingRow>
				</section>
			{/if}

			{#if visible("new")}
				<section class="settings-section">
					{@render header("New conversations", [])}
					<SettingRow label="Remember my last choices" description="Each new conversation's tool, account, model, effort and permissions become the defaults below.">
						<Switch checked={nc.remember} onCheckedChange={(v) => setNc((d) => (d.remember = v))} aria-label="Remember my last choices" />
					</SettingRow>
					<SettingRow label="Default tool">
						<Segmented
							label="Default tool"
							value={nc.provider}
							options={[
								{ value: "claude", label: "Claude Code" },
								{ value: "codex", label: "Codex" },
							]}
							onChange={(v) => setNc((d) => (d.provider = v))}
						/>
					</SettingRow>
					{#each ["claude", "codex"] as const as p (p)}
						{@const name = p === "claude" ? "Claude" : "Codex"}
						<SettingRow label="{name} account" keywords="default {p}">
							<Choice label="{name} account" value={nc.account[p]} options={accountOptions(p)} onChange={(v) => setNc((d) => (d.account[p] = v))} />
						</SettingRow>
						<SettingRow label="{name} model" keywords="default {p}">
							<Choice label="{name} model" value={nc.model[p]} options={modelOptions(p)} onChange={(v) => setNc((d) => (d.model[p] = v))} />
						</SettingRow>
						<SettingRow label="{name} effort" keywords="reasoning thinking {p}">
							<Choice label="{name} effort" value={nc.effort[p]} options={efforts[p]} onChange={(v) => setNc((d) => (d.effort[p] = v))} />
						</SettingRow>
						<SettingRow label="{name} permissions" keywords="mode approval {p}">
							<Choice label="{name} permissions" class="w-52" value={nc.mode[p]} options={permissionModes(p)} onChange={(v) => setNc((d) => (d.mode[p] = v))} />
						</SettingRow>
					{/each}
					<SettingRow label="Send with" description="The other key adds a new line." keywords="enter return composer">
						<Segmented
							label="Send with"
							value={prefs.sendKey}
							options={[
								{ value: "enter", label: "↵ Enter" },
								{ value: "mod-enter", label: `${mod}↵` },
							]}
							onChange={(v) => prefs.set("sendKey", v)}
						/>
					</SettingRow>
				</section>
			{/if}

			{#if visible("conversations")}
				<section class="settings-section">
					{@render header("Conversations", sections[2].keys)}
					<SettingRow
						label="Use my Claude statusline script"
						description="Runs the statusLine.command from your Claude settings.json with the input Claude Code gives it, and shows its output under Claude conversations."
					>
						<Switch checked={prefs.statuslineScript} onCheckedChange={(v) => prefs.set("statuslineScript", v)} aria-label="Use my Claude statusline script" />
					</SettingRow>
					<SettingRow label="Progress panel" description="Turn state, plan and current tool beside each conversation.">
						<Switch checked={prefs.progressPanel} onCheckedChange={(v) => prefs.set("progressPanel", v)} aria-label="Progress panel" />
					</SettingRow>
					<SettingRow label="Open file links with" description="Paths in replies (like src/main.rs:12) open here, at the line." keywords="editor vscode cursor zed">
						<Choice label="Editor" value={prefs.editor} options={editors.map((e) => ({ value: e, label: editorNames[e] }))} onChange={(v) => prefs.set("editor", v)} />
					</SettingRow>
				</section>
			{/if}

			{#if visible("background")}
				<section class="settings-section">
					{@render header("Background & alerts", sections[3].keys)}
					<SettingRow
						label="Keep agents working after quitting"
						description="Agents finish what they are doing (and queued messages and scheduled resumes) after you quit Glaude. Off: quitting stops them."
						keywords="host background quit"
					>
						<Switch checked={prefs.keepAgents} onCheckedChange={(v) => prefs.set("keepAgents", v)} aria-label="Keep agents working after quitting" />
					</SettingRow>
					<SettingRow label="Stop idle agents after" description="An idle conversation's process stops; your next message resumes it." keywords="memory timeout">
						<Choice
							label="Stop idle agents after"
							value={prefs.idleMinutes}
							options={[
								{ value: 5, label: "5 minutes" },
								{ value: 15, label: "15 minutes" },
								{ value: 30, label: "30 minutes" },
								{ value: 60, label: "1 hour" },
								{ value: 0, label: "Never" },
							]}
							onChange={(v) => prefs.set("idleMinutes", v)}
						/>
					</SettingRow>
					<SettingRow label="When an agent finishes" description="Only when you can't see that conversation." keywords="notification done">
						<Switch checked={prefs.notifyDone} onCheckedChange={(v) => prefs.set("notifyDone", v)} aria-label="Notify when an agent finishes" />
					</SettingRow>
					<SettingRow label="When an agent needs you" description="Permission requests, questions and plans to review." keywords="notification permission question">
						<Switch checked={prefs.notifyWaiting} onCheckedChange={(v) => prefs.set("notifyWaiting", v)} aria-label="Notify when an agent needs you" />
					</SettingRow>
					<SettingRow label="When a turn fails" keywords="notification error">
						<Switch checked={prefs.notifyErrors} onCheckedChange={(v) => prefs.set("notifyErrors", v)} aria-label="Notify when a turn fails" />
					</SettingRow>
					<SettingRow label="While Glaude is closed" description="Agents still working in the background tell you when they finish or need you." keywords="notification host quit">
						<Switch checked={prefs.notifyClosed} onCheckedChange={(v) => prefs.set("notifyClosed", v)} aria-label="Notify while Glaude is closed" />
					</SettingRow>
					<SettingRow label="Play a sound" keywords="notification audio">
						<Switch checked={prefs.notifySound} onCheckedChange={(v) => prefs.set("notifySound", v)} aria-label="Play a sound" />
					</SettingRow>
				</section>
			{/if}

			{#if visible("terminal")}
				<section class="settings-section">
					{@render header("Terminal", sections[4].keys)}
					<SettingRow label="Font" keywords="terminal monospace">
						<Choice label="Terminal font" value={prefs.termFont} options={fonts} onChange={(v) => prefs.set("termFont", v)} />
						{#if prefs.termFont === "custom"}
							<Input class="h-9 w-40" placeholder="Font name" value={prefs.termFontCustom} onchange={(e) => prefs.set("termFontCustom", e.currentTarget.value)} />
						{/if}
					</SettingRow>
					<SettingRow label="Font size" keywords="terminal text">
						<Choice
							label="Terminal font size"
							class="w-24"
							value={prefs.termFontSize}
							options={[11, 12, 13, 14, 15, 16, 18].map((n) => ({ value: n, label: `${n} px` }))}
							onChange={(v) => prefs.set("termFontSize", v)}
						/>
					</SettingRow>
					<SettingRow label="Cursor" keywords="terminal caret">
						<Segmented
							label="Cursor style"
							value={prefs.termCursor}
							options={[
								{ value: "block", label: "Block" },
								{ value: "bar", label: "Bar" },
								{ value: "underline", label: "Underline" },
							]}
							onChange={(v) => prefs.set("termCursor", v)}
						/>
					</SettingRow>
					<SettingRow label="Blinking cursor" keywords="terminal caret">
						<Switch checked={prefs.termCursorBlink} onCheckedChange={(v) => prefs.set("termCursorBlink", v)} aria-label="Blinking cursor" />
					</SettingRow>
					<SettingRow label="Shell" description="Leave empty for your login shell. Used by new terminals." keywords="zsh bash fish pwsh">
						<Input
							class="h-9 w-52 font-mono text-xs"
							dir="ltr"
							placeholder={isMac ? "/bin/zsh" : "/bin/bash"}
							value={prefs.termShell}
							onchange={(e) => prefs.set("termShell", e.currentTarget.value.trim())}
							aria-label="Shell"
						/>
					</SettingRow>
				</section>
			{/if}

			{#if visible("keys")}
				<section class="settings-section">
					{@render header("Keyboard shortcuts", [])}
					{#each shortcuts as [what, keys] (what)}
						<SettingRow label={what} keywords="shortcut key">
							<kbd class="bg-muted text-muted-foreground rounded px-1.5 py-0.5 font-mono text-[11px]">{keys}</kbd>
						</SettingRow>
					{/each}
				</section>
			{/if}
		</div>
	</div>
</div>

<Sheet.Root bind:open={sheetOpen}>
	<Sheet.Content side="bottom" class="gap-2 px-3 pb-4">
		<Sheet.Header class="px-1">
			<Sheet.Title>Settings</Sheet.Title>
		</Sheet.Header>
		{@render nav()}
	</Sheet.Content>
</Sheet.Root>

<style>
	/* While searching, sections with no matching setting disappear. */
	.settings-section:not(:has(:global(.setting-row))) {
		display: none;
	}
</style>
