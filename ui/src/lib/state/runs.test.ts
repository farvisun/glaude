import { describe, expect, it } from "vitest";
import type { Item } from "$lib/types";
import { groupRuns, runSummary, editedFiles, failures } from "./runs";

const tool = (id: string, name: string, title: string, extra: Partial<Extract<Item, { kind: "tool" }>> = {}): Item => ({
	kind: "tool",
	id,
	name,
	title,
	input: {},
	status: "done",
	...extra,
});
const say = (id: string): Item => ({ kind: "assistant", id, blocks: [], streaming: false });
const think = (id: string): Item => ({ kind: "reasoning", id, blocks: [], streaming: false });
const none = () => false;

describe("groupRuns", () => {
	it("folds consecutive tool calls with thinking between them", () => {
		const items = [say("a1"), tool("t1", "Read", "a.ts"), think("r1"), tool("t2", "Bash", "ls"), think("r2"), say("a2")];
		const rows = groupRuns(items, { compact: true, hasTask: none });
		expect(rows.map((r) => r.kind)).toEqual(["item", "run", "item", "item"]);
		const run = rows[1];
		expect(run.kind === "run" && run.items.map((i) => i.id)).toEqual(["t1", "r1", "t2"]);
		// Thinking after the last call stays outside the run.
		expect(rows[2].id).toBe("r2");
	});
	it("leaves single calls, agents and background calls alone", () => {
		const items = [tool("t1", "Read", "a.ts"), say("a1"), tool("t2", "Agent", "x"), tool("t3", "Bash", "y"), tool("t4", "Bash", "z")];
		const rows = groupRuns(items, { compact: true, hasTask: (id) => id === "t3" });
		expect(rows.every((r) => r.kind === "item")).toBe(true);
	});
	it("does nothing when not compact", () => {
		const items = [tool("t1", "Read", "a"), tool("t2", "Read", "b")];
		expect(groupRuns(items, { compact: false, hasTask: none }).every((r) => r.kind === "item")).toBe(true);
	});
});

describe("runSummary", () => {
	it("counts distinct files read and edited, commands and searches", () => {
		const items = [
			tool("1", "Read", "a.ts"),
			tool("2", "Read", "a.ts"),
			tool("3", "Read", "b.ts"),
			tool("4", "Edit", "a.ts", { diffs: [{ path: "a.ts", kind: "update", diff: "", added: 3, removed: 1 }] }),
			tool("5", "Edit", "a.ts", { diffs: [{ path: "a.ts", kind: "update", diff: "", added: 1, removed: 0 }] }),
			tool("6", "Bash", "bun test"),
			tool("7", "Grep", "foo"),
		];
		expect(runSummary(items)).toBe("Read 2 files · Edited 1 file · Ran 1 command · Searched once");
		expect(editedFiles(items)).toEqual([{ path: "a.ts", added: 4, removed: 1 }]);
	});
	it("counts failures", () => {
		const items: Item[] = [
			tool("1", "Bash", "x", { status: "error" }),
			{ kind: "command", id: "2", command: "false", cwd: "/", status: "done", output: "", exitCode: 1 },
		];
		expect(failures(items)).toBe(2);
	});
});
