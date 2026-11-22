<script lang="ts">
	// Create or edit a project (sidebar): name, folders, look, and the
	// defaults new conversations in it start with.
	import ResponsiveDialog from "./ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import Choice from "./settings/Choice.svelte";
	import Segmented from "./settings/Segmented.svelte";
	import ProjectIcon, { PROJECT_ICONS, type ProjectIconId } from "./ProjectIcon.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { projects, PROJECT_COLORS, type Project, type ProjectDefaults } from "$lib/state/projects.svelte";
	import { proxies } from "$lib/state/proxies.svelte";
	import { history } from "$lib/state/history.svelte";
	import { accounts } from "$lib/state/accounts.svelte";
	import { permissionModes } from "$lib/state/sessions";
	import { claudeModels, efforts } from "$lib/state/newconv";
	import { call } from "$lib/ipc";
	import { basename } from "$lib/format";
	import { toast } from "svelte-sonner";
	import { untrack } from "svelte";
	import FolderPlus from "@lucide/svelte/icons/folder-plus";
	import X from "@lucide/svelte/icons/x";
	import Check from "@lucide/svelte/icons/check";
	import type { Provider } from "$lib/types";

	let open = $state(false);
	let id = $state<string | undefined>();
	let name = $state("");
	let folders = $state<string[]>([]);
	let color = $state<string | undefined>();
	let icon = $state<string | undefined>();
	let tool = $state<"any" | Provider>("any");
	let account = $state("");
	let model = $state("");
	let effort = $state("");
	let mode = $state("");
	let proxy = $state("");

	function nextColor(): string {
		const used = new Set(projects.list.map((p) => p.color));
		return (PROJECT_COLORS.find((c) => c.id !== "gray" && !used.has(c.id)) ?? PROJECT_COLORS[6]).id;
	}

	$effect(() => {
		const e = ui.projectEdit;
		if (!e) return;
		untrack(() => {
			if ("id" in e) {
				const p = e as Project;
				id = p.id;
				name = p.name;
				folders = [...p.folders];
				color = p.color;
				icon = p.icon;
				tool = p.defaults.provider ?? "any";
				account = p.defaults.account ?? "";
				model = p.defaults.model ?? "";
				effort = p.defaults.effort ?? "";
				mode = p.defaults.mode ?? "";
			} else {
				id = undefined;
				folders = e.folder ? [e.folder] : [];
				name = e.folder ? basename(e.folder) : "";
				color = nextColor();
				icon = undefined;
				tool = "any";
				account = model = effort = mode = "";
			}
			proxy = (folders[0] && proxies.s.byProject[folders[0]]) || "";
			open = true;
		});
		if (!accounts.list.length) void accounts.load();
		void proxies.ready();
	});
	$effect(() => {
		if (!open) ui.projectEdit = null;
	});

	// A folder may belong to one project only.
	const taken = $derived(
		new Map(projects.list.filter((p) => p.id !== id).flatMap((p) => p.folders.map((f) => [f, p.name] as [string, string]))),
	);
	const clash = $derived(folders.find((f) => taken.has(f)));
	const recent = $derived(history.groups.map((g) => g.cwd).filter((f) => !folders.includes(f) && !taken.has(f)));
	const valid = $derived(!!name.trim() && folders.length > 0 && !clash);

	async function addFolder() {
		try {
			const picked = await call<string | null>("pick_folder", { start: folders[0] ?? null });
			if (picked && !folders.includes(picked)) {
				folders = [...folders, picked];
				if (!name.trim()) name = basename(picked);
			}
		} catch (e) {
			toast.error("Could not open the folder picker", { description: String(e) });
		}
	}

	const accountOptions = $derived([
		{ value: "", label: "Last used" },
		...accounts.list.filter((a) => a.provider === tool && !a.archived).map((a) => ({ value: a.name, label: a.name })),
	]);
	const modelOptions = $derived(tool === "claude" ? claudeModels : [{ value: "", label: "Default" }]);
	const modeOptions = $derived(tool === "any" ? [] : [{ value: "", label: "Last used" }, ...permissionModes(tool)]);
	const proxyOptions = $derived([
		{ value: "", label: "Account or app default" },
		...proxies.list.map((p) => ({ value: p.id, label: p.name })),
		{ value: "none", label: "No proxy" },
	]);

	function save() {
		if (!valid) return;
		const defaults: ProjectDefaults =
			tool === "any"
				? {}
				: { provider: tool, account: account || undefined, model: model || undefined, effort: effort || undefined, mode: mode || undefined };
		const data = { name: name.trim(), folders, color, icon, defaults };
		if (id) projects.update(id, data);
		else projects.add(data);
		// The project's proxy is the Network setting of its folders.
		if (proxies.list.length) {
			const byProject = { ...proxies.s.byProject };
			for (const f of folders) {
				if (proxy) byProject[f] = proxy;
				else delete byProject[f];
			}
			void proxies.save({ ...proxies.s, byProject });
		}
		toast(id ? "Project saved" : `Added ${data.name}`);
		open = false;
	}
</script>

{#snippet field(label: string, value: string, options: { value: string; label: string }[], onChange: (v: string) => void)}
	<div class="flex min-w-0 flex-col gap-1">
		<span class="text-muted-foreground text-[12px]">{label}</span>
		<Choice {label} class="w-full" {value} {options} {onChange} />
	</div>
{/snippet}

<ResponsiveDialog bind:open title={id ? "Edit project" : "New project"} description="Its conversations are listed together in the sidebar.">
	<div class="flex flex-col gap-1.5">
		<Label for="project-name">Name</Label>
		<Input id="project-name" bind:value={name} placeholder="Shop" dir="auto" />
	</div>

	<div class="flex flex-col gap-1.5">
		<Label>Folders</Label>
		{#if folders.length}
			<ul class="border-border divide-border divide-y rounded-md border">
				{#each folders as f, i (f)}
					<li class="flex h-9 items-center gap-2 ps-3 pe-1">
						<span class="min-w-0 flex-1 truncate font-mono text-[12px]" dir="ltr" title={f}>{f}</span>
						{#if i === 0 && folders.length > 1}<span class="text-muted-foreground shrink-0 text-[11px]">new chats start here</span>{/if}
						{#if taken.has(f)}<span class="text-destructive shrink-0 text-[11px]">in {taken.get(f)}</span>{/if}
						<Button variant="ghost" size="icon-sm" aria-label="Remove {basename(f)}" onclick={() => (folders = folders.filter((x) => x !== f))}><X /></Button>
					</li>
				{/each}
			</ul>
		{/if}
		<div class="flex flex-wrap items-center gap-2">
			<Button variant="outline" size="sm" onclick={addFolder}><FolderPlus /> Add folder</Button>
			{#if recent.length}
				<Choice
					label="Add a recent folder"
					class="w-52"
					value={""}
					options={[{ value: "", label: "Add a recent folder…" }, ...recent.map((f) => ({ value: f, label: basename(f) }))]}
					onChange={(f) => {
						if (!f) return;
						folders = [...folders, f];
						if (!name.trim()) name = basename(f);
					}}
				/>
			{/if}
		</div>
	</div>

	<div class="flex flex-col gap-2">
		<Label>Look</Label>
		<div class="flex flex-wrap items-center gap-1.5" role="radiogroup" aria-label="Colour">
			{#each PROJECT_COLORS as c (c.id)}
				<button
					role="radio"
					aria-checked={color === c.id}
					aria-label={c.name}
					title={c.name}
					class="grid size-7 place-items-center rounded-full ring-offset-2 ring-offset-(--popover) {color === c.id ? 'ring-foreground/60 ring-2' : ''}"
					onclick={() => (color = c.id)}
				>
					<span class="grid size-5 place-items-center rounded-full" style:background={c.value}>{#if color === c.id}<Check class="size-3 text-white" />{/if}</span>
				</button>
			{/each}
		</div>
		<div class="flex flex-wrap items-center gap-1" role="radiogroup" aria-label="Icon">
			<button
				role="radio"
				aria-checked={!icon}
				class="hover:bg-accent grid size-8 place-items-center rounded-md border {!icon ? 'border-foreground/40 bg-accent' : 'border-transparent'}"
				title="Dot"
				aria-label="Dot"
				onclick={() => (icon = undefined)}><ProjectIcon project={{ color }} /></button
			>
			{#each Object.keys(PROJECT_ICONS) as k (k)}
				<button
					role="radio"
					aria-checked={icon === k}
					class="hover:bg-accent grid size-8 place-items-center rounded-md border {icon === k ? 'border-foreground/40 bg-accent' : 'border-transparent'}"
					title={k}
					aria-label={k}
					onclick={() => (icon = k as ProjectIconId)}><ProjectIcon project={{ color, icon: k }} class="size-4" /></button
				>
			{/each}
		</div>
	</div>

	<div class="flex flex-col gap-2">
		<Label>New conversations</Label>
		<Segmented
			label="Tool"
			value={tool}
			options={[
				{ value: "any", label: "Last used" },
				{ value: "claude", label: "Claude" },
				{ value: "codex", label: "Codex" },
			]}
			onChange={(v) => {
				tool = v;
				account = model = effort = mode = "";
			}}
		/>
		{#if tool !== "any"}
			<div class="grid grid-cols-2 gap-x-3 gap-y-2">
				{@render field("Account", account, accountOptions, (v) => (account = v))}
				{@render field("Model", model, modelOptions, (v) => (model = v))}
				{@render field("Effort", effort, efforts[tool], (v) => (effort = v))}
				{@render field("Permissions", mode, modeOptions, (v) => (mode = v))}
			</div>
		{/if}
		{#if proxies.list.length}
			<div class="flex items-center gap-2">
				<span class="text-muted-foreground text-[13px]">Proxy</span>
				<Choice label="Proxy" class="ms-auto w-56" value={proxy} options={proxyOptions} onChange={(v) => (proxy = v)} />
			</div>
		{/if}
	</div>

	{#snippet actions()}
		{#if clash}<p class="text-destructive me-auto self-center text-[12px]">{basename(clash)} is already in {taken.get(clash)}.</p>{/if}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button disabled={!valid} onclick={save}>{id ? "Save" : "Create project"}</Button>
	{/snippet}
</ResponsiveDialog>
