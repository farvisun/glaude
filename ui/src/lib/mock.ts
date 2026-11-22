// Mock backend for running the UI in a plain browser (vite dev, Playwright).
// It plays a scripted Persian/English session so layout, bidi and streaming
// can be checked without spawning real agents.
import type { AppInfo, MockChannel } from "./ipc";
import type { Block, ConvEvent, Item } from "./types";
import { detectDir } from "./bidi";

type Chan = MockChannel<ConvEvent[]>;
const chans = new Map<string, Chan>();
const ptys = new Map<string, { send: (t: string) => void; line: string }>();

function mockAccount(provider: "claude" | "codex", name: string, extra: Record<string, unknown>): Record<string, unknown> {
	const iso = (msAgo: number) => new Date(Date.now() - msAgo).toISOString();
	return {
		provider,
		name,
		path: provider === "claude" ? (name === "default" ? "~/.claude" : `~/.claude_multi/${name}`) : name === "default" ? "~/.codex" : `~/.codex_${name}`,
		archived: false,
		protected: name === "default",
		active: false,
		running: false,
		email: "",
		org: "",
		plan: "",
		fiveHour: null,
		week: null,
		limitsAsOf: "",
		tokens: 0,
		tokensToday: 0,
		tokens7d: 0,
		sessions: 0,
		launches: 0,
		lastUsed: null,
		createdAt: iso(40 * 86400e3),
		usageUpdated: null,
		sharedLinked: name === "default" ? 27 : 0,
		sharedPending: 0,
		sharedTotal: provider === "claude" ? 27 : 21,
		details: [],
		...extra,
	};
}

const mockAccounts = [
	mockAccount("claude", "default", {
		active: true, running: true, email: "me@example.com", plan: "max 20x", tokens: 1_497_993_411, tokensToday: 12_400_000, tokens7d: 310_000_000, sessions: 412, launches: 230,
		fiveHour: { used: 42, resetsAt: new Date(Date.now() + 2.5 * 3600e3).toISOString() }, week: { used: 63.5, resetsAt: new Date(Date.now() + 3 * 86400e3).toISOString() },
		limitsAsOf: new Date(Date.now() - 600e3).toISOString(), lastUsed: new Date(Date.now() - 120e3).toISOString(),
		details: [{ label: "usage", text: "in 2.1M · out 8.4M · cache read 1.4B · cache write 61.2M · 98.1K msgs · 412 sessions" }, { label: "models", text: "opus-5-5 71% · sonnet-5-5 22% · haiku-5-5 7%" }],
	}),
	mockAccount("claude", "work_example.com", {
		email: "work@example.com", plan: "team 5x", tokens: 427_623_882, tokens7d: 25_747_887, sessions: 31, launches: 18, sharedLinked: 27,
		fiveHour: { used: 91, resetsAt: new Date(Date.now() + 40 * 60e3).toISOString() }, week: { used: 12 }, lastUsed: new Date(Date.now() - 2 * 86400e3).toISOString(),
	}),
	mockAccount("claude", "old_trial", { archived: true, email: "trial@example.com", plan: "pro", sharedPending: 4 }),
	mockAccount("codex", "default", {
		active: true, email: "me@example.com", plan: "prolite", tokens: 34_743_027_618, tokens7d: 339_413_329, sessions: 4070, launches: 77,
		week: { used: 21, resetsAt: new Date(Date.now() + 2.7 * 86400e3).toISOString() }, lastUsed: new Date(Date.now() - 3600e3).toISOString(),
	}),
	mockAccount("codex", "varahome", { email: "(api key)", plan: "", sharedLinked: 21 }),
];

function esc(s: string) {
	return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function p(text: string): Block {
	const html = esc(text).replace(/`([^`]+)`/g, "<code>$1</code>");
	return { html: `<p>${html}</p>`, dir: detectDir(text) };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const demoDiff = `--- a/src/lib/bidi.ts
+++ b/src/lib/bidi.ts
@@ -1,6 +1,8 @@
 export function detectDir(text: string): Dir {
-  return /[\\u0600-\\u06FF]/.test(text) ? "rtl" : "ltr";
+  let rtl = 0;
+  let ltr = 0;
+  for (const ch of text) if (RTL_RE.test(ch)) rtl++; else if (LETTER_RE.test(ch)) ltr++;
+  return rtl / (rtl + ltr) >= 0.3 ? "rtl" : "ltr";
 }
`;

export function demoItems(): Item[] {
	return [
		{
			kind: "user",
			id: "u1",
			blocks: [p("سلام! توی پروژه‌ی `glaude` تشخیص جهت متن خراب است. وقتی جمله با Claude شروع می‌شود کل پاراگراف چپ‌چین می‌شود. درستش کن.")],
			attachments: [],
		},
		{
			kind: "reasoning",
			id: "r1",
			blocks: [p("The user reports a bidi bug: first-strong detection picks LTR for Persian sentences starting with a Latin word.")],
			streaming: false,
		},
		{
			kind: "tool",
			id: "t1",
			name: "TodoWrite",
			title: "3 todos",
			status: "done",
			input: {
				todos: [
					{ content: "پیدا کردن تابع تشخیص جهت", status: "completed" },
					{ content: "Rewrite detectDir to use a ratio", status: "in_progress", activeForm: "بازنویسی detectDir با نسبت حروف" },
					{ content: "اجرای تست‌ها", status: "pending" },
				],
			},
		},
		{ kind: "tool", id: "t2", name: "Grep", title: "detectDir  src/", status: "done", input: { pattern: "detectDir" }, output: "src/lib/bidi.ts:5\nsrc/lib/components/conv/Composer.svelte:41" },
		{ kind: "tool", id: "t3", name: "Read", title: "src/lib/bidi.ts", status: "done", input: { file_path: "src/lib/bidi.ts" }, output: "export function detectDir(text: string): Dir {\n  return /[\\u0600-\\u06FF]/.test(text) ? \"rtl\" : \"ltr\";\n}" },
		{
			kind: "assistant",
			id: "a1",
			streaming: false,
			blocks: [
				p("مشکل پیدا شد. تابع `detectDir` فقط وجود یک حرف فارسی را چک می‌کند، و مرورگر هم با `dir=\"auto\"` اولین حرف قوی را ملاک می‌گیرد."),
				{
					html: "<p>راه‌حل: نسبت حروف راست‌به‌چپ را در کل بلوک بشماریم:</p>",
					dir: "rtl",
				},
				{
					html: '<ol><li>حروف داخل <code>`code`</code> و URLها حساب نشوند</li><li>اگر حداقل ۳۰٪ حروف قوی فارسی باشند، بلوک <code>rtl</code> است</li><li>بلوک‌های کد همیشه <code>ltr</code> می‌مانند</li></ol>',
					dir: "rtl",
				},
				{
					html: '<pre><code><span class="hl-keyword">export function</span> <span class="hl-function">detectDir</span>(text: <span class="hl-storage">string</span>): Dir {\n  <span class="hl-keyword">let</span> rtl = <span class="hl-number">0</span>, ltr = <span class="hl-number">0</span>;\n  <span class="hl-comment">// count strong letters only</span>\n  <span class="hl-keyword">return</span> rtl / (rtl + ltr) &gt;= <span class="hl-number">0.3</span> ? <span class="hl-string">"rtl"</span> : <span class="hl-string">"ltr"</span>;\n}</code></pre>',
					dir: "ltr",
				},
				{
					html: "<table><thead><tr><th>ورودی</th><th>قبل</th><th>بعد</th></tr></thead><tbody><tr><td>Claude یک دستیار است</td><td><code>ltr</code></td><td><code>rtl</code></td></tr><tr><td>The word سلام means hello</td><td><code>rtl</code></td><td><code>ltr</code></td></tr></tbody></table>",
					dir: "rtl",
				},
			],
		},
		{
			kind: "permission",
			id: "p1",
			requestId: "req-1",
			tool: "Edit",
			title: "src/lib/bidi.ts",
			input: { file_path: "src/lib/bidi.ts" },
			diffs: [{ path: "src/lib/bidi.ts", kind: "update", diff: demoDiff, added: 4, removed: 1 }],
			options: [
				{ id: "allow", label: "Allow", kind: "allow" },
				{ id: "allow-always", label: "Always allow edits here", kind: "allow" },
				{ id: "deny", label: "Deny", kind: "deny" },
			],
			status: "pending",
		},
	];
}

async function streamReply(ch: Chan, text: string) {
	const uid = "u" + Date.now();
	ch.push([{ type: "item", item: { kind: "user", id: uid, blocks: [p(text)], attachments: [] } }]);
	ch.push([{ type: "turn", state: "thinking" }]);
	await sleep(500);
	const aid = "a" + Date.now();
	const reply =
		detectDir(text) === "rtl"
			? "باشه. این یک پاسخ آزمایشی از بک‌اند ساختگی است تا جریان متن، جهت پاراگراف و نشانگر تایپ را ببینیم. فایل `src/main.rs` را هم بررسی کردم و همه‌چیز درست است."
			: "Sure. This is a mock reply streamed word by word so the streaming caret, block updates and auto-scroll can be checked without a real agent.";
	ch.push([
		{ type: "turn", state: "writing" },
		{ type: "item", item: { kind: "assistant", id: aid, blocks: [], streaming: true } },
	]);
	const words = reply.split(" ");
	for (let i = 1; i <= words.length; i++) {
		await sleep(45);
		ch.push([{ type: "blocks", id: aid, from: 0, blocks: [p(words.slice(0, i).join(" "))] }]);
	}
	ch.push([
		{ type: "item", item: { kind: "assistant", id: aid, blocks: [p(reply)], streaming: false } },
		{ type: "session", info: { contextUsed: 48200, costUsd: 0.214, linesAdded: 4, linesRemoved: 1 } },
		{ type: "turnDone", durationMs: 2600 },
	]);
}

export const mockBackend: Record<string, (args: Record<string, unknown>) => unknown> = {
	app_info: (): AppInfo => ({
		version: "0.1.0",
		os: "macos",
		home: "/Users/demo",
		claude: { path: "/Users/demo/.local/bin/claude", version: "2.1.329 (Claude Code)" },
		codex: { path: "/opt/homebrew/bin/codex", version: "codex-cli 0.171.0" },
	}),
	conv_start: async (args) => {
		const ch = args.onEvent as Chan;
		chans.set(args.convId as string, ch);
		const spec = args.spec as { provider: string; cwd: string; resume?: string };
		await sleep(80);
		ch.push([
			{
				type: "session",
				info: {
					sessionId: "demo-session",
					model: spec.provider === "claude" ? "opus" : "gpt-5.5-codex",
					effort: "high",
					permissionMode: spec.provider === "claude" ? "default" : "full",
					cwd: spec.cwd,
					gitBranch: "main",
					contextUsed: 41000,
					contextWindow: 200000,
					costUsd: 0.182,
					slashCommands: ["compact", "clear", "review", "init", "cost", "context"],
					models: [
						{ value: "opus", label: "Opus 5.5" },
						{ value: "sonnet", label: "Sonnet 5.5" },
						{ value: "haiku", label: "Haiku 5.5" },
					],
				},
			},
			{
				type: "limits",
				limits: {
					fiveHour: { used: 38, resetsAt: new Date(Date.now() + 2.2 * 3600e3).toISOString() },
					week: { used: 71, resetsAt: new Date(Date.now() + 3.5 * 86400e3).toISOString() },
					plan: "max 20x",
				},
			},
			{ type: "reset", items: spec.resume ? demoItems() : [] },
			{ type: "turn", state: spec.resume ? "waitingPermission" : "idle" },
		]);
		return null;
	},
	conv_send: async (args) => {
		const ch = chans.get(args.convId as string);
		if (ch) void streamReply(ch, args.text as string);
		return null;
	},
	conv_interrupt: async () => null,
	conv_permission: async (args) => {
		const ch = chans.get(args.convId as string);
		const allow = String(args.optionId).startsWith("allow");
		if (ch) {
			const it = demoItems().find((i) => i.kind === "permission") as Extract<Item, { kind: "permission" }>;
			ch.push([
				{ type: "item", item: { ...it, status: allow ? "done" : "declined" } },
				{ type: "turnDone", durationMs: 41000 },
			]);
		}
		return null;
	},
	conv_set_mode: async (args) => {
		chans.get(args.convId as string)?.push([{ type: "session", info: { permissionMode: args.mode as string } }]);
		return null;
	},
	conv_set_model: async (args) => {
		chans.get(args.convId as string)?.push([{ type: "session", info: { model: args.model as string } }]);
		return null;
	},
	conv_close: async (args) => {
		chans.delete(args.convId as string);
		return null;
	},
	pick_files: async () => [],
	save_attachment: async (args) => ({ name: args.name, mime: args.mime, path: `/tmp/${args.name}` }),
	pick_folder: async () => "/Users/demo/Projects/glaude",
	default_cwd: async () => "/Users/demo/Projects/glaude",
	// A tiny line-echo shell so terminal tabs work in the browser.
	pty_open: async (args) => {
		const id = args.id as string;
		const out = args.onData as MockChannel<Uint8Array>;
		const enc = new TextEncoder();
		const send = (t: string) => out.push(enc.encode(t));
		ptys.set(id, { send, line: "" });
		send("\x1b[1;36mdemo\x1b[0m mock shell — سلام، این ترمینال آزمایشی است\r\n$ ");
	},
	pty_write: async (args) => {
		const pty = ptys.get(args.id as string);
		if (!pty) return;
		for (const ch of args.data as string) {
			if (ch === "\r") {
				pty.send(`\r\n${pty.line ? `you typed: ${pty.line}\r\n` : ""}$ `);
				pty.line = "";
			} else if (ch === "\x7f") {
				if (pty.line) {
					pty.line = pty.line.slice(0, -1);
					pty.send("\b \b");
				}
			} else {
				pty.line += ch;
				pty.send(ch);
			}
		}
	},
	pty_resize: async () => {},
	statusline_render: async () =>
		'<span style="color:#7fd18b">glaude</span> <span class="ansi-fg-8">|</span> 🌿 (master) <span class="ansi-fg-8">|</span> <span style="color:#f2c66d;font-weight:600">42%</span> <span class="ansi-fg-8">|</span> 🤖 Opus 5.5 (high)',
	pty_close: async (args) => {
		ptys.delete(args.id as string);
	},
	codex_models: async () => [
		{ value: "gpt-6.2-sol", label: "GPT-6.2-Sol", efforts: ["low", "medium", "high"] },
		{ value: "gpt-6-luna", label: "GPT-6-Luna", efforts: ["low", "medium"] },
	],
	sessions_list: async () => {
		const now = Date.now();
		return [
			{ provider: "claude", id: "s1", cwd: "/Users/demo/Projects/glaude", title: "رفع مشکل جهت متن در پیام‌ها", updated: now - 4 * 60e3 },
			{ provider: "claude", id: "s3", cwd: "/Users/demo/Projects/shop", title: "گزارش فروش هفتگی", updated: now - 26 * 3600e3 },
			{ provider: "claude", id: "s4", cwd: "/Users/demo/Projects/shop", title: "Fix checkout rounding", updated: now - 3 * 86400e3 },
		];
	},
	codex_sessions_list: async () => [
		{ provider: "codex", id: "s2", cwd: "/Users/demo/Projects/glaude", title: "Add terminal tabs with portable-pty", updated: Date.now() - 3 * 3600e3 },
	],
	accounts_list: async () => mockAccounts.map((a) => ({ ...a })),
	accounts_refresh: async (args) => {
		const ch = args.onProgress as MockChannel<number>;
		for (const f of [0.3, 0.7, 1]) {
			await sleep(150);
			ch.push(f);
		}
		const a = mockAccounts.find((x) => x.provider === args.provider && x.name === args.name)!;
		a.usageUpdated = new Date().toISOString();
		return { ...a };
	},
	accounts_pref: async (args) => (args.value as string | null) ?? "name",
	accounts_share: async (args) => (args.dryRun ? ["would link projects", "would link settings.json"] : ["linked projects", "linked settings.json"]),
	accounts_create: async (args) => {
		mockAccounts.push(mockAccount(args.provider as "claude" | "codex", String(args.name).trim().replace(/\s+/g, "_"), {}));
		return String(args.name).trim().replace(/\s+/g, "_");
	},
	accounts_rename: async () => {},
	accounts_delete: async () => {},
	accounts_archive: async () => {},
	accounts_set_active: async () => {},
	accounts_brief: async () => [
		{ provider: "claude", name: "default", email: "me@example.com", active: true, archived: false },
		{ provider: "claude", name: "work", email: "work@example.com", active: false, archived: false },
		{ provider: "codex", name: "default", email: "me@example.com", active: true, archived: false },
	],
};
