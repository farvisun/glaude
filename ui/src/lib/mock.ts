// Mock backend for running the UI in a plain browser (vite dev, Playwright).
// It plays a scripted Persian/English session so layout, bidi and streaming
// can be checked without spawning real agents.
import { MockChannel, type AppInfo } from "./ipc";
import type { Block, ConvEvent, FileDiff, Item, QueuedMessage, TaskInfo } from "./types";
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
	const rid = "r" + Date.now();
	ch.push([{ type: "item", item: { kind: "reasoning", id: rid, blocks: [], streaming: true } }]);
	await sleep(1800);
	ch.push([
		{ type: "item", item: { kind: "reasoning", id: rid, blocks: [p("The user wants a quick check; read the file first.")], streaming: false } },
		{ type: "turn", state: "tool" },
	]);
	const tid = "t" + Date.now();
	const tool = { kind: "tool", id: tid, name: "Read", title: "src/main.rs", input: { file_path: "src/main.rs" } } as const;
	ch.push([{ type: "item", item: { ...tool, status: "running" } as Item }]);
	await sleep(1500);
	ch.push([{ type: "item", item: { ...tool, status: "done", output: "fn main() {}" } as Item }]);
	const aid = "a" + Date.now();
	const reply =
		detectDir(text) === "rtl"
			? "باشه. این یک پاسخ آزمایشی از بک‌اند ساختگی است تا جریان متن، جهت پاراگراف و نشانگر تایپ را ببینیم. فایل `src/main.rs` را هم بررسی کردم و همه‌چیز درست است."
			: "Sure. This is a mock reply streamed word by word so streaming, block updates and auto-scroll can be checked without a real agent.";
	ch.push([
		{ type: "turn", state: "writing" },
		{ type: "item", item: { kind: "assistant", id: aid, blocks: [], streaming: true } },
	]);
	const words = reply.split(" ");
	for (let i = 1; i <= words.length; i++) {
		await sleep(90);
		ch.push([{ type: "blocks", id: aid, from: 0, blocks: [p(words.slice(0, i).join(" "))] }]);
	}
	ch.push([
		{ type: "item", item: { kind: "assistant", id: aid, blocks: [p(reply)], streaming: false } },
		{ type: "session", info: { contextUsed: 48200, costUsd: 0.214, linesAdded: 4, linesRemoved: 1 } },
		{ type: "turnDone", durationMs: 2600 },
	]);
}

// ---- Agents, workflows, questions, plans (type "demo agents" etc.) ----

const pendingPrompts = new Map<string, Item>();
const mockTasks = new Map<string, TaskInfo>();
const mockAgents = [
	{ name: "general-purpose", description: "General-purpose agent for research and multi-step tasks." },
	{ name: "Explore", description: "Fast read-only search across the codebase.", model: "haiku" },
	{ name: "Plan", description: "Designs an implementation plan without editing files." },
	{ name: "code-reviewer", description: "Reviews a diff for bugs and risky changes.", model: "sonnet" },
];

function pushTask(ch: Chan, t: TaskInfo) {
	mockTasks.set(t.id, t);
	ch.push([{ type: "task", task: t }]);
}

async function userTurn(ch: Chan, text: string) {
	ch.push([
		{ type: "item", item: { kind: "user", id: "u" + Date.now(), blocks: [p(text)], attachments: [] } },
		{ type: "turn", state: "thinking" },
	]);
	await sleep(500);
}

async function say(ch: Chan, text: string, parent?: string) {
	const id = "a" + Date.now() + Math.random().toString(36).slice(2, 6);
	ch.push([{ type: "item", item: { kind: "assistant", id, blocks: [p(text)], parent, streaming: false } }]);
}

async function demoAgents(ch: Chan, text: string) {
	await userTurn(ch, text);
	await say(ch, "I'll explore the codebase with two agents in parallel and run the tests in the background.");
	const now = Date.now();
	const calls = [
		{ id: "toolu_ag1", type: "Explore", desc: "Find RTL handling", prompt: "Find where text direction is decided and list the files." },
		{ id: "toolu_ag2", type: "code-reviewer", desc: "Review the bidi module", prompt: "Review src/lib/bidi.ts for edge cases with mixed Persian and English." },
	];
	for (const c of calls) {
		ch.push([
			{
				type: "item",
				item: { kind: "tool", id: c.id, name: "Agent", title: c.desc, status: "done", input: { description: c.desc, subagent_type: c.type, prompt: c.prompt }, output: "Async agent launched successfully." },
			},
		]);
		pushTask(ch, { id: "task-" + c.id, toolUseId: c.id, kind: "agent", description: c.desc, subagentType: c.type, status: "running", background: true, startedAt: now });
	}
	ch.push([
		{ type: "item", item: { kind: "tool", id: "toolu_sh1", name: "Bash", title: "bun run test", status: "done", input: { command: "bun run test", run_in_background: true }, output: "Command running in background with ID: b1" } },
	]);
	pushTask(ch, { id: "b1", toolUseId: "toolu_sh1", kind: "shell", description: "bun run test", status: "running", background: true, startedAt: now });
	ch.push([{ type: "turnDone", durationMs: 4200 }]);

	// The agents work in the background.
	const steps: [string, string, string][] = [
		["toolu_ag1", "Grep", "detectDir"],
		["toolu_ag2", "Read", "src/lib/bidi.ts"],
		["toolu_ag1", "Read", "crates/glaude-core/src/bidi.rs"],
		["toolu_ag2", "Grep", "\\u200c"],
	];
	let n = 0;
	for (const [parent, name, title] of steps) {
		await sleep(900);
		const id = `st${n++}`;
		ch.push([{ type: "item", item: { kind: "tool", id, name, title, status: "done", parent, input: { pattern: title }, output: "3 matches" } }]);
		const t = mockTasks.get("task-" + parent);
		if (t && t.status === "running")
			pushTask(ch, { ...t, lastTool: name, activity: name === "Grep" ? `Searching for ${title}` : `Reading ${title}`, usage: { tokens: 4200 * n, toolUses: n, durationMs: Date.now() - now } });
	}
	await say(ch, "Direction is decided per block from the share of strong RTL letters.", "toolu_ag1");
	for (const c of calls) {
		await sleep(700);
		const t = mockTasks.get("task-" + c.id);
		if (!t || t.status !== "running") continue;
		const report =
			c.type === "Explore"
				? "Direction is decided in crates/glaude-core/src/bidi.rs (detect_dir) and mirrored in ui/src/lib/bidi.ts. Code blocks are always LTR."
				: "Found one risk: a block that starts with a URL and is mostly Persian is marked LTR. Suggest ignoring URLs when counting.";
		pushTask(ch, { ...t, status: "completed", endedAt: Date.now(), report, usage: { tokens: 18400, toolUses: 4, durationMs: Date.now() - now } });
		ch.push([{ type: "item", item: { kind: "system", id: "sys" + c.id, level: "task", text: `${c.type} agent “${c.desc}” finished` } }]);
	}
	ch.push([{ type: "turn", state: "thinking" }]);
	await sleep(800);
	await say(ch, "Both agents are done. The bidi detector is in Rust with a TypeScript twin; the review found one edge case with leading URLs.");
	ch.push([{ type: "turnDone", durationMs: 1800 }]);
	await sleep(6000);
	const sh = mockTasks.get("b1");
	if (sh?.status === "running") {
		pushTask(ch, { ...sh, status: "completed", endedAt: Date.now(), report: "42 tests passed" });
		ch.push([{ type: "item", item: { kind: "system", id: "sys-b1", level: "task", text: "Background command “bun run test” completed (exit code 0)" } }]);
	}
}

async function demoWorkflow(ch: Chan, text: string) {
	await userTurn(ch, text);
	const tid = "toolu_wf1";
	const script = "export const meta = { name: 'review-bidi', phases: [{ title: 'Review' }, { title: 'Verify' }] }\nconst r = await agent('Review bidi.ts')\n";
	ch.push([{ type: "item", item: { kind: "tool", id: tid, name: "Workflow", title: "review-bidi", status: "done", input: { script }, output: "Workflow launched" } }]);
	const base: TaskInfo = {
		id: "wf1",
		toolUseId: tid,
		kind: "workflow",
		description: "Review changed files, verify each finding",
		workflowName: "review-bidi",
		status: "running",
		background: true,
		startedAt: Date.now(),
		phases: [
			{ index: 0, title: "Review" },
			{ index: 1, title: "Verify" },
		],
		agents: [],
	};
	pushTask(ch, base);
	ch.push([{ type: "turnDone", durationMs: 900 }]);
	const agents = [
		{ index: 0, label: "review:bugs", phaseIndex: 0, agentType: "general-purpose", model: "claude-opus-5-5" },
		{ index: 1, label: "review:rtl", phaseIndex: 0, agentType: "general-purpose", model: "claude-opus-5-5" },
		{ index: 2, label: "verify:bidi.ts", phaseIndex: 1, agentType: "Explore", model: "claude-haiku-5-5" },
	];
	let cur = base;
	for (const a of agents) {
		await sleep(700);
		cur = { ...cur, agents: [...(cur.agents ?? []), { ...a, state: "progress", lastToolName: "Read", lastToolSummary: "Reading src/lib/bidi.ts", promptPreview: "Review the diff for…" }] };
		pushTask(ch, cur);
	}
	for (const a of agents) {
		await sleep(900);
		if (mockTasks.get("wf1")?.status !== "running") return;
		cur = {
			...cur,
			agents: (cur.agents ?? []).map((x) =>
				x.index === a.index ? { ...x, state: a.index === 1 ? "error" : "done", tokens: 12000 + a.index * 3100, durationMs: 4100 + a.index * 900, resultPreview: "1 finding: leading URL flips direction", error: a.index === 1 ? "Agent hit the turn limit" : undefined } : x,
			),
			usage: { tokens: 40000, toolUses: 12, durationMs: Date.now() - cur.startedAt },
		};
		pushTask(ch, cur);
	}
	pushTask(ch, { ...cur, status: "completed", endedAt: Date.now(), report: "1 confirmed finding: a block starting with a URL is marked LTR." });
	ch.push([{ type: "item", item: { kind: "system", id: "sys-wf1", level: "task", text: "Workflow “review-bidi” finished" } }]);
}

async function demoQuestion(ch: Chan, text: string) {
	await userTurn(ch, text);
	const item: Item = {
		kind: "question",
		id: "perm-q1",
		requestId: "q1",
		toolUseId: "toolu_q1",
		status: "pending",
		questions: [
			{
				question: "Which layout should the settings page use?",
				header: "Layout",
				multiSelect: false,
				options: [
					{ label: "Sidebar sections", description: "A list of sections on the left, content on the right.", preview: "┌────────┬──────────────┐\n│ General│ Theme  ◉ Dark │\n│ Agents │ Font   Estedad│\n│ Keys   │              │\n└────────┴──────────────┘" },
					{ label: "Single page", description: "Everything on one scrolling page.", preview: "┌───────────────────────┐\n│ General               │\n│ Theme ◉ Dark          │\n│ Agents                │\n└───────────────────────┘" },
				],
			},
			{
				question: "کدام زبان‌ها پشتیبانی شوند؟",
				header: "زبان‌ها",
				multiSelect: true,
				options: [
					{ label: "فارسی", description: "راست‌به‌چپ با فونت استعداد" },
					{ label: "English", description: "Left to right" },
					{ label: "العربية" },
				],
			},
		],
	};
	pendingPrompts.set("q1", item);
	ch.push([{ type: "item", item }, { type: "turn", state: "waitingPermission" }]);
}

async function demoPlan(ch: Chan, text: string) {
	await userTurn(ch, text);
	ch.push([{ type: "session", info: { permissionMode: "plan" } }]);
	await say(ch, "I read the composer and the bidi helpers. Here is the plan.");
	const item: Item = {
		kind: "permission",
		id: "perm-plan1",
		requestId: "plan1",
		tool: "ExitPlanMode",
		toolUseId: "toolu_plan1",
		title: "Plan",
		input: { plan: "…", planFilePath: "/Users/demo/.claude/plans/rtl-composer.md" },
		options: [
			{ id: "allow-accept-edits", label: "Yes, and auto-accept edits", kind: "allow" },
			{ id: "allow-default", label: "Yes, and ask before each edit", kind: "allow" },
			{ id: "deny", label: "No, keep planning", kind: "deny" },
		],
		status: "pending",
		blocks: [
			{ html: "<h2>RTL composer</h2>", dir: "ltr" },
			{ html: "<p>Switch the composer direction as the user types, per paragraph.</p>", dir: "ltr" },
			{ html: "<ol><li>Detect direction per line in <code>bidi.ts</code></li><li>Apply <code>dir</code> to the textarea</li><li>Add tests for mixed text: <bdi>سلام world</bdi></li></ol>", dir: "ltr" },
			{ html: "<p>متن فارسی در پیش‌نمایش هم باید راست‌به‌چپ بماند.</p>", dir: "rtl" },
		],
	};
	pendingPrompts.set("plan1", item);
	ch.push([{ type: "item", item }, { type: "turn", state: "waitingPermission" }]);
}

async function demoLimit(ch: Chan) {
	ch.push([{ type: "turn", state: "requesting" }]);
	await sleep(300);
	const until = new Date(Date.now() + 2 * 3600e3).toISOString();
	ch.push([
		{ type: "turnDone", durationMs: 300, error: "You've hit your usage limit. Resets in 2 hours." },
		{ type: "schedule", limitedUntil: until },
	]);
}

// Like the host: messages queued during a turn go out when it ends.
const mockQueues = new Map<string, QueuedMessage[]>();
const busy = new Set<string>();
function hostChannel(convId: string, ch: Chan): Chan {
	const proxy = new MockChannel<ConvEvent[]>();
	proxy.push = (batch: ConvEvent[]) => {
		ch.push(batch);
		for (const e of batch) {
			if (e.type === "turn" && e.state !== "idle" && e.state !== "waitingPermission") busy.add(convId);
			if (e.type === "turnDone") {
				busy.delete(convId);
				const q = mockQueues.get(convId) ?? [];
				if (q.length && !e.error) {
					const [next, ...rest] = q;
					mockQueues.set(convId, rest);
					setTimeout(() => {
						ch.push([{ type: "queue", items: rest }]);
						void mockBackend.conv_send({ convId, text: next.text, attachments: next.attachments ?? [] });
					}, 50);
				}
			}
		}
	};
	return proxy;
}

async function demoPaths(ch: Chan, text: string) {
	ch.push([{ type: "item", item: { kind: "user", id: "u" + Date.now(), blocks: [p(text)], attachments: [] } }]);
	await sleep(200);
	ch.push([
		{
			type: "item",
			item: {
				kind: "assistant",
				id: "a" + Date.now(),
				streaming: false,
				blocks: [
					{
						html: '<p>The bug is in <code>src/main.rs:12</code>; the helpers live in <code>ui/src</code>. See <a href="docs/plan.md">the plan</a>. <code>nope/missing.ts</code> does not exist and <code>cargo test</code> is a command.</p>',
						dir: "ltr",
					},
				],
			},
		},
		{ type: "turnDone", durationMs: 200 },
	]);
}

// Files the mock pretends exist under the demo project.
const MOCK_ROOT = "/Users/demo/Projects/glaude";
const mockFiles: Record<string, string> = {
	"src/main.rs": Array.from({ length: 40 }, (_, i) => (i === 11 ? "    let x = parse(input)?; // line 12" : `// line ${i + 1}`)).join("\n"),
	"docs/plan.md": "# Plan\n\nShip it.",
};
const mockDirs = new Set(["ui/src", "src", "docs"]);

const mockDiffs = (): FileDiff[] => [
	{ path: "src/main.rs", kind: "update", added: 2, removed: 1, diff: "@@ -10,4 +10,5 @@\n fn main() {\n-    let x = parse(input);\n+    let x = parse(input)?;\n+    run(x);\n }\n" },
	{ path: "ui/src/app.css", kind: "add", added: 2, removed: 0, diff: "@@ -0,0 +1,2 @@\n+.path-link { cursor: pointer; }\n+.x {}\n" },
];
let mockDirty = 2;

const demoScenarios: [RegExp, (ch: Chan, text: string) => Promise<void>][] = [
	[/^demo paths/i, demoPaths],
	[/^demo limit/i, demoLimit],
	[/^demo agents/i, demoAgents],
	[/^demo workflow/i, demoWorkflow],
	[/^demo ask/i, demoQuestion],
	[/^demo plan/i, demoPlan],
];

function mockControl(convId: string, req: Record<string, unknown>): unknown {
	const ch = chans.get(convId);
	switch (req.subtype) {
		case "get_context_usage":
			return {
				totalTokens: 42592,
				maxTokens: 200000,
				percentage: 21,
				model: "claude-opus-5-5",
				categories: [
					{ id: "system_prompt", name: "System prompt", tokens: 3421, kind: "used" },
					{ id: "system_tools", name: "System tools", tokens: 2771, kind: "used" },
					{ id: "mcp_instructions", name: "MCP server instructions", tokens: 3971, kind: "used" },
					{ id: "mcp_tools_deferred", name: "MCP tools (deferred)", tokens: 23480, kind: "deferred" },
					{ id: "custom_agents", name: "Custom agents", tokens: 432, kind: "used" },
					{ id: "memory_files", name: "Memory files", tokens: 4229, kind: "used" },
					{ id: "skills", name: "Skills", tokens: 9959, kind: "used" },
					{ id: "messages", name: "Messages", tokens: 17809, kind: "used" },
					{ id: "autocompact_buffer", name: "Autocompact buffer", tokens: 33000, kind: "buffer" },
					{ id: "free_space", name: "Free space", tokens: 124408, kind: "free" },
				],
				memoryFiles: [
					{ path: "/Users/demo/.claude/CLAUDE.md", type: "User", tokens: 3858 },
					{ path: "/Users/demo/Projects/glaude/CLAUDE.md", type: "Project", tokens: 371 },
				],
			};
		case "mcp_status":
			return {
				mcpServers: [
					{ name: "context", status: "connected", scope: "user", transport: "http", serverInfo: { name: "context-hub", version: "1.4.0" }, tools: [{ name: "search" }, { name: "get" }, { name: "remember" }] },
					{ name: "figma", status: "failed", scope: "user", transport: "http", error: "502 Bad Gateway" },
					{ name: "taskie", status: "connected", scope: "user", transport: "http", tools: [{ name: "task_create" }, { name: "task_list" }] },
					{ name: "webstorm", status: "disabled", scope: "local", transport: "sse" },
				],
			};
		case "mcp_reconnect":
		case "mcp_toggle":
			return {};
		case "get_skills_dialog":
			return {
				skills: [
					{ name: "codex-image", description: "Generate raster images by delegating to Codex.", source: "userSettings", tokens: 210, state: "on" },
					{ name: "commit", description: "Commit pending changes as logical Conventional Commits.", source: "userSettings", tokens: 96, state: "on" },
					{ name: "taskie-work-log", description: "Record agent work as a Taskie task.", source: "userSettings", tokens: 140, state: "user-invocable-only" },
				],
			};
		case "get_hooks_listing":
			return {
				hooks: [
					{ event: "PreToolUse", matcher: "Bash", source: "userSettings", sourceLabel: "User settings (~/.claude/settings.json)", type: "command", displayText: "rtk hook claude" },
					{ event: "PostToolUse", matcher: "Bash", source: "userSettings", sourceLabel: "User settings (~/.claude/settings.json)", type: "command", displayText: "atuin hook claude-code" },
					{ event: "SessionStart", matcher: "", source: "plugin", sourceLabel: "Plugin caveman", type: "command", displayText: "caveman init" },
				],
			};
		case "get_memory_dialog":
			return {
				files: [
					{ kind: "user", path: "/Users/demo/.claude/CLAUDE.md", label: "User instructions", exists: true },
					{ kind: "project", path: "/Users/demo/Projects/glaude/CLAUDE.md", label: "Project instructions", exists: false },
				],
				folders: [{ kind: "auto", path: "/Users/demo/.claude/projects/-Users-demo-Projects-glaude/memory", label: "Open auto-memory folder" }],
				auto_memory: { enabled: true, status: "on" },
			};
		case "get_task_output":
			return { output: "$ bun run test\n✓ bidi > detects Persian (2 ms)\n✓ bidi > mixed lines (1 ms)\n✓ blocks > tables\n\n 42 pass\n 0 fail\n", total_bytes: 120, truncated: false };
		case "stop_task": {
			const t = mockTasks.get(req.task_id as string);
			if (t && ch) {
				const stopped = { ...t, status: "stopped", endedAt: Date.now() } as TaskInfo;
				mockTasks.set(t.id, stopped);
				ch.push([{ type: "task", task: stopped }, { type: "item", item: { kind: "system", id: "stop" + t.id, level: "task", text: `“${t.description}” was stopped` } }]);
			}
			return {};
		}
		case "rewind_files":
			return req.dry_run ? { canRewind: true, filesChanged: ["src/lib/bidi.ts", "src/lib/components/conv/Composer.svelte"], insertions: 24, deletions: 6 } : { canRewind: true };
		case "apply_flag_settings":
			return {};
		case "file_suggestions": {
			const all = ["src/", "src/lib/", "src/lib/bidi.ts", "src/lib/types.ts", "src/App.svelte", "README.md", "docs/protocols.md"];
			const q = String(req.query ?? "").toLowerCase();
			return { suggestions: all.filter((f) => f.toLowerCase().includes(q)).map((path) => ({ path })) };
		}
	}
	throw new Error(`${String(req.subtype)} is not available`);
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
		const ch = hostChannel(args.convId as string, args.onEvent as Chan);
		chans.set(args.convId as string, ch);
		const spec = args.spec as { provider: string; cwd: string; resume?: string; skipHistory?: boolean };
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
					agents: spec.provider === "claude" ? mockAgents : undefined,
					outputStyle: "default",
					outputStyles: ["default", "Proactive", "Concise", "Explanatory", "Learning"],
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
			// Like the real driver: a restart (skipHistory) keeps what the UI shows.
			...(spec.skipHistory ? [] : [{ type: "reset", items: spec.resume ? demoItems() : [] } as ConvEvent]),
			{ type: "turn", state: spec.resume ? "waitingPermission" : "idle" },
		]);
		return null;
	},
	conv_send: async (args) => {
		const ch = chans.get(args.convId as string);
		const text = args.text as string;
		const scenario = demoScenarios.find(([re]) => re.test(text));
		if (ch) void (scenario ? scenario[1](ch, text) : streamReply(ch, text));
		return null;
	},
	conv_interrupt: async () => null,
	host_list: async () => [],
	paths_check: async (args) =>
		(args.candidates as string[]).map((c) => {
			const m = /^(.*?)(?::(\d+))?(?::(\d+))?$/.exec(c.replace(/^file:\/\//, ""));
			const rel = (m?.[1] ?? c).replace(/^\.\//, "").replace(/\/$/, "");
			if (!(rel in mockFiles) && !mockDirs.has(rel)) return null;
			return { abs: `${MOCK_ROOT}/${rel}`, dir: mockDirs.has(rel), line: m?.[2] ? Number(m[2]) : undefined, col: m?.[3] ? Number(m[3]) : undefined };
		}),
	open_path: async (args) => (String(args.path).endsWith(".app") ? "revealed" : String(args.how)),
	editors_available: async () => ["vscode", "zed"],
	read_text_file: async (args) => {
		const rel = String(args.path).replace(MOCK_ROOT + "/", "");
		const text = mockFiles[rel];
		if (text === undefined) throw new Error("That file does not exist");
		return { path: args.path, text, html: null, lines: text.split("\n").length, size: text.length, truncated: false, binary: false };
	},
	git_info: async (args) => {
		if (!String(args.cwd).startsWith(MOCK_ROOT)) throw new Error("This folder is not in a git repository");
		return { root: MOCK_ROOT, branch: "feature/links", base: "origin/main", upstream: "origin/feature/links", ahead: 1, behind: 0, dirty: mockDirty, hasCommits: true, worktree: false, managed: false, gh: true };
	},
	git_diff: async (args) => (args.source === "uncommitted" ? (mockDirty ? mockDiffs() : []) : mockDiffs().slice(0, 1)),
	git_log: async () => [
		{ sha: "a1b2c3d4", short: "a1b2c3d", subject: "feat: clickable paths", author: "Demo", date: new Date(Date.now() - 3600e3).toISOString() },
		{ sha: "e5f6a7b8", short: "e5f6a7b", subject: "fix: queue drains after a limit", author: "Demo", date: new Date(Date.now() - 86400e3).toISOString() },
	],
	git_commit: async () => {
		mockDirty = 0;
		return "9f8e7d6";
	},
	git_push: async () => "To origin: feature/links",
	git_pr_status: async () => null,
	git_pr_create: async () => "https://example.com/pr/1",
	git_message: async (args) => {
		await sleep(300);
		return args.pr ? "Clickable paths in replies\n\n- Paths open in the editor" : "feat: open paths from replies";
	},
	project_config: async () => ({ actions: [{ name: "Dev", command: "bun run dev" }, { name: "Test", command: "bun test" }] }),
	worktree_add: async (args) => ({ path: `${MOCK_ROOT}-wt/${args.name}`, branch: `glaude/${args.name}`, setup: null }),
	worktree_remove: async () => null,
	conv_attach: async () => {
		throw new Error("not running");
	},
	conv_queue: async (args) => {
		const convId = args.convId as string;
		const items = args.items as QueuedMessage[];
		mockQueues.set(convId, items);
		chans.get(convId)?.push([{ type: "queue", items }]);
		if (!busy.has(convId) && items.length) {
			const [next, ...rest] = items;
			mockQueues.set(convId, rest);
			chans.get(convId)?.push([{ type: "queue", items: rest }]);
			void mockBackend.conv_send({ convId, text: next.text, attachments: next.attachments ?? [] });
		}
		return null;
	},
	conv_schedule: async (args) => {
		const ch = chans.get(args.convId as string);
		const at = (args.resumeAt as string | null) ?? undefined;
		ch?.push([{ type: "schedule", limitedUntil: at ? new Date(Date.now() + 2 * 3600e3).toISOString() : undefined, resumeAt: at }]);
		return null;
	},
	conv_permission: async (args) => {
		const ch = chans.get(args.convId as string);
		const allow = String(args.optionId).startsWith("allow");
		const prompt = pendingPrompts.get(args.requestId as string);
		if (ch && prompt && (prompt.kind === "permission" || prompt.kind === "question")) {
			pendingPrompts.delete(args.requestId as string);
			ch.push([{ type: "item", item: { ...prompt, status: allow ? "done" : "declined" } as Item }]);
			if (prompt.kind === "permission" && prompt.tool === "ExitPlanMode" && allow) {
				const mode = args.optionId === "allow-accept-edits" ? "acceptEdits" : "default";
				ch.push([{ type: "session", info: { permissionMode: mode } }, { type: "turn", state: "writing" }]);
				await say(ch, "Plan approved. Starting with the direction detector.");
			} else if (!allow) {
				await say(ch, args.message ? `Understood: ${String(args.message)}` : "Okay, skipping that.");
			}
			ch.push([{ type: "turnDone", durationMs: 2000 }]);
			return null;
		}
		if (ch) {
			const it = demoItems().find((i) => i.kind === "permission") as Extract<Item, { kind: "permission" }>;
			ch.push([
				{ type: "item", item: { ...it, status: allow ? "done" : "declined" } },
				{ type: "turnDone", durationMs: 41000 },
			]);
		}
		return null;
	},
	conv_answer_question: async (args) => {
		const ch = chans.get(args.convId as string);
		const prompt = pendingPrompts.get(args.requestId as string);
		if (ch && prompt?.kind === "question") {
			pendingPrompts.delete(args.requestId as string);
			const answers = args.answers as Record<string, string>;
			ch.push([{ type: "item", item: { ...prompt, status: "done", answers } }, { type: "turn", state: "writing" }]);
			await sleep(400);
			await say(ch, `Got it: ${Object.values(answers).join(" · ")}.`);
			ch.push([{ type: "turnDone", durationMs: 1500 }]);
		}
		return null;
	},
	conv_control: async (args) => {
		await sleep(250);
		return mockControl(args.convId as string, args.request as Record<string, unknown>);
	},
	conv_hook_runs: async () => [
		{ name: "PreToolUse:Bash", event: "PreToolUse", outcome: "success", exitCode: 0, output: "", at: Date.now() - 40_000 },
		{ name: "PostToolUse:Bash", event: "PostToolUse", outcome: "success", exitCode: 0, output: "", at: Date.now() - 38_000 },
		{ name: "Stop", event: "Stop", outcome: "error", exitCode: 1, output: "notifier not found", at: Date.now() - 5_000 },
	],
	claude_agents: async () => mockAgents,
	claude_rewind_target: async () => "demo-parent-uuid",
	task_output_file: async () => "42 pass\n0 fail\n",
	open_memory_path: async () => null,
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
	search_refresh: async () => {},
	search_status: async () => ({ ready: true, indexing: false, progress: 1, messages: 27465, error: null }),
	search_query: async (args) => {
		const q = String(args.query);
		if (!q.trim()) return [];
		const mark = (t: string) => t.replace(q, `<mark>${q}</mark>`);
		return [
			{ provider: "claude", sessionId: "s1", cwd: "/Users/demo/Projects/glaude", role: "user", ts: Date.now() - 4 * 60e3, snippet: mark(`…چرا جهت متن ${q} در پیام‌ها اشتباه نمایش داده می‌شود؟`), matches: 3 },
			{ provider: "codex", sessionId: "s2", cwd: "/Users/demo/Projects/glaude", role: "assistant", ts: Date.now() - 3 * 3600e3, snippet: mark(`Added terminal tabs; ${q} now survives tab moves…`), matches: 1 },
		];
	},
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
