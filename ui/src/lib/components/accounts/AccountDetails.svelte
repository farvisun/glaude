<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import X from "@lucide/svelte/icons/x";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import SquareTerminal from "@lucide/svelte/icons/square-terminal";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import Link2 from "@lucide/svelte/icons/link-2";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Archive from "@lucide/svelte/icons/archive";
	import ArchiveRestore from "@lucide/svelte/icons/archive-restore";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import CircleCheck from "@lucide/svelte/icons/circle-check";
	import LimitBar from "$lib/components/conv/LimitBar.svelte";
	import { accounts, accountKey, type AccountView } from "$lib/state/accounts.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { fmtAgo, fmtTokens } from "$lib/format";

	let { account: a, onClose, sheet = false }: { account: AccountView; onClose: () => void; sheet?: boolean } = $props();

	const toolName = $derived(a.provider === "claude" ? "Claude Code" : "Codex");
	const scanning = $derived(accountKey(a) in accounts.progress);

	// Share: dry run first, then apply.
	let shareOpen = $state(false);
	let sharePlan = $state<string[] | null>(null);
	let shareResult = $state<string[] | null>(null);
	let sharing = $state(false);
	async function openShare() {
		shareOpen = true;
		sharePlan = null;
		shareResult = null;
		sharePlan = await accounts.share(a, true);
	}
	async function applyShare() {
		sharing = true;
		shareResult = await accounts.share(a, false);
		sharing = false;
	}

	let renameOpen = $state(false);
	let newName = $state("");
	async function doRename(e: SubmitEvent) {
		e.preventDefault();
		if (await accounts.rename(a, newName)) renameOpen = false;
	}

	let deleteOpen = $state(false);
	let confirmName = $state("");
	async function doDelete() {
		if (confirmName !== a.name) return;
		if (await accounts.remove(a)) {
			deleteOpen = false;
			onClose();
		}
	}

	function fmtDate(iso: string | null) {
		return iso ? new Date(iso).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "–";
	}
</script>

<div class="space-y-4 p-4 text-[13px]">
	<div class="flex items-start gap-2">
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				<span class="size-2 shrink-0 rounded-full {a.provider === 'claude' ? 'bg-claude' : 'bg-codex'}"></span>
				<h3 class="truncate text-[15px] font-semibold">{a.name}</h3>
			</div>
			<p class="text-muted-foreground mt-0.5 truncate">{toolName} · {a.path}</p>
		</div>
		{#if !sheet}
			<Button variant="ghost" size="icon-sm" aria-label="Close details" onclick={onClose}><X /></Button>
		{/if}
	</div>

	<dl class="grid grid-cols-[6.5rem_1fr] gap-x-3 gap-y-1.5">
		<dt class="text-muted-foreground">Email</dt>
		<dd class="truncate">{a.email || "–"}</dd>
		{#if a.org}<dt class="text-muted-foreground">Organization</dt><dd class="truncate">{a.org}</dd>{/if}
		<dt class="text-muted-foreground">Plan</dt>
		<dd>{a.plan || "–"}</dd>
		<dt class="text-muted-foreground">5h left</dt>
		<dd>{#if a.fiveHour}<LimitBar label="" win={a.fiveHour} wide />{:else}–{/if}</dd>
		<dt class="text-muted-foreground">Week left</dt>
		<dd>{#if a.week}<LimitBar label="" win={a.week} wide />{:else}–{/if}</dd>
		{#if a.limitsAsOf}<dt class="text-muted-foreground">Limits as of</dt><dd>{fmtAgo(a.limitsAsOf)}</dd>{/if}
		<dt class="text-muted-foreground">Tokens</dt>
		<dd>{fmtTokens(a.tokens)} · today {fmtTokens(a.tokensToday)} · 7d {fmtTokens(a.tokens7d)}</dd>
		<dt class="text-muted-foreground">Last used</dt>
		<dd>{fmtAgo(a.lastUsed)}</dd>
		<dt class="text-muted-foreground">Launches</dt>
		<dd>{a.launches}</dd>
		<dt class="text-muted-foreground">Created</dt>
		<dd>{fmtDate(a.createdAt)}</dd>
		{#if !a.protected}
			<dt class="text-muted-foreground">Shared home</dt>
			<dd>
				{#if a.sharedPending}<span class="text-attention">{a.sharedPending} item{a.sharedPending === 1 ? "" : "s"} not linked yet</span
					>{:else}All linked ({a.sharedLinked} items){/if}
			</dd>
		{/if}
		{#each a.details as d (d.label)}
			<dt class="text-muted-foreground">{d.label}</dt>
			<dd class="text-[12px] leading-relaxed">{d.text}</dd>
		{/each}
	</dl>
	{#if a.usageUpdated}<p class="text-muted-foreground text-[12px]">Usage scanned {fmtAgo(a.usageUpdated)}</p>{/if}

	<div class="flex flex-wrap gap-2">
		<Button size="sm" onclick={() => ui.openNewConversation(undefined, { provider: a.provider, account: a.name })}>
			<MessageSquare /> New conversation
		</Button>
		<Button
			size="sm"
			variant="outline"
			onclick={() => ui.openTerminal({ run: { provider: a.provider, account: a.name } }, `${a.provider} · ${a.name}`)}
		>
			<SquareTerminal /> Open {a.provider === "claude" ? "Claude" : "Codex"} in terminal
		</Button>
	</div>
	<div class="flex flex-wrap gap-1.5">
		<Button size="sm" variant="ghost" disabled={scanning} onclick={() => accounts.refresh([a])}>
			<RefreshCw class={scanning ? "animate-spin" : ""} /> Refresh usage
		</Button>
		{#if !a.active}
			<Button size="sm" variant="ghost" onclick={() => accounts.makeActive(a)}><CircleCheck /> Make active</Button>
		{/if}
		{#if !a.protected}
			<Button size="sm" variant="ghost" onclick={openShare}><Link2 /> Share…</Button>
			<Button
				size="sm"
				variant="ghost"
				onclick={() => {
					newName = a.name;
					renameOpen = true;
				}}><Pencil /> Rename…</Button
			>
		{/if}
		<Button size="sm" variant="ghost" onclick={() => accounts.archive(a, !a.archived)}>
			{#if a.archived}<ArchiveRestore /> Restore{:else}<Archive /> Archive{/if}
		</Button>
		{#if !a.protected}
			<Button
				size="sm"
				variant="ghost"
				class="text-destructive hover:text-destructive"
				disabled={a.running}
				title={a.running ? "Close its sessions first" : undefined}
				onclick={() => {
					confirmName = "";
					deleteOpen = true;
				}}><Trash2 /> Delete…</Button
			>
		{/if}
	</div>
</div>

<Dialog.Root bind:open={shareOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Share {a.name}'s home</Dialog.Title>
			<Dialog.Description>
				Links sessions, settings and skills to the central {a.provider === "claude" ? "~/.claude" : "~/.codex"}. Real files are merged first;
				anything replaced is kept in ~/.agent-accounts-backup.
			</Dialog.Description>
		</Dialog.Header>
		{#if shareResult}
			<ul class="bg-muted max-h-60 list-disc space-y-0.5 overflow-auto rounded-md py-2 ps-6 pe-3 text-[12px]">
				{#each shareResult.length ? shareResult : ["Already shared; nothing changed."] as line (line)}<li>{line}</li>{/each}
			</ul>
		{:else if sharePlan === null}
			<p class="text-muted-foreground text-[13px]">Checking…</p>
		{:else if sharePlan.length === 0}
			<p class="text-[13px]">Everything is already linked. Sharing again only syncs MCP servers and project settings.</p>
		{:else}
			<ul class="bg-muted max-h-60 list-disc space-y-0.5 overflow-auto rounded-md py-2 ps-6 pe-3 text-[12px]">
				{#each sharePlan as line (line)}<li>{line}</li>{/each}
			</ul>
		{/if}
		<Dialog.Footer>
			{#if shareResult}
				<Button onclick={() => (shareOpen = false)}>Done</Button>
			{:else}
				<Button variant="ghost" onclick={() => (shareOpen = false)}>Cancel</Button>
				<Button disabled={sharePlan === null || sharing} onclick={applyShare}>{sharing ? "Sharing…" : "Share now"}</Button>
			{/if}
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={renameOpen}>
	<Dialog.Content class="sm:max-w-sm">
		<form onsubmit={doRename} class="space-y-4">
			<Dialog.Header>
				<Dialog.Title>Rename {a.name}</Dialog.Title>
				<Dialog.Description>Letters, numbers, “.”, “_” and “-”. Spaces become “_”.</Dialog.Description>
			</Dialog.Header>
			<Input bind:value={newName} aria-label="New name" autofocus />
			<Dialog.Footer>
				<Button type="button" variant="ghost" onclick={() => (renameOpen = false)}>Cancel</Button>
				<Button type="submit" disabled={!newName.trim() || newName.trim() === a.name}>Rename</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<AlertDialog.Root bind:open={deleteOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete {a.name}?</AlertDialog.Title>
			<AlertDialog.Description>
				This removes {a.path} and its login. Shared sessions stay in the central home. Type the account name to confirm.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<Input bind:value={confirmName} placeholder={a.name} aria-label="Account name" />
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
			<Button variant="destructive" disabled={confirmName !== a.name} onclick={doDelete}>Delete account</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
