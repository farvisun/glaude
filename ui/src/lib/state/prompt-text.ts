// Saved prompt text: fill-in parts like {{file}} and the built-ins
// {{cwd}}, {{project}}, {{branch}} and {{date}}.

export const BUILT_INS = ["cwd", "project", "branch", "date"] as const;

/** Name as a / command: "Review PR" → "review-pr". */
export function slugOf(name: string): string {
	return (
		name
			.trim()
			.toLowerCase()
			.replace(/[^\p{L}\p{N}]+/gu, "-")
			.replace(/^-+|-+$/g, "") || "prompt"
	);
}

/** The parts the user fills in, in order of first use. */
export function variablesOf(body: string): string[] {
	const out: string[] = [];
	for (const m of body.matchAll(/\{\{\s*([\p{L}\p{N}_ -]+?)\s*\}\}/gu)) {
		const v = m[1].trim();
		if (!out.includes(v) && !(BUILT_INS as readonly string[]).includes(v)) out.push(v);
	}
	return out;
}

export function fillPrompt(body: string, values: Record<string, string>, ctx: { cwd?: string; branch?: string; now?: Date } = {}): string {
	const builtin: Record<string, string> = {
		cwd: ctx.cwd ?? "",
		project: (ctx.cwd ?? "").split(/[\\/]/).filter(Boolean).pop() ?? "",
		branch: ctx.branch ?? "",
		date: (ctx.now ?? new Date()).toISOString().slice(0, 10),
	};
	return body.replace(/\{\{\s*([\p{L}\p{N}_ -]+?)\s*\}\}/gu, (all, name: string) => {
		const k = name.trim();
		return k in values ? values[k] : k in builtin ? builtin[k] : all;
	});
}
