import { describe, expect, it } from "vitest";
import { snoozeOptions } from "./triage.svelte";

describe("snoozeOptions", () => {
	it("offers this evening only when it is more than an hour away", () => {
		const morning = new Date(2026, 10, 23, 10, 0); // Monday
		const labels = snoozeOptions(morning).map((o) => o.label);
		expect(labels).toEqual(["1 hour", "This evening", "Tomorrow morning", "Next week"]);
		const late = new Date(2026, 10, 23, 18, 30);
		expect(snoozeOptions(late).map((o) => o.label)).not.toContain("This evening");
	});

	it("next week is a Monday morning after tomorrow", () => {
		const fri = new Date(2026, 10, 27, 10, 0);
		const next = new Date(snoozeOptions(fri).find((o) => o.label === "Next week")!.until);
		expect(next.getDay()).toBe(1);
		expect(next.getHours()).toBe(9);
		expect(next.getTime()).toBeGreaterThan(fri.getTime() + 86_400_000);
	});
});
