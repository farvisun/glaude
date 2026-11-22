<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import { api, type AppInfo } from "$lib/ipc";
	import { ui } from "$lib/state/ui.svelte";
	import { isMac } from "$lib/state/prefs.svelte";
	import { Kbd } from "$lib/components/ui/kbd";
	import CircleCheck from "@lucide/svelte/icons/circle-check";
	import CircleAlert from "@lucide/svelte/icons/circle-alert";

	let info = $state<AppInfo | null>(null);

	onMount(async () => {
		try {
			info = await api.appInfo();
		} catch {
			info = null;
		}
	});

	const mod = isMac ? "⌘" : "Ctrl";
</script>

<div class="grid h-full place-items-center overflow-y-auto px-6">
	<div class="flex w-full max-w-md flex-col gap-6 py-10">
		<div>
			<h1 class="text-2xl font-bold tracking-tight">Talk to your agents</h1>
			<p class="text-muted-foreground mt-1.5 text-[14px] leading-relaxed">
				Claude Code and Codex, with Persian that reads right to left. Uses the CLIs and logins already on this computer.
			</p>
			<p class="text-muted-foreground mt-3 text-[15px] leading-loose" dir="rtl" lang="fa">
				گفتگو با Claude و Codex، با متن فارسی راست‌به‌چپ و خوانا.
			</p>
		</div>

		<div class="flex flex-col gap-2">
			{#each [{ key: "claude", label: "Claude Code" }, { key: "codex", label: "Codex" }] as t (t.key)}
				{@const tool = info?.[t.key as "claude" | "codex"]}
				<div class="border-border flex items-center gap-3 rounded-lg border px-3 py-2.5">
					<span class="size-2.5 rounded-full {t.key === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
					<span class="font-medium">{t.label}</span>
					{#if info === null}
						<span class="bg-muted ms-auto h-4 w-28 animate-pulse rounded"></span>
					{:else if tool?.path}
						<span class="text-muted-foreground ms-auto truncate text-xs" dir="ltr">{tool.version ?? tool.path}</span>
						<CircleCheck class="text-ok size-4 shrink-0" />
					{:else}
						<span class="text-muted-foreground ms-auto text-xs">Not found on PATH</span>
						<CircleAlert class="text-attention size-4 shrink-0" />
					{/if}
				</div>
			{/each}
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<Button onclick={() => ui.openNewConversation()}>New conversation</Button>
			<Button variant="ghost" onclick={() => ui.openTerminal()}>Open a terminal</Button>
		</div>

		<div class="text-muted-foreground flex flex-wrap gap-x-5 gap-y-1.5 text-xs">
			<span><Kbd>{mod} K</Kbd> search history</span>
			<span><Kbd>{mod} B</Kbd> sidebar</span>
			<span><Kbd>{mod} W</Kbd> close tab</span>
			<span>Drag a tab to an edge to split</span>
		</div>
	</div>
</div>
