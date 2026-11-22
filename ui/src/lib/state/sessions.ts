// Starting conversations and the actions a conversation view can take.
import { toast } from "svelte-sonner";
import { call, channel } from "$lib/ipc";
import type { Attachment, ConvEvent, Item, Provider } from "$lib/types";
import { Conversation, conversations } from "./conversations.svelte";
import { layout, uid } from "./layout.svelte";
import { basename } from "$lib/format";

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
	try {
		const ch = await channel<ConvEvent[]>((batch) => {
			for (const ev of batch) conv.apply(ev);
			if (conv.loading && batch.some((e) => e.type === "reset")) conv.loading = false;
		});
		await call("conv_start", { convId: id, spec, onEvent: ch.raw });
		conv.alive = true;
		return conv;
	} catch (e) {
		conv.loading = false;
		showError(`Could not start ${spec.provider === "claude" ? "Claude" : "Codex"}`, e);
		return null;
	}
}

type Perm = Extract<Item, { kind: "permission" }>;

export const convActions = {
	async send(conv: Conversation, text: string, attachments: Attachment[]) {
		try {
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
		try {
			await call("conv_close", { convId });
		} catch {
			/* process already gone */
		}
		conversations.remove(convId);
	},
};
