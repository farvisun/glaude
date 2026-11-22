<script lang="ts">
	import ResponsiveDialog from "$lib/components/ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Textarea } from "$lib/components/ui/textarea";
	import { call } from "$lib/ipc";
	import { toast } from "svelte-sonner";
	import { showError } from "$lib/state/sessions";
	import Sparkles from "@lucide/svelte/icons/sparkles";

	let {
		open = $bindable(false),
		root,
		account,
		files,
		onDone,
	}: { open: boolean; root: string; account?: string; files: number; onDone: () => void } = $props();

	let message = $state("");
	let busy = $state<"" | "generate" | "commit" | "push">("");

	async function generate() {
		busy = "generate";
		try {
			message = await call<string>("git_message", { cwd: root, account: account ?? null, pr: false });
		} catch (e) {
			showError("Could not write a message", e);
		} finally {
			busy = "";
		}
	}

	async function commit(push: boolean) {
		busy = push ? "push" : "commit";
		try {
			const sha = await call<string>("git_commit", { cwd: root, message });
			if (push) {
				await call<string>("git_push", { cwd: root });
				toast.success(`Committed ${sha} and pushed`);
			} else toast.success(`Committed ${sha}`);
			message = "";
			open = false;
			onDone();
		} catch (e) {
			showError(push ? "Commit and push failed" : "Commit failed", e);
			onDone();
		} finally {
			busy = "";
		}
	}
</script>

<ResponsiveDialog bind:open title="Commit" description="All {files} changed {files === 1 ? 'file' : 'files'} are staged and committed.">
	<div class="relative">
		<Textarea
			bind:value={message}
			rows={6}
			placeholder="feat: what changed"
			class="font-mono text-[13px]"
			dir="auto"
			aria-label="Commit message"
			onkeydown={(e) => {
				if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && message.trim()) {
					e.preventDefault();
					void commit(false);
				}
			}}
		/>
	</div>
	<div>
		<Button variant="outline" size="sm" disabled={!!busy} onclick={generate}>
			<Sparkles class={busy === "generate" ? "animate-pulse" : ""} />
			{busy === "generate" ? "Writing…" : "Write it with Claude"}
		</Button>
		<span class="text-muted-foreground ms-2 text-xs">Uses a quick Haiku call on your account.</span>
	</div>
	{#snippet actions()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button variant="secondary" disabled={!message.trim() || !!busy} onclick={() => commit(true)}>{busy === "push" ? "Pushing…" : "Commit & push"}</Button>
		<Button disabled={!message.trim() || !!busy} onclick={() => commit(false)}>{busy === "commit" ? "Committing…" : "Commit"}</Button>
	{/snippet}
</ResponsiveDialog>
