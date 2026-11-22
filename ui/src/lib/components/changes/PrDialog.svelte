<script lang="ts">
	import ResponsiveDialog from "$lib/components/ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Textarea } from "$lib/components/ui/textarea";
	import { Switch } from "$lib/components/ui/switch";
	import { call } from "$lib/ipc";
	import { toast } from "svelte-sonner";
	import { openLink, showError } from "$lib/state/sessions";
	import Sparkles from "@lucide/svelte/icons/sparkles";

	let {
		open = $bindable(false),
		root,
		base,
		account,
		onDone,
	}: { open: boolean; root: string; base?: string | null; account?: string; onDone: () => void } = $props();

	let title = $state("");
	let body = $state("");
	let draft = $state(false);
	let busy = $state<"" | "generate" | "create">("");

	async function generate() {
		busy = "generate";
		try {
			const text = await call<string>("git_message", { cwd: root, account: account ?? null, pr: true });
			const [first, ...rest] = text.split("\n");
			title = first.replace(/^#+\s*/, "").trim();
			body = rest.join("\n").trim();
		} catch (e) {
			showError("Could not write the description", e);
		} finally {
			busy = "";
		}
	}

	async function create() {
		busy = "create";
		try {
			const url = await call<string>("git_pr_create", { cwd: root, title, body, base: base ?? null, draft });
			toast.success("Pull request created", { action: { label: "Open", onClick: () => void openLink(url) } });
			open = false;
			title = "";
			body = "";
			onDone();
		} catch (e) {
			showError("Could not create the pull request", e);
		} finally {
			busy = "";
		}
	}
</script>

<ResponsiveDialog bind:open title="Create pull request" description={base ? `Into ${base.replace(/^origin\//, "")}. The branch is pushed first if needed.` : "The branch is pushed first if needed."}>
	<Input bind:value={title} placeholder="Title" aria-label="Title" dir="auto" />
	<Textarea bind:value={body} rows={8} placeholder="What changed and why" aria-label="Description" dir="auto" class="text-[13px]" />
	<div class="flex flex-wrap items-center gap-3">
		<Button variant="outline" size="sm" disabled={!!busy} onclick={generate}>
			<Sparkles class={busy === "generate" ? "animate-pulse" : ""} />
			{busy === "generate" ? "Writing…" : "Write it with Claude"}
		</Button>
		<label class="ms-auto flex items-center gap-2 text-[13px]"><Switch bind:checked={draft} /> Draft</label>
	</div>
	{#snippet actions()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button disabled={!title.trim() || !!busy} onclick={create}>{busy === "create" ? "Creating…" : "Create pull request"}</Button>
	{/snippet}
</ResponsiveDialog>
