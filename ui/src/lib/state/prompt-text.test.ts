import { describe, expect, it } from "vitest";
import { fillPrompt, slugOf, variablesOf } from "./prompt-text";

describe("saved prompts", () => {
	it("finds the parts to fill, without built-ins or repeats", () => {
		expect(variablesOf("Fix {{file}} in {{project}}; see {{ file }} and {{issue id}}")).toEqual(["file", "issue id"]);
	});
	it("fills parts and built-ins, leaves unknown ones", () => {
		const out = fillPrompt("{{a}} on {{branch}} in {{project}} at {{date}} {{b}}", { a: "Test" }, {
			cwd: "/Users/x/glaude",
			branch: "main",
			now: new Date("2026-11-22T10:00:00Z"),
		});
		expect(out).toBe("Test on main in glaude at 2026-11-22 {{b}}");
	});
	it("makes / command names", () => {
		expect(slugOf("Review PR!")).toBe("review-pr");
		expect(slugOf("بررسی کد")).toBe("بررسی-کد");
	});
});
