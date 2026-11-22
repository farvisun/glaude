// Starting conversations and the actions a conversation view can take.
import { toast } from "svelte-sonner";
import { call, channel, inTauri } from "$lib/ipc";
import type { Attachment, ConvEvent, Item, Provider, QueuedMessage } from "$lib/types";
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
	/** Claude: resume only up to this message (rewind). */
	resumeAt?: string;
	/** Claude: run the session as this agent. */
	agent?: string;
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

// Conversation tabs survive a restart: each one's start spec, title and
// session id are saved, so it can reattach to the host or resume.
const SAVED_KEY = "glaude.convs.v1";
interface SavedConv {
	spec: StartSpec;
	title: string;
	sessionId?: string;
}

function loadSaved(): Record<string, SavedConv> {
	try {
		return JSON.parse(localStorage.getItem(SAVED_KEY) ?? "{}") as Record<string, SavedConv>;
	} catch {
		return {};
	}
}

function saveConv(conv: Conversation) {
	const spec = specs.get(conv.id);
	if (!spec) return;
	const all = loadSaved();
	const { title: _t, resumeAt: _r, fork: _f, skipHistory: _s, ...base } = spec;
	all[conv.id] = { spec: base, title: conv.title, sessionId: conv.session.sessionId ?? spec.resume };
	localStorage.setItem(SAVED_KEY, JSON.stringify(all));
}

function forgetConv(id: string) {
	const all = loadSaved();
	if (!(id in all)) return;
	delete all[id];
	localStorage.setItem(SAVED_KEY, JSON.stringify(all));
}

/** Bring back the conversation tabs of the last run: attach to the ones the
 *  host still runs (they may have kept working), and resume the others from
 *  their transcript when they are shown. */
export async function restoreConversations() {
	const saved = loadSaved();
	const tabs = Object.values(layout.tabs).filter((t) => t.kind === "conversation" && t.ref);
	let live = new Set<string>();
	try {
		const list = await call<{ convId: string }[]>("host_list");
		live = new Set(list.map((l) => l.convId));
	} catch {
		/* the host starts with the first conversation */
	}
	for (const tab of tabs) {
		const id = tab.ref as string;
		const s = saved[id];
		if (!s || conversations.get(id)) continue;
		const conv = conversations.add({
			id,
			provider: s.spec.provider,
			account: s.spec.account,
			cwd: s.spec.cwd,
			title: s.title,
			sessionId: s.sessionId,
		});
		specs.set(id, s.spec);
		if (live.has(id)) void attach(conv);
		else conv.needsHistory = !!s.sessionId;
	}
	// Drop saved conversations whose tab is gone.
	const open = new Set(tabs.map((t) => t.ref));
	const kept = Object.fromEntries(Object.entries(saved).filter(([id]) => open.has(id)));
	localStorage.setItem(SAVED_KEY, JSON.stringify(kept));
}

async function attach(conv: Conversation) {
	conv.loading = true;
	try {
		const ch = await channel<ConvEvent[]>((batch) => onBatch(conv, batch));
		await call("conv_attach", { convId: conv.id, onEvent: ch.raw });
		conv.alive = true;
	} catch {
		conv.loading = false;
		conv.needsHistory = !!conv.session.sessionId;
	}
}

/** A restored tab is shown: load its history (this resumes the session). */
export async function loadHistory(conv: Conversation) {
	if (!conv.needsHistory) return;
	conv.needsHistory = false;
	const prev = specs.get(conv.id);
	const sid = conv.session.sessionId;
	if (!prev || !sid) return;
	conv.loading = true;
	await connect(conv, { ...prev, resume: sid, fork: false, skipHistory: false });
}

/** The host went away (crash, update): everything restarts on demand. */
export function hostLost() {
	let any = false;
	for (const conv of Object.values(conversations.byId)) {
		if (conv.alive) any = true;
		conv.alive = false;
		conv.turn = "idle";
	}
	if (any) toast.warning("The agent host stopped", { description: "Conversations restart when you send a message." });
}

function onBatch(conv: Conversation, batch: ConvEvent[]) {
	for (const ev of batch) {
		const prevSid = conv.session.sessionId;
		conv.apply(ev);
		if (ev.type === "session" && ev.info.sessionId && ev.info.sessionId !== prevSid) saveConv(conv);
		if (ev.type === "notice") {
			if (ev.level === "error") showError(conv.provider === "claude" ? "Claude stopped" : "Codex stopped", ev.text);
			else if (ev.level === "warning") toast.warning(ev.text);
			else toast(ev.text);
		} else if (ev.type === "turnDone") {
			// A usage limit shows as a banner with "Resume at reset" instead.
			if (ev.error && !/usage limit|limit reached|rate limit/i.test(ev.error)) showError("The turn ended with an error", ev.error);
			history.refreshSoon();
			notifyTurnDone(conv, ev.error);
		} else if (ev.type === "item" && ev.item.kind === "permission" && ev.item.status === "pending") {
			notifyPermission(conv, ev.item.tool === "ExitPlanMode" ? "plan" : ev.item.tool);
		} else if (ev.type === "item" && ev.item.kind === "question" && ev.item.status === "pending") {
			notifyPermission(conv, "question");
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
		saveConv(conv);
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

const IDLE_MS = 15 * 60_000;
let reaper: ReturnType<typeof setInterval> | null = null;

/** Stop agent processes idle for 15 minutes; the next message resumes the
 *  same session (ensureAlive). Keeps memory flat with many open tabs. */
export function startIdleReaper() {
	if (reaper) return;
	reaper = setInterval(() => {
		const now = Date.now();
		for (const conv of Object.values(conversations.byId)) {
			if (!conv.alive || conv.running || conv.pendingPermissions > 0 || conv.runningTasks.length > 0) continue;
			if (now - conv.lastActivity < IDLE_MS) continue;
			conv.alive = false;
			call("conv_close", { convId: conv.id }).catch(() => {});
		}
	}, 60_000);
}

/** Focus the tab of a past session if it is open, otherwise resume it. */
export function openSession(s: { provider: Provider; id: string; cwd: string; title?: string; account?: string }) {
	const open = Object.values(conversations.byId).find((c) => c.session.sessionId === s.id);
	if (open) {
		const tab = Object.values(layout.tabs).find((t) => t.ref === open.id);
		if (tab) return layout.activate(tab.id);
	}
	void startConversation({ provider: s.provider, account: s.account ?? "default", cwd: s.cwd, resume: s.id, title: s.title });
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

export interface HookRun {
	name: string;
	event: string;
	outcome: string;
	exitCode?: number;
	output: string;
	at: number;
}

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
		conv.lastActivity = Date.now();
		try {
			if (!(await ensureAlive(conv))) return;
			if (conv.items.every((i) => i.kind !== "user") && text) {
				const title = text.replace(/\s+/g, " ").slice(0, 48);
				const tab = Object.values(layout.tabs).find((t) => t.ref === conv.id);
				if (tab) layout.rename(tab.id, title);
				conv.title = title;
				saveConv(conv);
			}
			await call("conv_send", { convId: conv.id, text, attachments });
		} catch (e) {
			showError("Message not sent", e);
		}
	},
	/** Send into the running turn now (Codex steers; Claude takes it mid-turn). */
	async steer(conv: Conversation, text: string, attachments: Attachment[]) {
		conv.lastActivity = Date.now();
		try {
			if (!(await ensureAlive(conv))) return;
			await call("conv_send", { convId: conv.id, text, attachments, steer: true });
		} catch (e) {
			showError("Message not sent", e);
		}
	},
	/** Replace the queue of messages sent when the turn ends. */
	async setQueue(conv: Conversation, items: QueuedMessage[]) {
		const before = conv.queue;
		conv.queue = items;
		try {
			if (!(await ensureAlive(conv))) throw new Error("The conversation is not running");
			await call("conv_queue", { convId: conv.id, items });
		} catch (e) {
			conv.queue = before;
			showError("Could not update the queue", e);
		}
	},
	async enqueue(conv: Conversation, text: string, attachments: Attachment[]) {
		await convActions.setQueue(conv, [...conv.queue, { id: uid("q"), text, attachments }]);
	},
	/** Continue at `at` (RFC 3339), or cancel with null. */
	async schedule(conv: Conversation, at: string | null) {
		try {
			if (!(await ensureAlive(conv))) throw new Error("The conversation is not running");
			await call("conv_schedule", { convId: conv.id, resumeAt: at });
		} catch (e) {
			showError("Could not schedule the resume", e);
		}
	},
	async interrupt(conv: Conversation) {
		try {
			await call("conv_interrupt", { convId: conv.id });
		} catch (e) {
			showError("Could not stop the turn", e);
		}
	},
	async decide(conv: Conversation, item: { requestId: string }, optionId: string, message?: string) {
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
	async answerQuestion(conv: Conversation, item: Extract<Item, { kind: "question" }>, answers: Record<string, string>) {
		try {
			await call("conv_answer_question", { convId: conv.id, requestId: item.requestId, answers });
			setTimeout(updateBadge, 100);
		} catch (e) {
			showError("Could not send your answer", e);
		}
	},
	/** A control request to the running Claude process (panels, tasks, rewind). */
	async control<T = unknown>(conv: Conversation, request: Record<string, unknown>): Promise<T> {
		if (!(await ensureAlive(conv))) throw new Error("The conversation is not running");
		return call<T>("conv_control", { convId: conv.id, request });
	},
	async stopTask(conv: Conversation, taskId: string) {
		try {
			await convActions.control(conv, { subtype: "stop_task", task_id: taskId });
		} catch (e) {
			showError("Could not stop the task", e);
		}
	},
	/** Output of a background shell/monitor: live from the CLI, else its file. */
	async taskOutput(conv: Conversation, taskId: string, outputFile?: string): Promise<string> {
		if (conv.alive) {
			try {
				const r = await call<{ output: string }>("conv_control", {
					convId: conv.id,
					request: { subtype: "get_task_output", task_id: taskId },
				});
				return r.output ?? "";
			} catch {
				/* fall back to the file */
			}
		}
		if (outputFile) return call<string>("task_output_file", { path: outputFile });
		throw new Error("The output is no longer available");
	},
	/** Files Claude changed since a user message (dry run), or undo them. */
	async rewindFiles(conv: Conversation, userId: string, dryRun: boolean) {
		return convActions.control<{ canRewind: boolean; error?: string; filesChanged?: string[]; insertions?: number; deletions?: number }>(
			conv,
			{ subtype: "rewind_files", user_message_id: userId, dry_run: dryRun },
		);
	},
	/** Restart the conversation from just before a user message, as a fork.
	 *  The message goes back into the composer. */
	async rewindConversation(conv: Conversation, userId: string, text: string) {
		const prev = specs.get(conv.id);
		const sid = conv.session.sessionId;
		if (!prev || !sid) {
			showError("Can't rewind yet", "The session has not started.");
			return false;
		}
		try {
			const target = await call<string | null>("claude_rewind_target", {
				account: conv.account,
				cwd: conv.cwd,
				sessionId: sid,
				userUuid: userId,
			});
			try {
				await call("conv_close", { convId: conv.id });
			} catch {
				/* already stopped */
			}
			conv.alive = false;
			conv.truncateBefore(userId);
			conv.tasks = {};
			const ok = await connect(conv, {
				...prev,
				resume: target ? sid : undefined,
				resumeAt: target ?? undefined,
				fork: !!target,
				skipHistory: true,
				model: conv.session.model ?? prev.model,
				permissionMode: conv.session.permissionMode ?? prev.permissionMode,
			});
			if (!target) conv.session = { ...conv.session, sessionId: undefined };
			conv.draft = text;
			return ok;
		} catch (e) {
			showError("Could not rewind the conversation", e);
			return false;
		}
	},
	async hookRuns(conv: Conversation): Promise<HookRun[]> {
		if (!conv.alive) return [];
		return call<HookRun[]>("conv_hook_runs", { convId: conv.id });
	},
	async close(convId: string) {
		specs.delete(convId);
		forgetConv(convId);
		try {
			await call("conv_close", { convId });
		} catch {
			/* process already gone */
		}
		conversations.remove(convId);
	},
};
