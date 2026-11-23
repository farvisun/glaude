<script lang="ts">
	// Make or change one of the user's themes (also where imports land).
	import ResponsiveDialog from "../ResponsiveDialog.svelte";
	import Segmented from "../settings/Segmented.svelte";
	import ThemePreview from "./ThemePreview.svelte";
	import ColorField from "./ColorField.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { ui } from "$lib/state/ui.svelte";
	import { prefs } from "$lib/state/prefs.svelte";
	import { themes } from "$lib/themes/themes.svelte";
	import { themeVars } from "$lib/themes/vars";
	import { ANSI_NAMES, SYNTAX_KEYS, type Kind, type SyntaxKey, type Theme } from "$lib/themes/types";
	import { setMode, userPrefersMode } from "mode-watcher";
	import { toast } from "svelte-sonner";

	let open = $state(false);
	let draft = $state<Theme | null>(null);
	$effect(() => {
		const t = ui.themeEdit;
		if (t) {
			draft = JSON.parse(JSON.stringify(t)) as Theme;
			open = true;
		}
	});
	$effect(() => {
		if (!open) ui.themeEdit = null;
	});

	// What "auto" fields come out as, shown in their swatches.
	const computed = $derived<Record<string, string>>(draft ? themeVars(draft, "theme") : {});
	const auto = (k: string) => computed[`--${k}`]?.startsWith("#") ? computed[`--${k}`] : undefined;
	const isNew = $derived(!!draft && !draft.id);

	const surfaces: [keyof Theme, string, string | null][] = [
		["bg", "Background", null],
		["fg", "Text", null],
		["muted", "Secondary text", "muted-foreground"],
		["card", "Cards", "card"],
		["popover", "Menus", "popover"],
		["sidebar", "Sidebar", "sidebar"],
	];
	const status: [keyof Theme, string][] = [
		["accent", "Accent"],
		["red", "Errors"],
		["green", "Success"],
		["yellow", "Needs attention"],
		["blue", "Links"],
	];
	const synLabels: Record<SyntaxKey, string> = {
		comment: "Comments",
		keyword: "Keywords",
		string: "Strings",
		number: "Numbers",
		function: "Functions",
		type: "Types",
		variable: "Parameters",
		tag: "Tags",
		attribute: "Attributes",
		operator: "Operators",
	};
	const ansiLabel = (n: string) => n.replace(/^bright(\w)/, (_, c) => `Bright ${c.toLowerCase()}`).replace(/^\w/, (c) => c.toUpperCase());

	function set<K extends keyof Theme>(k: K, v: Theme[K]) {
		if (draft) draft[k] = v;
	}

	function save() {
		if (!draft) return;
		const name = draft.name.trim();
		if (!name) return void toast.error("Give the theme a name");
		const saved = themes.put({ ...$state.snapshot(draft), name } as Theme);
		// Use it right away in its mode.
		prefs.set(saved.kind === "dark" ? "darkTheme" : "lightTheme", saved.id);
		if (userPrefersMode.current !== "system" && userPrefersMode.current !== saved.kind) setMode(saved.kind);
		toast(isNew ? `Added ${saved.name}` : `Saved ${saved.name}`);
		open = false;
	}

	function remove() {
		if (!draft?.id) return;
		const was = themes.remove(draft.id);
		open = false;
		if (was) toast(`Deleted ${was.name}`, { action: { label: "Undo", onClick: () => themes.restore(was) } });
	}
</script>

<ResponsiveDialog bind:open title={isNew ? "New theme" : "Edit theme"} description="Colours for the window, code and terminals. Empty fields follow from the others." class="sm:max-w-2xl">
	{#if draft}
		<!-- The preview stays in view while the colours below scroll. -->
		<div class="flex max-h-[min(74vh,48rem)] min-h-0 flex-col gap-3">
			<ThemePreview theme={draft} size="lg" />
			<div class="grid shrink-0 grid-cols-[1fr_auto] items-end gap-3">
				<div class="flex flex-col gap-1.5">
					<Label for="theme-name">Name</Label>
					<Input id="theme-name" bind:value={draft.name} placeholder="My theme" />
				</div>
				<Segmented
					label="Light or dark"
					value={draft.kind}
					options={[
						{ value: "light" as Kind, label: "Light" },
						{ value: "dark" as Kind, label: "Dark" },
					]}
					onChange={(k) => set("kind", k)}
				/>
			</div>
			<div class="-me-1 flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto pe-1 pb-1">
				<section class="flex flex-col gap-2">
					<h3 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Window</h3>
					<div class="grid gap-x-5 gap-y-2 sm:grid-cols-2">
						{#each surfaces as [k, label, autoKey] (k)}
							<ColorField
								{label}
								value={draft[k] as string | undefined}
								auto={autoKey ? auto(autoKey) : undefined}
								onChange={(v) => set(k, (v ?? (autoKey ? undefined : draft![k])) as never)}
							/>
						{/each}
					</div>
				</section>

				<section class="flex flex-col gap-2">
					<h3 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Accent and status</h3>
					<div class="grid gap-x-5 gap-y-2 sm:grid-cols-2">
						{#each status as [k, label] (k)}
							<ColorField {label} value={draft[k] as string} onChange={(v) => v && set(k, v as never)} />
						{/each}
					</div>
				</section>

				<section class="flex flex-col gap-2">
					<h3 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Code</h3>
					<div class="grid gap-x-5 gap-y-2 sm:grid-cols-2">
						<ColorField label="Code background" value={draft.codeBg} auto={auto("code-bg")} onChange={(v) => set("codeBg", v)} />
						{#each SYNTAX_KEYS as k (k)}
							<ColorField label={synLabels[k]} value={draft.syntax[k]} onChange={(v) => v && (draft!.syntax[k] = v)} />
						{/each}
					</div>
				</section>

				<section class="flex flex-col gap-2">
					<h3 class="text-muted-foreground text-[11px] font-semibold tracking-wide uppercase">Terminal</h3>
					<div class="grid gap-x-5 gap-y-2 sm:grid-cols-2">
						<ColorField label="Terminal background" value={draft.termBg} auto={auto("term-bg")} onChange={(v) => set("termBg", v)} />
						<div class="hidden sm:block"></div>
						{#each ANSI_NAMES as n, i (n)}
							<ColorField label={ansiLabel(n)} value={draft.ansi[i]} onChange={(v) => v && (draft!.ansi[i] = v)} />
						{/each}
					</div>
				</section>
			</div>
		</div>
	{/if}
	{#snippet actions()}
		{#if draft?.id}<Button variant="ghost" class="text-destructive me-auto" onclick={remove}>Delete</Button>{/if}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button onclick={save}>{isNew ? "Add theme" : "Save"}</Button>
	{/snippet}
</ResponsiveDialog>
