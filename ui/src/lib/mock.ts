// Mock backend for running the UI in a plain browser (vite dev, Playwright).
// It plays a scripted Persian/English session so layout, bidi and streaming
// can be checked without spawning real agents.
import type { AppInfo, MockChannel } from "./ipc";
import type { Block, ConvEvent, Item } from "./types";
import { detectDir } from "./bidi";

type Chan = MockChannel<ConvEvent[]>;
const chans = new Map<string, Chan>();

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
	sessions_list: async () => {
		const now = Date.now();
		return [
			{ provider: "claude", id: "s1", cwd: "/Users/demo/Projects/glaude", title: "رفع مشکل جهت متن در پیام‌ها", updated: now - 4 * 60e3 },
			{ provider: "codex", id: "s2", cwd: "/Users/demo/Projects/glaude", title: "Add terminal tabs with portable-pty", updated: now - 3 * 3600e3 },
			{ provider: "claude", id: "s3", cwd: "/Users/demo/Projects/shop", title: "گزارش فروش هفتگی", updated: now - 26 * 3600e3 },
			{ provider: "claude", id: "s4", cwd: "/Users/demo/Projects/shop", title: "Fix checkout rounding", updated: now - 3 * 86400e3 },
		];
	},
	accounts_brief: async () => [
		{ provider: "claude", name: "default", email: "me@example.com", active: true, archived: false },
		{ provider: "claude", name: "work", email: "work@example.com", active: false, archived: false },
		{ provider: "codex", name: "default", email: "me@example.com", active: true, archived: false },
	],
};
