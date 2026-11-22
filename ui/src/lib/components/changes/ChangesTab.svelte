<script lang="ts">
	// Changes of a project: uncommitted, the branch vs its base, or one
	// commit; notes on lines go to a conversation as a review.
	import { call } from "$lib/ipc";
	import type { FileDiff, ReviewNote } from "$lib/types";
	import { filterDiffs, type Commit, type DiffSource, type GitInfo, type PullRequest } from "$lib/state/git";
	import { conversations } from "$lib/state/conversations.svelte";
	import { convActions, openLink, showError } from "$lib/state/sessions";
	import { layout } from "$lib/state/layout.svelte";
	import { fmtAgo, basename } from "$lib/format";
	import { toast } from "svelte-sonner";
	import DiffView from "$lib/components/conv/DiffView.svelte";
	import ResponsiveDialog from "$lib/components/ResponsiveDialog.svelte";
	import CommitDialog from "./CommitDialog.svelte";
	import PrDialog from "./PrDialog.svelte";
	import * as Tabs from "$lib/components/ui/tabs";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Textarea } from "$lib/components/ui/textarea";
	import GitBranch from "@lucide/svelte/icons/git-branch";
	import GitCommit from "@lucide/svelte/icons/git-commit-horizontal";
	import ScanEye from "@lucide/svelte/icons/scan-eye";
	import GitPullRequest from "@lucide/svelte/icons/git-pull-request";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import Search from "@lucide/svelte/icons/search";
	import Upload from "@lucide/svelte/icons/upload";
	import Send from "@lucide/svelte/icons/send";
	import Trash from "@lucide/svelte/icons/trash-2";
	import ArrowUp from "@lucide/svelte/icons/arrow-up";
	import ArrowDown from "@lucide/svelte/icons/arrow-down";
	import FolderGit from "@lucide/svelte/icons/folder-git-2";

	let { cwd, active }: { cwd: string; active: boolean } = $props();

	let info = $state<GitInfo | null>(null);
	let error = $state<string | null>(null);
	let source = $state<DiffSource>("uncommitted");
	let diffs = $state.raw<FileDiff[] | null>(null);
	let commits = $state.raw<Commit[] | null>(null);
	let picked = $state<string | null>(null);
	let filter = $state("");
	let pr = $state<PullRequest | null>(null);
	let notes = $state<ReviewNote[]>([]);
	let noteAt = $state<{ path: string; side: "a" | "b"; n: number; code: string } | null>(null);
	let noteText = $state("");
	let noteOpen = $state(false);
	let commitOpen = $state(false);
	let prOpen = $state(false);
	let removeOpen = $state(false);
	let pushing = $state(false);
	let loadSeq = 0;

	const root = $derived(info?.root ?? cwd);
	// Conversations working in this repository (newest activity first).
	const targets = $derived(
		Object.values(conversations.byId)
			.filter((c) => c.cwd === root || c.cwd.startsWith(root + "/") || root.startsWith(c.cwd + "/") || c.cwd === cwd)
			.sort((a, b) => b.lastActivity - a.lastActivity),
	);
	const account = $derived(targets.find((c) => c.provider === "claude")?.account);
	const shown = $derived(diffs ? filterDiffs(diffs, filter) : null);
	const totals = $derived(
		(shown ?? []).reduce((t, d) => ({ added: t.added + d.added, removed: t.removed + d.removed }), { added: 0, removed: 0 }),
	);

	async function loadInfo() {
		try {
			info = await call<GitInfo>("git_info", { cwd });
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			info = null;
		}
	}

	async function loadDiffs() {
		if (!info) return;
		const seq = ++loadSeq;
		try {
			let next: FileDiff[] | null = null;
			if (source === "uncommitted") next = await call<FileDiff[]>("git_diff", { cwd: root, source: "uncommitted" });
			else if (source === "branch") next = info.base ? await call<FileDiff[]>("git_diff", { cwd: root, source: "branch", rev: info.base }) : [];
			else {
				const list = await call<Commit[]>("git_log", { cwd: root, base: info.base ?? null, limit: 100 });
				if (seq !== loadSeq) return;
				commits = list;
				if (!picked || !list.some((c) => c.sha === picked)) picked = list[0]?.sha ?? null;
				next = picked ? await call<FileDiff[]>("git_diff", { cwd: root, source: "commit", rev: picked }) : [];
			}
			if (seq === loadSeq) diffs = next;
		} catch (e) {
			if (seq === loadSeq) {
				diffs = [];
				showError("Could not read the changes", e);
			}
		}
	}

	async function loadPr() {
		pr = null;
		if (!info?.gh || !info.upstream) return;
		pr = await call<PullRequest | null>("git_pr_status", { cwd: root }).catch(() => null);
	}

	async function refresh() {
		await loadInfo();
		await Promise.all([loadDiffs(), loadPr()]);
	}

	// Load when shown, and again when the window comes back into focus.
	$effect(() => {
		if (!active) return;
		void refresh();
		const onFocus = () => void refresh();
		window.addEventListener("focus", onFocus);
		return () => window.removeEventListener("focus", onFocus);
	});

	function pickSource(s: string) {
		source = s as DiffSource;
		diffs = null;
		void loadDiffs();
	}

	function pickCommit(sha: string) {
		picked = sha;
		diffs = null;
		void loadDiffs();
	}

	async function push() {
		pushing = true;
		try {
			const msg = await call<string>("git_push", { cwd: root });
			toast.success("Pushed", { description: msg });
		} catch (e) {
			showError("Push failed", e);
		} finally {
			pushing = false;
			void refresh();
		}
	}

	async function removeWorktree() {
		try {
			await call("worktree_remove", { path: root, force: false });
			toast.success("Worktree removed");
			const tab = Object.values(layout.tabs).find((t) => t.kind === "changes" && t.ref === cwd);
			if (tab) layout.closeTab(tab.id);
		} catch (e) {
			showError("Could not remove the worktree", e);
		}
	}

	function addNote() {
		if (!noteAt || !noteText.trim()) return;
		notes = [...notes, { id: `n${Date.now().toString(36)}${notes.length}`, ...noteAt, text: noteText.trim() }];
		noteOpen = false;
		noteText = "";
	}

	const sourceLabel = $derived(
		source === "uncommitted"
			? "the uncommitted changes"
			: source === "branch"
				? `the changes on ${info?.branch ?? "this branch"}`
				: `commit ${commits?.find((c) => c.sha === picked)?.short ?? ""}`,
	);

	function reviewText(): string {
		const lines = [`Review notes on ${sourceLabel}:`, ""];
		notes.forEach((n, i) => {
			lines.push(`${i + 1}. \`${n.path}:${n.n}\`${n.side === "a" ? " (removed line)" : ""}`);
			if (n.code.trim()) lines.push(`   > ${n.code.trim()}`);
			lines.push(`   ${n.text.replace(/\n/g, "\n   ")}`, "");
		});
		lines.push("Please address these.");
		return lines.join("\n");
	}

	async function sendReview(convId: string) {
		const conv = conversations.get(convId);
		if (!conv) return;
		const text = reviewText();
		if (conv.running || conv.queue.length) await convActions.enqueue(conv, text, []);
		else await convActions.send(conv, text, []);
		notes = [];
		const tab = Object.values(layout.tabs).find((t) => t.kind === "conversation" && t.ref === convId);
		if (tab) layout.activate(tab.id);
		toast.success(`Review sent to ${conv.title}`);
	}
</script>

<div class="flex h-full min-h-0 flex-col">
	{#if error}
		<div class="text-muted-foreground grid flex-1 place-items-center p-6 text-center text-[13px]">
			<div class="flex flex-col items-center gap-2">
				<FolderGit class="size-8 opacity-50" />
				<span>{error}</span>
				<span class="font-mono text-xs" dir="ltr">{cwd}</span>
			</div>
		</div>
	{:else if !info}
		<div class="flex flex-col gap-3 p-4">
			<div class="skeleton-shimmer h-8 w-1/2 rounded"></div>
			<div class="skeleton-shimmer h-40 rounded"></div>
		</div>
	{:else}
		<header class="border-border flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-4 py-2 text-[13px]">
			<span class="inline-flex min-w-0 items-center gap-1.5 font-medium">
				<GitBranch class="text-muted-foreground size-4 shrink-0" />
				<span class="truncate font-mono" dir="ltr">{info.branch}</span>
			</span>
			{#if info.worktree}<span class="bg-muted text-muted-foreground rounded px-1.5 py-0.5 text-[11px]">worktree</span>{/if}
			{#if info.upstream}
				<span class="text-muted-foreground inline-flex items-center gap-2 text-xs" title="Compared with {info.upstream}">
					{#if info.ahead}<span class="inline-flex items-center"><ArrowUp class="size-3" />{info.ahead}</span>{/if}
					{#if info.behind}<span class="inline-flex items-center"><ArrowDown class="size-3" />{info.behind}</span>{/if}
					{#if !info.ahead && !info.behind}up to date{/if}
				</span>
			{/if}
			<span class="ms-auto flex flex-wrap items-center gap-1.5">
				<Button variant="ghost" size="icon-sm" aria-label="Refresh" title="Refresh" onclick={refresh}><RefreshCw /></Button>
				{#if info.managed}
					<Button variant="ghost" size="sm" class="h-7" onclick={() => (removeOpen = true)}><Trash /> Remove worktree</Button>
				{/if}
				{#if targets[0] && info.dirty}
					<Button
						variant="outline"
						size="sm"
						class="h-7"
						title="A read-only review of the uncommitted changes, beside {targets[0].title}"
						onclick={() => void convActions.crossReview(targets[0])}
						><ScanEye /> Ask {targets[0].provider === "claude" ? "Codex" : "Claude"} to review</Button
					>
				{/if}
				<Button variant="outline" size="sm" class="h-7" disabled={!info.dirty} onclick={() => (commitOpen = true)}>
					<GitCommit /> Commit{info.dirty ? ` (${info.dirty})` : ""}
				</Button>
				{#if info.hasCommits && (info.ahead > 0 || !info.upstream)}
					<Button variant="outline" size="sm" class="h-7" disabled={pushing} onclick={push}><Upload /> {pushing ? "Pushing…" : "Push"}</Button>
				{/if}
				{#if pr}
					<Button variant="outline" size="sm" class="h-7" onclick={() => pr && openLink(pr.url)} title={pr.title}
						><GitPullRequest /> PR #{pr.number}</Button
					>
				{:else if info.gh && info.base && info.hasCommits}
					<Button size="sm" class="h-7" onclick={() => (prOpen = true)}><GitPullRequest /> Create PR</Button>
				{/if}
			</span>
		</header>

		<div class="flex flex-wrap items-center gap-2 px-4 pt-2">
			<Tabs.Root value={source} onValueChange={pickSource}>
				<Tabs.List class="h-8">
					<Tabs.Trigger value="uncommitted" class="text-xs">Uncommitted{info.dirty ? ` · ${info.dirty}` : ""}</Tabs.Trigger>
					<Tabs.Trigger value="branch" class="text-xs" disabled={!info.base}>Branch</Tabs.Trigger>
					<Tabs.Trigger value="commit" class="text-xs" disabled={!info.hasCommits}>Commits</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>
			<div class="relative ms-auto w-full max-w-64 min-w-40 flex-1 sm:flex-none">
				<Search class="text-muted-foreground absolute start-2 top-1/2 size-3.5 -translate-y-1/2" />
				<Input bind:value={filter} placeholder="Filter by path or text" class="h-8 ps-7 text-xs" aria-label="Filter changes" />
			</div>
		</div>
		{#if source === "branch" && info.base}
			<div class="text-muted-foreground px-4 pt-1.5 text-xs">Commits on <span class="font-mono">{info.branch}</span> since <span class="font-mono">{info.base}</span></div>
		{/if}

		<div class="flex min-h-0 flex-1 flex-col gap-3 md:flex-row">
			{#if source === "commit"}
				<div class="scrollbar-thin border-border max-h-48 shrink-0 overflow-y-auto border-b px-2 py-2 md:max-h-none md:w-72 md:border-e md:border-b-0">
					{#if commits === null}
						{#each [0, 1, 2] as i (i)}<div class="skeleton-shimmer mb-2 h-10 rounded"></div>{/each}
					{:else if commits.length === 0}
						<div class="text-muted-foreground p-3 text-xs">No commits{info.base ? ` since ${info.base}` : ""}.</div>
					{:else}
						{#each commits as c (c.sha)}
							<button
								class="hover:bg-accent flex w-full flex-col items-start rounded-md px-2 py-1.5 text-start {picked === c.sha ? 'bg-accent' : ''}"
								onclick={() => pickCommit(c.sha)}
							>
								<span class="line-clamp-2 text-[13px]" dir="auto">{c.subject}</span>
								<span class="text-muted-foreground text-[11px]"><span class="font-mono">{c.short}</span> · {c.author} · {fmtAgo(c.date)}</span>
							</button>
						{/each}
					{/if}
				</div>
			{/if}
			<div class="scrollbar-thin min-h-0 min-w-0 flex-1 overflow-y-auto px-4 pt-1 pb-4">
				{#if shown === null}
					<div class="flex flex-col gap-3 pt-2">
						{#each [0, 1] as i (i)}<div class="skeleton-shimmer h-32 rounded-md"></div>{/each}
					</div>
				{:else if shown.length === 0}
					<div class="text-muted-foreground py-16 text-center text-[13px]">
						{#if filter.trim()}Nothing matches “{filter}”.{:else if source === "uncommitted"}No uncommitted changes.{:else if source === "branch"}No commits on this branch since {info.base}.{:else}Pick a commit.{/if}
					</div>
				{:else}
					<div class="text-muted-foreground flex items-center gap-2 py-1.5 text-xs">
						{shown.length} {shown.length === 1 ? "file" : "files"}
						<span class="text-ok font-mono">+{totals.added}</span><span class="text-destructive font-mono">−{totals.removed}</span>
						<span class="ms-auto hidden sm:inline">Click a line number to leave a note</span>
					</div>
					<div class="flex flex-col gap-3">
						<DiffView
							diffs={shown}
							maxHeight="none"
							{notes}
							onLine={(path, side, n, code) => {
								noteAt = { path, side, n, code };
								noteText = "";
								noteOpen = true;
							}}
							onRemoveNote={(id) => (notes = notes.filter((x) => x.id !== id))}
						/>
					</div>
				{/if}
			</div>
		</div>

		{#if notes.length}
			<footer class="border-border bg-card flex flex-wrap items-center gap-2 border-t px-4 py-2 text-[13px]">
				<span class="font-medium">{notes.length} {notes.length === 1 ? "note" : "notes"}</span>
				<span class="text-muted-foreground text-xs">on {sourceLabel}</span>
				<Button variant="ghost" size="sm" class="ms-auto h-7" onclick={() => (notes = [])}>Clear</Button>
				{#if targets.length === 0}
					<span class="text-muted-foreground text-xs">Open a conversation in {basename(root)} to send the review.</span>
				{:else if targets.length === 1}
					<Button size="sm" class="h-7" onclick={() => sendReview(targets[0].id)}><Send /> Send to {targets[0].title}</Button>
				{:else}
					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button {...props} size="sm" class="h-7"><Send /> Send review</Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="end" class="w-64">
							{#each targets as c (c.id)}
								<DropdownMenu.Item onclick={() => sendReview(c.id)}>
									<span class="truncate" dir="auto">{c.title}</span>
									<span class="text-muted-foreground ms-auto text-[11px]">{c.provider === "codex" ? "Codex" : "Claude"}</span>
								</DropdownMenu.Item>
							{/each}
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				{/if}
			</footer>
		{/if}

		<CommitDialog bind:open={commitOpen} {root} {account} files={info.dirty} onDone={refresh} />
		<PrDialog bind:open={prOpen} {root} base={info.base} {account} onDone={refresh} />
	{/if}
</div>

<ResponsiveDialog
	bind:open={noteOpen}
	title={noteAt ? `Note on ${basename(noteAt.path)}:${noteAt.n}` : "Note"}
	description="Notes are sent together as one review."
>
	{#if noteAt?.code.trim()}
		<pre class="bg-muted overflow-x-auto rounded px-2 py-1 font-mono text-xs" dir="ltr">{noteAt.code}</pre>
	{/if}
	<Textarea
		bind:value={noteText}
		rows={4}
		placeholder="What should change here?"
		dir="auto"
		aria-label="Note"
		onkeydown={(e) => {
			if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
				e.preventDefault();
				addNote();
			}
		}}
	/>
	{#snippet actions()}
		<Button variant="ghost" onclick={() => (noteOpen = false)}>Cancel</Button>
		<Button disabled={!noteText.trim()} onclick={addNote}>Add note</Button>
	{/snippet}
</ResponsiveDialog>

<AlertDialog.Root bind:open={removeOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Remove this worktree?</AlertDialog.Title>
			<AlertDialog.Description>
				The folder <span class="font-mono">{root}</span> is deleted. Its branch <span class="font-mono">{info?.branch}</span> stays. Uncommitted changes
				block the removal.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Keep it</AlertDialog.Cancel>
			<AlertDialog.Action class="bg-destructive text-white hover:bg-destructive/90" onclick={removeWorktree}>Remove</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
