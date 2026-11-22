// Starting conversations and the actions a conversation view can take.
import { toast } from "svelte-sonner";
import { call, channel, inTauri } from "$lib/ipc";
import type { Attachment, ConvEvent, Item, Provider } from "$lib/types";
import { Conversation, conversations } from "./conversations.svelte";
import { layout, uid } from "./layout.svelte";
import { basename } from "$lib/format";
import { history } from "./history.svelte";
import { notifyPermission, notifyTurnDone, updateBadge } from "./notify";
import { ui } from "./ui.svelte";

export interface StartSpec {
	provider: Provider;
	account: string;
	cwd: string;
	model?: string;
	effort?: string;
	permissionMode?: string;
	/** Resume an existing session/thread. */
	resume?: string;
	fork?: boolean;
	skipHistory?: boolean;
	title?: string;
}

export function permissionModes(p: Provider): { value: string; label: string }[] {
	return p === "claude"
		? [
				{ value: "default", label: "Ask before acting" },
				{ value: "acceptEdits", label: "Accept edits" },
				{ value: "plan", label: "Plan only" },
				{ value: "auto", label: "Auto" },
				{ value: "bypassPermissions", label: "Bypass permissions" },
			]
		: [
				{ value: "untrusted", label: "Ask for untrusted" },
				{ value: "on-request", label: "Ask when needed" },
				{ value: "full", label: "Full access" },
				{ value: "read-only", label: "Read only" },
			];
}

let errorShown = new Set<string>();
/** Show an error once as a toast (global rule: errors once, friendly). */
export function showError(context: string, err: unknown) {
	const msg = err instanceof Error ? err.message : String(err);
	const key = context + msg;
	if (errorShown.has(key)) return;
	errorShown.add(key);
	setTimeout(() => errorShown.delete(key), 5000);
	toast.error(context, { description: msg });
}

const specs = new Map<string, StartSpec>();

function onBatch(conv: Conversation, batch: ConvEvent[]) {
	for (const ev of batch) {
		conv.apply(ev);
		if (ev.type === "notice") {
			if (ev.level === "error") showError(conv.provider === "claude" ? "Claude stopped" : "Codex stopped", ev.text);
			else if (ev.level === "warning") toast.warning(ev.text);
			else toast(ev.text);
		} else if (ev.type === "turnDone") {
			if (ev.error) showError("The turn ended with an error", ev.error);
			history.refreshSoon();
			notifyTurnDone(conv, ev.error);
		} else if (ev.type === "item" && ev.item.kind === "permission" && ev.item.status === "pending") {
			notifyPermission(conv, ev.item.tool);
		}
	}
	if (conv.loading && batch.some((e) => e.type === "reset" || e.type === "session")) conv.loading = false;
}

async function connect(conv: Conversation, spec: StartSpec): Promise<boolean> {
	specs.set(conv.id, spec);
	try {
		const ch = await channel<ConvEvent[]>((batch) => onBatch(conv, batch));
		await call("conv_start", { convId: conv.id, spec, onEvent: ch.raw });
		conv.alive = true;
		return true;
	} catch (e) {
		conv.loading = false;
		showError(`Could not start ${spec.provider === "claude" ? "Claude" : "Codex"}`, e);
		return false;
	}
}

export async function startConversation(spec: StartSpec, paneId?: string): Promise<Conversation | null> {
	const id = uid("conv");
	const title = spec.title ?? basename(spec.cwd) ?? "Conversation";
	const conv = conversations.add({
		id,
		provider: spec.provider,
		account: spec.account,
		cwd: spec.cwd,
		title,
		sessionId: spec.resume,
	});
	conv.loading = !!spec.resume;
	layout.openTab({ kind: "conversation", title, ref: id }, paneId);
	return (await connect(conv, spec)) ? conv : null;
}

/** Restart an ended process, resuming the same session. */
async function ensureAlive(conv: Conversation): Promise<boolean> {
	if (conv.alive) return true;
	const prev = specs.get(conv.id);
	const sid = conv.session.sessionId;
	if (!prev) return false;
	return connect(conv, {
		...prev,
		resume: sid ?? prev.resume,
		fork: false,
		skipHistory: true,
		model: conv.session.model ?? prev.model,
		permissionMode: conv.session.permissionMode ?? prev.permissionMode,
	});
}

type Perm = Extract<Item, { kind: "permission" }>;

/** Hand a conversation to the CLI's own TUI in a terminal tab. The
 *  structured process stops first so both never write the same session;
 *  sending from the chat tab later resumes it again. */
export async function openInTerminal(conv: Conversation) {
	const sid = conv.session.sessionId;
	if (!sid) {
		showError("Nothing to open yet", "Send a message first so the session exists.");
		return;
	}
	if (conv.running) {
		showError("The agent is still working", "Stop the turn or wait for it to finish, then open it in the terminal.");
		return;
	}
	try {
		await call("conv_close", { convId: conv.id });
	} catch {
		/* already stopped */
	}
	conv.alive = false;
	const tool = conv.provider === "claude" ? "claude" : "codex";
	ui.openTerminal({ cwd: conv.cwd, run: { provider: conv.provider, account: conv.account, resume: sid } }, `${tool} · ${conv.title || "session"}`);
}

/** Open http(s)/mailto links from rendered messages in the system browser. */
export async function openLink(href: string) {
	if (!/^(https?:|mailto:)/i.test(href)) return;
	if (inTauri) {
		const { openUrl } = await import("@tauri-apps/plugin-opener");
		await openUrl(href).catch((e) => showError("Could not open the link", e));
	} else window.open(href, "_blank", "noopener");
}

export const convActions = {
	async send(conv: Conversation, text: string, attachments: Attachment[]) {
		try {
			if (!(await ensureAlive(conv))) return;
			if (conv.items.every((i) => i.kind !== "user") && text) {
				const title = text.replace(/\s+/g, " ").slice(0, 48);
				const tab = Object.values(layout.tabs).find((t) => t.ref === conv.id);
				if (tab) layout.rename(tab.id, title);
				conv.title = title;
			}
			await call("conv_send", { convId: conv.id, text, attachments });
		} catch (e) {
			showError("Message not sent", e);
		}
	},
	async interrupt(conv: Conversation) {
		try {
			await call("conv_interrupt", { convId: conv.id });
		} catch (e) {
			showError("Could not stop the turn", e);
		}
	},
	async decide(conv: Conversation, item: Perm, optionId: string, message?: string) {
		try {
			await call("conv_permission", { convId: conv.id, requestId: item.requestId, optionId, message: message ?? null });
			setTimeout(updateBadge, 100);
		} catch (e) {
			showError("Could not answer the permission request", e);
		}
	},
	async setMode(conv: Conversation, mode: string) {
		try {
			await call("conv_set_mode", { convId: conv.id, mode });
		} catch (e) {
			showError("Could not change the permission mode", e);
		}
	},
	async setModel(conv: Conversation, model: string) {
		try {
			await call("conv_set_model", { convId: conv.id, model });
		} catch (e) {
			showError("Could not change the model", e);
		}
	},
	async pickFiles(): Promise<Attachment[]> {
		try {
			return await call<Attachment[]>("pick_files");
		} catch (e) {
			showError("Could not attach files", e);
			return [];
		}
	},
	async saveImage(blob: Blob, name: string): Promise<Attachment | null> {
		try {
			const buf = new Uint8Array(await blob.arrayBuffer());
			let bin = "";
			for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
			return await call<Attachment>("save_attachment", { name, mime: blob.type || "application/octet-stream", data: btoa(bin) });
		} catch (e) {
			showError("Could not attach the image", e);
			return null;
		}
	},
	async close(convId: string) {
		specs.delete(convId);
		try {
			await call("conv_close", { convId });
		} catch {
			/* process already gone */
		}
		conversations.remove(convId);
	},
};
