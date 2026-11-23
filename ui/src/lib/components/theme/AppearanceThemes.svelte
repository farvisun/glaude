<script lang="ts">
	// Settings → Appearance: mode, the light and dark theme, accent, the chat
	// and terminal themes, and the user's own themes (new, import, edit).
	import SettingRow from "../settings/SettingRow.svelte";
	import Segmented from "../settings/Segmented.svelte";
	import ThemeCard from "./ThemeCard.svelte";
	import ThemeSelect from "./ThemeSelect.svelte";
	import AccentPicker from "./AccentPicker.svelte";
	import { Button } from "$lib/components/ui/button";
	import { setMode, userPrefersMode, mode } from "mode-watcher";
	import { prefs } from "$lib/state/prefs.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { themes } from "$lib/themes/themes.svelte";
	import { FAMILIES } from "$lib/themes/presets";
	import { parseThemeFile, ThemeImportError } from "$lib/themes/import";
	import type { Kind, Theme } from "$lib/themes/types";
	import { toast } from "svelte-sonner";
	import Plus from "@lucide/svelte/icons/plus";
	import Upload from "@lucide/svelte/icons/upload";

	const order = (t: Theme) => (t.custom ? 99 : FAMILIES.indexOf(t.family));
	const byKind = (k: Kind) => themes.all.filter((t) => t.kind === k).sort((a, b) => order(a) - order(b));
	const light = $derived(byKind("light"));
	const dark = $derived(byKind("dark"));

	function choose(t: Theme) {
		prefs.set(t.kind === "dark" ? "darkTheme" : "lightTheme", t.id);
		// Show it: switch an explicit mode; with System, say when it applies.
		const m = userPrefersMode.current;
		if (m !== "system" && m !== t.kind) setMode(t.kind);
		else if (m === "system" && mode.current !== t.kind)
			toast(`${t.name} is used when your system is in ${t.kind} mode`, { action: { label: `Use ${t.kind} now`, onClick: () => setMode(t.kind) } });
	}

	function customize(t: Theme) {
		ui.themeEdit = themes.draft(t);
	}

	async function copyJson(t: Theme) {
		const { id: _id, custom: _c, vars: _v, ...rest } = $state.snapshot(t) as Theme;
		try {
			await navigator.clipboard.writeText(JSON.stringify(rest, null, 2));
			toast(`Copied ${t.name} as JSON`, { description: "Import it on another Mac, or share it." });
		} catch {
			toast.error("Couldn't copy to the clipboard");
		}
	}

	function remove(t: Theme) {
		const was = themes.remove(t.id);
		if (was) toast(`Deleted ${t.name}`, { action: { label: "Undo", onClick: () => themes.restore(was) } });
	}

	let fileInput = $state<HTMLInputElement>();
	async function onFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = "";
		if (!file) return;
		if (file.size > 2_000_000) return void toast.error("That file is too big for a theme");
		try {
			const t = parseThemeFile(file.name, await file.text());
			ui.themeEdit = { ...t, id: "", family: "Yours", custom: true };
		} catch (err) {
			toast.error("Couldn't import the theme", { description: err instanceof ThemeImportError ? err.message : String(err) });
		}
	}
</script>

{#snippet gallery(list: Theme[], selectedId: string)}
	<div class="grid w-full grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-2">
		{#each list as t (t.id)}
			<ThemeCard
				theme={t}
				selected={t.id === selectedId}
				onSelect={() => choose(t)}
				onCustomize={() => customize(t)}
				onEdit={t.custom ? () => (ui.themeEdit = $state.snapshot(t) as Theme) : undefined}
				onCopyJson={() => copyJson(t)}
				onDelete={t.custom ? () => remove(t) : undefined}
			/>
		{/each}
	</div>
{/snippet}

<SettingRow label="Mode" description="System follows macOS: the light theme by day, the dark theme at night." keywords="theme dark light system appearance">
	<Segmented
		label="Mode"
		value={userPrefersMode.current}
		options={[
			{ value: "system", label: "System" },
			{ value: "light", label: "Light" },
			{ value: "dark", label: "Dark" },
		]}
		onChange={(m) => setMode(m)}
	/>
</SettingRow>
{#snippet darkRow()}
	<SettingRow label="Dark theme" keywords="theme color scheme palette dracula nord tokyo night one dark github solarized catppuccin mocha rose pine gruvbox everforest sade lajevard dood" stacked>
		{@render gallery(dark, prefs.darkTheme)}
	</SettingRow>
{/snippet}
{#snippet lightRow()}
	<SettingRow label="Light theme" keywords="theme color scheme palette github one light solarized catppuccin latte rose pine dawn gruvbox everforest sade lajevard dood" stacked>
		{@render gallery(light, prefs.lightTheme)}
	</SettingRow>
{/snippet}
<!-- The gallery of the mode in use comes first. -->
{#if mode.current === "dark"}{@render darkRow()}{@render lightRow()}{:else}{@render lightRow()}{@render darkRow()}{/if}
<SettingRow label="Accent" description="Buttons, focus and the terminal cursor. The theme's own, or a colour you pick." keywords="accent color colour primary">
	<AccentPicker />
</SettingRow>
<SettingRow label="Chat theme" description="Conversations can use another theme, for example a dark chat in a light window." keywords="chat conversation dark theme color">
	<ThemeSelect label="Chat theme" value={prefs.chatTheme} onChange={(v) => prefs.set("chatTheme", v)} />
</SettingRow>
<SettingRow label="Terminal theme" description="Terminal tabs and the terminal panel, with their 16 colours." keywords="terminal dark theme color ansi">
	<ThemeSelect label="Terminal theme" value={prefs.termTheme} onChange={(v) => prefs.set("termTheme", v)} />
</SettingRow>
<SettingRow
	label="Your themes"
	description="Start from the current theme, or import a VS Code theme (.json), an iTerm2 scheme (.itermcolors), or a Ghostty, kitty or Windows Terminal scheme. Yours show up in the lists above."
	keywords="custom theme import vscode iterm ghostty kitty windows terminal editor colors"
>
	<Button variant="outline" size="sm" onclick={() => customize(themes.app)}><Plus /> New theme</Button>
	<Button variant="outline" size="sm" onclick={() => fileInput?.click()}><Upload /> Import…</Button>
	<input bind:this={fileInput} type="file" class="hidden" onchange={onFile} aria-hidden="true" tabindex="-1" />
</SettingRow>
