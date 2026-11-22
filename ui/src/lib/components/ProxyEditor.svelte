<script lang="ts">
	// Add or edit a saved proxy (Settings → Network).
	import ResponsiveDialog from "./ResponsiveDialog.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import Segmented from "./settings/Segmented.svelte";
	import ProxyCheck from "./ProxyCheck.svelte";
	import { ui } from "$lib/state/ui.svelte";
	import { proxies, type CheckResult } from "$lib/state/proxies.svelte";
	import { uid } from "$lib/state/layout.svelte";
	import { toast } from "svelte-sonner";
	import { untrack } from "svelte";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	type Kind = "http" | "https" | "socks5";
	let open = $state(false);
	let id = $state<string | undefined>();
	let name = $state("");
	let kind = $state<Kind>("http");
	let host = $state("");
	let port = $state("");
	let user = $state("");
	let pass = $state("");
	let noProxy = $state("");
	let checking = $state(false);
	let result = $state<CheckResult | null>(null);

	const PORTS: Record<Kind, string> = { http: "8080", https: "443", socks5: "1080" };

	/** Fill the fields from a URL (editing, or a URL pasted into Host). */
	function fromUrl(raw: string): boolean {
		try {
			const u = new URL(raw.trim());
			const scheme = u.protocol.replace(":", "").toLowerCase();
			kind = scheme.startsWith("socks") ? "socks5" : scheme === "https" ? "https" : "http";
			host = u.hostname.replace(/^\[|\]$/g, "");
			port = u.port || PORTS[kind];
			user = decodeURIComponent(u.username);
			pass = decodeURIComponent(u.password);
			return true;
		} catch {
			return false;
		}
	}

	$effect(() => {
		const e = ui.proxyEdit;
		if (!e) return;
		untrack(() => {
			result = null;
			if (e === "new") {
				id = undefined;
				name = "";
				kind = "http";
				host = "";
				port = "";
				user = "";
				pass = "";
				noProxy = "";
			} else {
				id = e.id;
				name = e.name;
				noProxy = e.noProxy;
				fromUrl(e.url);
			}
			open = true;
		});
	});
	$effect(() => {
		if (!open) ui.proxyEdit = null;
	});

	const portNum = $derived(Number(port || PORTS[kind]));
	const portOk = $derived(Number.isInteger(portNum) && portNum > 0 && portNum < 65536);
	const valid = $derived(!!name.trim() && !!host.trim() && portOk);
	const url = $derived.by(() => {
		const h = host.trim().includes(":") ? `[${host.trim()}]` : host.trim();
		const auth = user ? `${encodeURIComponent(user)}${pass ? `:${encodeURIComponent(pass)}` : ""}@` : "";
		return `${kind}://${auth}${h}:${portNum}`;
	});
	const taken = $derived(proxies.list.some((p) => p.id !== id && p.name.trim().toLowerCase() === name.trim().toLowerCase()));

	function onHostInput(e: Event & { currentTarget: HTMLInputElement }) {
		const v = e.currentTarget.value;
		if (v.includes("://") && fromUrl(v)) {
			if (!name.trim()) name = host;
		}
	}

	async function check() {
		if (!valid) return;
		checking = true;
		result = null;
		try {
			result = await proxies.check(url);
		} catch (e) {
			toast.error("Could not check the proxy", { description: e instanceof Error ? e.message : String(e) });
		} finally {
			checking = false;
		}
	}

	async function save() {
		if (!valid || taken) return;
		const entry = { id: id ?? uid("proxy"), name: name.trim(), url, noProxy: noProxy.trim() };
		const list = id ? proxies.list.map((p) => (p.id === id ? entry : p)) : [...proxies.list, entry];
		if (!(await proxies.save({ ...proxies.s, proxies: list }))) return;
		toast(id ? "Proxy saved" : "Proxy added", {
			description: id ? "Conversations pick up the change when they restart." : "Choose it as a default below, or per conversation.",
		});
		open = false;
	}
</script>

<ResponsiveDialog bind:open title={id ? "Edit proxy" : "Add proxy"} description="Claude, Codex and terminals can go out through it.">
	<div class="flex flex-col gap-1.5">
		<Label for="proxy-name">Name</Label>
		<Input id="proxy-name" bind:value={name} placeholder="Office proxy" dir="auto" />
		{#if taken}<p class="text-destructive text-[12px]">Another proxy has this name.</p>{/if}
	</div>
	<div class="flex flex-col gap-1.5">
		<Label>Type</Label>
		<Segmented
			label="Proxy type"
			value={kind}
			options={[
				{ value: "http", label: "HTTP" },
				{ value: "https", label: "HTTPS" },
				{ value: "socks5", label: "SOCKS5" },
			]}
			onChange={(v) => (kind = v)}
		/>
		{#if kind === "socks5"}
			<p class="text-attention flex items-start gap-1.5 text-[12px]">
				<TriangleAlert class="mt-0.5 size-3.5 shrink-0" />Claude Code does not support SOCKS proxies. Codex and terminals do.
			</p>
		{/if}
	</div>
	<div class="grid grid-cols-[1fr_6.5rem] gap-3">
		<div class="flex flex-col gap-1.5">
			<Label for="proxy-host">Host</Label>
			<Input id="proxy-host" bind:value={host} oninput={onHostInput} placeholder="proxy.example.com" dir="ltr" class="font-mono text-[13px]" autocomplete="off" />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="proxy-port">Port</Label>
			<Input id="proxy-port" bind:value={port} placeholder={PORTS[kind]} inputmode="numeric" dir="ltr" class="font-mono text-[13px] {portOk ? '' : 'border-destructive'}" />
		</div>
	</div>
	<div class="grid grid-cols-2 gap-3">
		<div class="flex flex-col gap-1.5">
			<Label for="proxy-user">Username <span class="text-muted-foreground font-normal">(optional)</span></Label>
			<Input id="proxy-user" bind:value={user} dir="ltr" autocomplete="off" />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="proxy-pass">Password <span class="text-muted-foreground font-normal">(optional)</span></Label>
			<Input id="proxy-pass" type="password" bind:value={pass} dir="ltr" autocomplete="new-password" />
		</div>
	</div>
	<div class="flex flex-col gap-1.5">
		<Label for="proxy-bypass">Reach directly</Label>
		<Input id="proxy-bypass" bind:value={noProxy} placeholder="localhost,127.0.0.1,::1" dir="ltr" class="font-mono text-[13px]" />
		<p class="text-muted-foreground text-[12px]">Hosts that skip the proxy, separated by commas. Empty: only this computer.</p>
	</div>
	{#if checking || result}
		<div class="border-border rounded-md border px-3 py-2"><ProxyCheck {result} busy={checking} /></div>
	{/if}
	{#snippet actions()}
		<Button variant="outline" class="sm:me-auto" disabled={!valid || checking} onclick={check}>Check</Button>
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button disabled={!valid || taken} onclick={save}>{id ? "Save" : "Add proxy"}</Button>
	{/snippet}
</ResponsiveDialog>
