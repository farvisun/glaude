<script lang="ts">
	import { prefs, type Palette } from "$lib/state/prefs.svelte";
	import { setMode, userPrefersMode } from "mode-watcher";
	import { Switch } from "$lib/components/ui/switch";

	const palettes: { id: Palette; name: string; note: string; swatch: string[] }[] = [
		{ id: "lajevard", name: "Lajevard", note: "Lapis ink with a turquoise accent", swatch: ["#101a2e", "#5cc8bd", "#f2b544", "#e4eaf4"] },
		{ id: "dood", name: "Dood", note: "Quiet graphite with an indigo accent", swatch: ["#18181b", "#9aa4ff", "#f5b041", "#e7e7ea"] },
	];
</script>

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-2xl flex-col gap-8 px-6 py-8">
		<h1 class="text-xl font-bold">Settings</h1>

		<section class="flex flex-col gap-3">
			<h2 class="font-semibold">Appearance</h2>
			<div class="flex gap-2">
				{#each ["system", "light", "dark"] as m (m)}
					<button
						class="border-border rounded-md border px-3 py-1.5 text-[13px] capitalize {userPrefersMode.current === m
							? 'border-primary bg-primary/10'
							: 'hover:bg-accent'}"
						onclick={() => setMode(m as "system" | "light" | "dark")}>{m}</button
					>
				{/each}
			</div>
			<div class="grid gap-2 sm:grid-cols-2">
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
		</section>

		<section class="flex flex-col gap-3">
			<h2 class="font-semibold">Conversations</h2>
			<label class="flex items-start justify-between gap-4 text-[13px]">
				<span>
					<span class="font-medium">Use my Claude statusline script</span>
					<span class="text-muted-foreground block">
						Runs the <code>statusLine.command</code> from your Claude settings.json with the same input Claude Code gives it, and shows its
						output under Claude conversations. Falls back to the built-in line if the script fails.
					</span>
				</span>
				<Switch
					checked={prefs.statuslineScript}
					onCheckedChange={(v) => {
						prefs.statuslineScript = v;
						prefs.save();
					}}
				/>
			</label>
			<label class="flex items-start justify-between gap-4 text-[13px]">
				<span>
					<span class="font-medium">Progress panel</span>
					<span class="text-muted-foreground block">Turn state, plan and current tool beside each conversation.</span>
				</span>
				<Switch
					checked={prefs.progressPanel}
					onCheckedChange={(v) => {
						prefs.progressPanel = v;
						prefs.save();
					}}
				/>
			</label>
		</section>
	</div>
</div>
