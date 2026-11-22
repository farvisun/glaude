import { describe, expect, it } from "vitest";
import { groupItems } from "./group";
import type { Item } from "$lib/types";

const tool = (id: string, name: string, parent?: string): Item => ({ kind: "tool", id, name, title: "", input: {}, status: "done", parent });

describe("groupItems", () => {
	it("moves a subagent's steps into its card", () => {
		const items: Item[] = [
			tool("ag", "Agent"),
			tool("s1", "Read", "ag"),
			{ kind: "assistant", id: "a1", blocks: [], parent: "ag", streaming: false },
			tool("t2", "Bash"),
		];
		const { main, steps } = groupItems(items);
		expect(main.map((i) => i.id)).toEqual(["ag", "t2"]);
		expect(steps.ag.map((i) => i.id)).toEqual(["s1", "a1"]);
	});

	it("keeps steps whose agent call is unknown in the main list", () => {
		const { main } = groupItems([tool("s1", "Read", "missing")]);
		expect(main.map((i) => i.id)).toEqual(["s1"]);
	});

	it("hides the tool call behind a question card", () => {
		const items: Item[] = [
			tool("toolu_q", "AskUserQuestion"),
			{ kind: "question", id: "q", requestId: "r", questions: [], status: "pending", toolUseId: "toolu_q" },
			tool("toolu_p", "ExitPlanMode"),
		];
		expect(groupItems(items).main.map((i) => i.id)).toEqual(["q", "toolu_p"]);
	});
});
